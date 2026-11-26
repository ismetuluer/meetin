//! `.mee` zengin medya ZIP arşivi okuyucu ve yazıcı modülü (Faz B).
//!
//! Standart ZIP (Stored yöntem 0 + IEEE CRC-32 + Local File Header + Central Directory + EOCD)
//! konteyneri kullanır; sıfır harici ZIP bağımlılığıyla çevrimdışı derlenir ve
//! Windows Gezgini dahil tüm standart ZIP araçlarıyla açılabilir.
//!
//! Arşiv içi yapı:
//! - `manifest.json` (`format: "meetin-archive"`, `schema_version: 2`, `app_version`, `created_at`, `phrase_count`, `asset_count`)
//! - `library.json` (klasörler ve ifadeler; `content_html` içindeki `data:image/...;base64,...` görselleri `assets/<hash>.<ext>` olarak bağlanır)
//! - `assets/<hash>.<ext>` (ikili görsel dosyaları)

use std::collections::{BTreeMap, HashMap};

use chrono::{Datelike, Timelike, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::db::{FolderRecord, PhraseRecord};
use crate::io::formats::ImportedPhraseItem;
use crate::matcher::case::CaseMode;

pub const MEE_ARCHIVE_FORMAT: &str = "meetin-archive";
pub const MEE_SCHEMA_VERSION: u32 = 2;

const LFH_SIGNATURE: u32 = 0x0403_4b50;
const CD_SIGNATURE: u32 = 0x0201_4b50;
const EOCD_SIGNATURE: u32 = 0x0605_4b50;
const ZIP_VERSION_20: u16 = 20;
const ZIP_FLAG_UTF8: u16 = 0x0800;
const ZIP_FLAG_DATA_DESCRIPTOR: u16 = 0x0008;
const ZIP_METHOD_STORED: u16 = 0;
const MAX_ARCHIVE_ENTRIES: usize = 10_000;
const MAX_TOTAL_UNCOMPRESSED_BYTES: usize = 128 * 1024 * 1024;

#[derive(Debug, Error)]
pub enum MeeArchiveError {
    #[error("Geçersiz veya bozuk .mee ZIP arşivi: {0}")]
    InvalidZip(String),
    #[error("Desteklenmeyen ZIP sıkıştırma yöntemi ({0}); yalnızca sıkıştırılmamış (Stored) .mee arşivleri desteklenir.")]
    UnsupportedCompression(u16),
    #[error("Arşiv bütünlük hatası: '{name}' dosyası için CRC32 doğrulaması başarısız oldu (beklenen {expected:#010x}, hesaplanan {actual:#010x}).")]
    CrcMismatch {
        name: String,
        expected: u32,
        actual: u32,
    },
    #[error("Güvenlik hatası: Arşiv içinde geçersiz veya yol geçişi içeren dosya yolu engellendi: '{0}'.")]
    PathTraversal(String),
    #[error("Geçersiz .mee arşivi: '{0}' dosyası eksik.")]
    MissingEntry(String),
    #[error("Geçersiz .mee arşivi: '{entry}' JSON ayrıştırması başarısız oldu: {source}")]
    InvalidJson {
        entry: String,
        #[source]
        source: serde_json::Error,
    },
    #[error("Geçersiz .mee arşivi: beklenmeyen arşiv biçimi ('{0}').")]
    InvalidFormat(String),
    #[error("Desteklenmeyen .mee arşiv şema sürümü: {0} (desteklenen sürüm: {MEE_SCHEMA_VERSION}).")]
    UnsupportedSchemaVersion(u32),
    #[error("Bozuk .mee arşivi: İfade içeriğinde başvurulan '{0}' görsel dosyası arşivde bulunamadı.")]
    MissingAsset(String),
    #[error("Arşiv oluşturma sınırı aşıldı: {0}")]
    ArchiveLimit(String),
    #[error("JSON serileştirme hatası: {0}")]
    Serialize(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MeeManifest {
    pub format: String,
    pub schema_version: u32,
    pub app_version: String,
    pub created_at: String,
    pub phrase_count: usize,
    pub asset_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MeeLibraryDoc {
    format: String,
    version: u32,
    #[serde(default)]
    folders: Vec<String>,
    phrases: Vec<MeeArchivePhrase>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MeeArchivePhrase {
    abbr: String,
    content: String,
    #[serde(default)]
    content_html: Option<String>,
    #[serde(default)]
    description: String,
    #[serde(default)]
    folder: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    case_mode: CaseMode,
}

/// Ham baytların ZIP konteyneri imza baytlarıyla (`PK\x03\x04` veya `PK\x05\x06`) başlayıp başlamadığını döndürür.
pub fn is_zip_signature(raw: &[u8]) -> bool {
    raw.len() >= 4
        && raw[0] == b'P'
        && raw[1] == b'K'
        && ((raw[2] == 0x03 && raw[3] == 0x04) || (raw[2] == 0x05 && raw[3] == 0x06))
}

/// Ham baytların ZIP imzası (`PK...`) ile başlayıp başlamadığını denetler (bozuk/yarım ZIP'leri de yakalar).
pub fn looks_like_zip(raw: &[u8]) -> bool {
    raw.len() >= 2 && raw[0] == b'P' && raw[1] == b'K'
}

// ============================================================================
// IEEE 802.3 CRC-32
// ============================================================================

const fn build_crc32_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    let mut i = 0usize;
    while i < 256 {
        let mut crc = i as u32;
        let mut bit = 0;
        while bit < 8 {
            if (crc & 1) != 0 {
                crc = (crc >> 1) ^ 0xEDB8_8320;
            } else {
                crc >>= 1;
            }
            bit += 1;
        }
        table[i] = crc;
        i += 1;
    }
    table
}

const CRC32_TABLE: [u32; 256] = build_crc32_table();

pub fn crc32_ieee(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        let idx = ((crc ^ (b as u32)) & 0xFF) as usize;
        crc = (crc >> 8) ^ CRC32_TABLE[idx];
    }
    !crc
}

// ============================================================================
// Saf Rust SHA-256 (varlık içerik karma adı için)
// ============================================================================

const SHA256_K: [u32; 64] = [
    0x428a_2f98, 0x7137_4491, 0xb5c0_fbcf, 0xe9b5_dba5, 0x3956_c25b, 0x59f1_11f1, 0x923f_82a4,
    0xab1c_5ed5, 0xd807_aa98, 0x1283_5b01, 0x2431_85be, 0x550c_7dc3, 0x72be_5d74, 0x80de_b1fe,
    0x9bdc_06a7, 0xc19b_f174, 0xe49b_69c1, 0xefbe_4786, 0x0fc1_9dc6, 0x240c_a1cc, 0x2de9_2c6f,
    0x4a74_84aa, 0x5cb0_a9dc, 0x76f9_88da, 0x983e_5152, 0xa831_c66d, 0xb003_27c8, 0xbf59_7fc7,
    0xc6e0_0bf3, 0xd5a7_9147, 0x06ca_6351, 0x1429_2967, 0x27b7_0a85, 0x2e1b_2138, 0x4d2c_6dfc,
    0x5338_0d13, 0x650a_7354, 0x766a_0abb, 0x81c2_c92e, 0x9272_2c85, 0xa2bf_e8a1, 0xa81a_664b,
    0xc24b_8b70, 0xc76c_51a3, 0xd192_e819, 0xd699_0624, 0xf40e_3585, 0x106a_a070, 0x19a4_c116,
    0x1e37_6c08, 0x2748_774c, 0x34b0_bcb5, 0x391c_0cb3, 0x4ed8_aa4a, 0x5b9c_ca4f, 0x682e_6ff3,
    0x748f_82ee, 0x78a5_636f, 0x84c8_7814, 0x8cc7_0208, 0x90be_fffa, 0xa450_6ceb, 0xbef9_a3f7,
    0xc671_78f2,
];

/// Akış tabanlı saf Rust SHA-256 hesaplayıcı (büyük kurulum paketlerini sabit bellek ile doğrular).
pub(crate) struct Sha256Hasher {
    state: [u32; 8],
    buf: [u8; 64],
    buf_len: usize,
    total_len: u64,
}

impl Sha256Hasher {
    pub(crate) fn new() -> Self {
        Self {
            state: [
                0x6a09_e667,
                0xbb67_ae85,
                0x3c6e_f372,
                0xa54f_f53a,
                0x510e_527f,
                0x9b05_688c,
                0x1f83_d9ab,
                0x5be0_cd19,
            ],
            buf: [0u8; 64],
            buf_len: 0,
            total_len: 0,
        }
    }

    pub(crate) fn update(&mut self, mut data: &[u8]) {
        self.total_len = self.total_len.wrapping_add(data.len() as u64);

        if self.buf_len > 0 {
            let take = (64 - self.buf_len).min(data.len());
            self.buf[self.buf_len..self.buf_len + take].copy_from_slice(&data[..take]);
            self.buf_len += take;
            data = &data[take..];
            if self.buf_len == 64 {
                let block = self.buf;
                Self::compress(&mut self.state, &block);
                self.buf_len = 0;
            }
        }

        let mut chunks = data.chunks_exact(64);
        for chunk in &mut chunks {
            let mut block = [0u8; 64];
            block.copy_from_slice(chunk);
            Self::compress(&mut self.state, &block);
        }

        let rem = chunks.remainder();
        if !rem.is_empty() {
            self.buf[..rem.len()].copy_from_slice(rem);
            self.buf_len = rem.len();
        }
    }

    pub(crate) fn finalize(mut self) -> [u8; 32] {
        let bit_len = self.total_len.wrapping_mul(8);
        self.buf[self.buf_len] = 0x80;
        self.buf_len += 1;

        if self.buf_len > 56 {
            for b in &mut self.buf[self.buf_len..64] {
                *b = 0;
            }
            let block = self.buf;
            Self::compress(&mut self.state, &block);
            self.buf = [0u8; 64];
            self.buf_len = 0;
        } else {
            for b in &mut self.buf[self.buf_len..56] {
                *b = 0;
            }
        }

        self.buf[56..64].copy_from_slice(&bit_len.to_be_bytes());
        let final_block = self.buf;
        Self::compress(&mut self.state, &final_block);

        let mut out = [0u8; 32];
        for (i, word) in self.state.iter().enumerate() {
            out[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
        }
        out
    }

    fn compress(h: &mut [u32; 8], chunk: &[u8; 64]) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }

        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = *h;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(SHA256_K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }
}

pub(crate) fn sha256_digest(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256Hasher::new();
    hasher.update(data);
    hasher.finalize()
}

fn short_content_hash(data: &[u8]) -> String {
    let digest = sha256_digest(data);
    let mut hex = String::with_capacity(32);
    for &b in &digest[..16] {
        use std::fmt::Write as _;
        let _ = write!(&mut hex, "{b:02x}");
    }
    hex
}

// ============================================================================
// Saf Rust Base64 (RFC 4648)
// ============================================================================

const B64_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub(crate) fn base64_encode(input: &[u8]) -> String {
    if input.is_empty() {
        return String::new();
    }
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    let mut i = 0;
    while i < input.len() {
        let b0 = input[i];
        let b1 = if i + 1 < input.len() { input[i + 1] } else { 0 };
        let b2 = if i + 2 < input.len() { input[i + 2] } else { 0 };

        out.push(B64_ALPHABET[(b0 >> 2) as usize] as char);
        out.push(B64_ALPHABET[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);
        if i + 1 < input.len() {
            out.push(B64_ALPHABET[(((b1 & 0x0f) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            out.push('=');
        }
        if i + 2 < input.len() {
            out.push(B64_ALPHABET[(b2 & 0x3f) as usize] as char);
        } else {
            out.push('=');
        }
        i += 3;
    }
    out
}

fn base64_val(b: u8) -> Option<u8> {
    match b {
        b'A'..=b'Z' => Some(b - b'A'),
        b'a'..=b'z' => Some(b - b'a' + 26),
        b'0'..=b'9' => Some(b - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

pub(crate) fn base64_decode(input: &str) -> Option<Vec<u8>> {
    let mut cleaned: Vec<u8> = Vec::with_capacity(input.len());
    for &b in input.as_bytes() {
        if b.is_ascii_whitespace() {
            continue;
        }
        cleaned.push(b);
    }
    if cleaned.is_empty() {
        return Some(Vec::new());
    }
    if cleaned.len() % 4 == 1 {
        return None;
    }

    let mut out = Vec::with_capacity((cleaned.len() / 4) * 3 + 2);
    let mut i = 0;
    while i < cleaned.len() {
        let rem = cleaned.len() - i;
        let c0 = cleaned[i];
        let c1 = cleaned[i + 1];
        let c2 = if rem > 2 { cleaned[i + 2] } else { b'=' };
        let c3 = if rem > 3 { cleaned[i + 3] } else { b'=' };

        let v0 = base64_val(c0)?;
        let v1 = base64_val(c1)?;
        out.push((v0 << 2) | (v1 >> 4));

        if c2 == b'=' {
            if c3 != b'=' || i + 4 < cleaned.len() {
                return None;
            }
            break;
        }
        let v2 = base64_val(c2)?;
        out.push((v1 << 4) | (v2 >> 2));

        if c3 == b'=' {
            if i + 4 < cleaned.len() {
                return None;
            }
            break;
        }
        let v3 = base64_val(c3)?;
        out.push((v2 << 6) | v3);

        i += 4;
    }
    Some(out)
}

// ============================================================================
// Yol Güvenliği ve MIME Yardımcıları
// ============================================================================

fn validate_archive_path(path: &str, allow_trailing_slash: bool) -> Result<(), MeeArchiveError> {
    if path.is_empty()
        || path.contains('\0')
        || path.contains('\\')
        || path.contains(':')
        || path.starts_with('/')
    {
        return Err(MeeArchiveError::PathTraversal(path.to_string()));
    }
    let check_str = if allow_trailing_slash {
        path.strip_suffix('/').unwrap_or(path)
    } else {
        if path.ends_with('/') {
            return Err(MeeArchiveError::PathTraversal(path.to_string()));
        }
        path
    };
    if check_str.is_empty() {
        return Err(MeeArchiveError::PathTraversal(path.to_string()));
    }
    for segment in check_str.split('/') {
        if segment.is_empty() || segment == "." || segment == ".." {
            return Err(MeeArchiveError::PathTraversal(path.to_string()));
        }
    }
    Ok(())
}

fn image_subtype_to_ext(subtype: &str) -> Option<&'static str> {
    if subtype.eq_ignore_ascii_case("png") {
        Some("png")
    } else if subtype.eq_ignore_ascii_case("jpeg") || subtype.eq_ignore_ascii_case("jpg") {
        Some("jpg")
    } else if subtype.eq_ignore_ascii_case("gif") {
        Some("gif")
    } else if subtype.eq_ignore_ascii_case("webp") {
        Some("webp")
    } else if subtype.eq_ignore_ascii_case("svg+xml") {
        Some("svg")
    } else if subtype.eq_ignore_ascii_case("bmp") {
        Some("bmp")
    } else if subtype.eq_ignore_ascii_case("x-icon")
        || subtype.eq_ignore_ascii_case("vnd.microsoft.icon")
    {
        Some("ico")
    } else {
        None
    }
}

fn ext_to_image_mime(path: &str) -> &'static str {
    let ext = path.rsplit('.').next().unwrap_or("");
    if ext.eq_ignore_ascii_case("png") {
        "image/png"
    } else if ext.eq_ignore_ascii_case("jpg") || ext.eq_ignore_ascii_case("jpeg") {
        "image/jpeg"
    } else if ext.eq_ignore_ascii_case("gif") {
        "image/gif"
    } else if ext.eq_ignore_ascii_case("webp") {
        "image/webp"
    } else if ext.eq_ignore_ascii_case("svg") {
        "image/svg+xml"
    } else if ext.eq_ignore_ascii_case("bmp") {
        "image/bmp"
    } else if ext.eq_ignore_ascii_case("ico") {
        "image/x-icon"
    } else {
        "application/octet-stream"
    }
}

fn starts_with_ignore_ascii_case(haystack: &str, needle: &str) -> bool {
    if haystack.len() < needle.len() {
        return false;
    }
    haystack.as_bytes()[..needle.len()].eq_ignore_ascii_case(needle.as_bytes())
}

fn find_ignore_ascii_case(haystack: &str, needle: &str, from: usize) -> Option<usize> {
    let h = haystack.as_bytes();
    let n = needle.as_bytes();
    if n.is_empty() || from + n.len() > h.len() {
        return None;
    }
    for i in from..=h.len() - n.len() {
        if h[i..i + n.len()].eq_ignore_ascii_case(n) {
            return Some(i);
        }
    }
    None
}

fn is_uri_terminator(b: u8) -> bool {
    matches!(b, b'"' | b'\'' | b')' | b'<' | b'>' | b' ' | b'\t' | b'\r' | b'\n')
}

/// HTML içindeki `data:image/<tur>;base64,<veri>` URI'lerini ayıklayıp `assets/<hash>.<ext>` yoluna bağlar.
fn extract_data_uri_images(html: &str, assets: &mut BTreeMap<String, Vec<u8>>) -> String {
    let mut out = String::with_capacity(html.len());
    let mut cursor = 0usize;

    while let Some(pos) = find_ignore_ascii_case(html, "data:image/", cursor) {
        out.push_str(&html[cursor..pos]);

        let bytes = html.as_bytes();
        let mut end = pos;
        while end < bytes.len() && !is_uri_terminator(bytes[end]) {
            end += 1;
        }
        let uri = &html[pos..end];

        if let Some(semi_b64) = find_ignore_ascii_case(uri, ";base64,", "data:image/".len()) {
            let subtype = &uri["data:image/".len()..semi_b64];
            let payload = &uri[semi_b64 + ";base64,".len()..];
            if let (Some(ext), Some(decoded)) =
                (image_subtype_to_ext(subtype), base64_decode(payload))
            {
                let hash = short_content_hash(&decoded);
                let asset_path = format!("assets/{hash}.{ext}");
                assets.entry(asset_path.clone()).or_insert(decoded);
                out.push_str(&asset_path);
                cursor = end;
                continue;
            }
        }

        // Geçerli bir base64 görsel URI'si değilse dokunmadan devam et.
        out.push_str(&html[pos..end]);
        cursor = end;
    }

    out.push_str(&html[cursor..]);
    out
}

/// HTML içindeki `assets/<dosya>` referanslarını arşivden okuyup `data:image/...;base64,...` olarak geri yükler.
fn restore_asset_images(
    html: &str,
    entries: &BTreeMap<String, Vec<u8>>,
) -> Result<String, MeeArchiveError> {
    let mut out = String::with_capacity(html.len());
    let mut cursor = 0usize;
    let bytes = html.as_bytes();

    while cursor < bytes.len() {
        let Some(pos) = find_ignore_ascii_case(html, "assets/", cursor) else {
            out.push_str(&html[cursor..]);
            break;
        };

        // Yalnızca HTML öznitelik değeri (src="assets/...", src='assets/...') veya url(assets/...) başlangıçlarını dönüştür.
        let prev_byte = if pos > 0 { Some(bytes[pos - 1]) } else { None };
        let is_attr_start = matches!(prev_byte, Some(b'"' | b'\'' | b'(' | b'='));
        if !is_attr_start {
            out.push_str(&html[cursor..pos + "assets/".len()]);
            cursor = pos + "assets/".len();
            continue;
        }

        out.push_str(&html[cursor..pos]);

        let mut end = pos;
        while end < bytes.len() && !is_uri_terminator(bytes[end]) {
            end += 1;
        }
        let asset_ref = &html[pos..end];
        validate_archive_path(asset_ref, false)?;

        let data = entries
            .get(asset_ref)
            .ok_or_else(|| MeeArchiveError::MissingAsset(asset_ref.to_string()))?;
        let mime = ext_to_image_mime(asset_ref);
        let encoded = base64_encode(data);
        out.push_str("data:");
        out.push_str(mime);
        out.push_str(";base64,");
        out.push_str(&encoded);

        cursor = end;
    }

    Ok(out)
}

// ============================================================================
// Saf Rust Standart ZIP Yazıcı ve Okuyucu (Stored yöntem 0)
// ============================================================================

fn dos_datetime_utc() -> (u16, u16) {
    let now = Utc::now();
    let year = now.year().clamp(1980, 2107) as u16;
    let month = now.month() as u16;
    let day = now.day() as u16;
    let hour = now.hour() as u16;
    let min = now.minute() as u16;
    let sec = (now.second() / 2) as u16;
    let dos_time = (hour << 11) | (min << 5) | sec;
    let dos_date = ((year - 1980) << 9) | (month << 5) | day;
    (dos_time, dos_date)
}

struct CentralDirRecord<'a> {
    name: &'a [u8],
    crc32: u32,
    size: u32,
    local_offset: u32,
}

fn write_zip_stored(entries: &[(String, Vec<u8>)]) -> Result<Vec<u8>, MeeArchiveError> {
    if entries.len() > u16::MAX as usize {
        return Err(MeeArchiveError::ArchiveLimit(
            "Arşiv en fazla 65535 dosya içerebilir.".to_string(),
        ));
    }

    let (dos_time, dos_date) = dos_datetime_utc();
    let mut out = Vec::new();
    let mut cd_records: Vec<CentralDirRecord<'_>> = Vec::with_capacity(entries.len());

    for (name, data) in entries {
        validate_archive_path(name, false)?;
        let name_bytes = name.as_bytes();
        if name_bytes.len() > u16::MAX as usize {
            return Err(MeeArchiveError::ArchiveLimit(format!(
                "Dosya adı çok uzun: '{name}'."
            )));
        }
        let size = u32::try_from(data.len()).map_err(|_| {
            MeeArchiveError::ArchiveLimit(format!("'{name}' dosyası ZIP32 boyut sınırını aşıyor."))
        })?;
        let local_offset = u32::try_from(out.len()).map_err(|_| {
            MeeArchiveError::ArchiveLimit("Arşiv boyutu ZIP32 sınırını aşıyor.".to_string())
        })?;
        let crc = crc32_ieee(data);

        // Local File Header (30 bayt + dosya adı)
        out.extend_from_slice(&LFH_SIGNATURE.to_le_bytes());
        out.extend_from_slice(&ZIP_VERSION_20.to_le_bytes());
        out.extend_from_slice(&ZIP_FLAG_UTF8.to_le_bytes());
        out.extend_from_slice(&ZIP_METHOD_STORED.to_le_bytes());
        out.extend_from_slice(&dos_time.to_le_bytes());
        out.extend_from_slice(&dos_date.to_le_bytes());
        out.extend_from_slice(&crc.to_le_bytes());
        out.extend_from_slice(&size.to_le_bytes());
        out.extend_from_slice(&size.to_le_bytes());
        out.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // extra field length
        out.extend_from_slice(name_bytes);
        out.extend_from_slice(data);

        cd_records.push(CentralDirRecord {
            name: name_bytes,
            crc32: crc,
            size,
            local_offset,
        });
    }

    let cd_offset = u32::try_from(out.len()).map_err(|_| {
        MeeArchiveError::ArchiveLimit("Merkezi dizin konumu ZIP32 sınırını aşıyor.".to_string())
    })?;

    for rec in &cd_records {
        // Central Directory File Header (46 bayt + dosya adı)
        out.extend_from_slice(&CD_SIGNATURE.to_le_bytes());
        out.extend_from_slice(&ZIP_VERSION_20.to_le_bytes()); // version made by
        out.extend_from_slice(&ZIP_VERSION_20.to_le_bytes()); // version needed
        out.extend_from_slice(&ZIP_FLAG_UTF8.to_le_bytes());
        out.extend_from_slice(&ZIP_METHOD_STORED.to_le_bytes());
        out.extend_from_slice(&dos_time.to_le_bytes());
        out.extend_from_slice(&dos_date.to_le_bytes());
        out.extend_from_slice(&rec.crc32.to_le_bytes());
        out.extend_from_slice(&rec.size.to_le_bytes());
        out.extend_from_slice(&rec.size.to_le_bytes());
        out.extend_from_slice(&(rec.name.len() as u16).to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // extra len
        out.extend_from_slice(&0u16.to_le_bytes()); // comment len
        out.extend_from_slice(&0u16.to_le_bytes()); // disk number start
        out.extend_from_slice(&0u16.to_le_bytes()); // internal file attrs
        out.extend_from_slice(&0u32.to_le_bytes()); // external file attrs
        out.extend_from_slice(&rec.local_offset.to_le_bytes());
        out.extend_from_slice(rec.name);
    }

    let cd_size = u32::try_from(out.len() - cd_offset as usize).map_err(|_| {
        MeeArchiveError::ArchiveLimit("Merkezi dizin boyutu ZIP32 sınırını aşıyor.".to_string())
    })?;
    let entry_count = cd_records.len() as u16;

    // End of Central Directory Record (22 bayt)
    out.extend_from_slice(&EOCD_SIGNATURE.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes()); // disk number
    out.extend_from_slice(&0u16.to_le_bytes()); // disk with CD
    out.extend_from_slice(&entry_count.to_le_bytes());
    out.extend_from_slice(&entry_count.to_le_bytes());
    out.extend_from_slice(&cd_size.to_le_bytes());
    out.extend_from_slice(&cd_offset.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes()); // comment len

    Ok(out)
}

fn read_u16_le(data: &[u8], offset: usize) -> Result<u16, MeeArchiveError> {
    let slice = data.get(offset..offset + 2).ok_or_else(|| {
        MeeArchiveError::InvalidZip("Beklenmeyen dosya sonu (16-bit alan okunamadı).".to_string())
    })?;
    Ok(u16::from_le_bytes([slice[0], slice[1]]))
}

fn read_u32_le(data: &[u8], offset: usize) -> Result<u32, MeeArchiveError> {
    let slice = data.get(offset..offset + 4).ok_or_else(|| {
        MeeArchiveError::InvalidZip("Beklenmeyen dosya sonu (32-bit alan okunamadı).".to_string())
    })?;
    Ok(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

fn find_eocd(raw: &[u8]) -> Result<usize, MeeArchiveError> {
    if raw.len() < 22 {
        return Err(MeeArchiveError::InvalidZip(
            "Dosya geçerli bir ZIP konteyneri olmak için çok kısa.".to_string(),
        ));
    }
    let min_pos = raw.len().saturating_sub(22 + 65_535);
    let mut pos = raw.len() - 22;
    loop {
        if read_u32_le(raw, pos)? == EOCD_SIGNATURE {
            let comment_len = read_u16_le(raw, pos + 20)? as usize;
            if pos + 22 + comment_len == raw.len() {
                return Ok(pos);
            }
        }
        if pos == min_pos {
            break;
        }
        pos -= 1;
    }
    Err(MeeArchiveError::InvalidZip(
        "ZIP sonu (EOCD) kaydı bulunamadı; dosya bozuk veya eksik olabilir.".to_string(),
    ))
}

fn read_zip_entries(raw: &[u8]) -> Result<BTreeMap<String, Vec<u8>>, MeeArchiveError> {
    if !is_zip_signature(raw) {
        return Err(MeeArchiveError::InvalidZip(
            "ZIP imza baytları (PK\\x03\\x04) bulunamadı.".to_string(),
        ));
    }

    let eocd_pos = find_eocd(raw)?;
    let disk_num = read_u16_le(raw, eocd_pos + 4)?;
    let cd_disk = read_u16_le(raw, eocd_pos + 6)?;
    if disk_num != 0 || cd_disk != 0 {
        return Err(MeeArchiveError::InvalidZip(
            "Çok diskli (bölünmüş) ZIP arşivleri desteklenmiyor.".to_string(),
        ));
    }
    let entries_disk = read_u16_le(raw, eocd_pos + 8)? as usize;
    let total_entries = read_u16_le(raw, eocd_pos + 10)? as usize;
    if entries_disk != total_entries {
        return Err(MeeArchiveError::InvalidZip(
            "ZIP dizin kayıt sayısı tutarsız.".to_string(),
        ));
    }
    if total_entries > MAX_ARCHIVE_ENTRIES {
        return Err(MeeArchiveError::InvalidZip(format!(
            "Arşiv çok fazla dosya içeriyor ({total_entries})."
        )));
    }

    let cd_size = read_u32_le(raw, eocd_pos + 12)? as usize;
    let cd_offset = read_u32_le(raw, eocd_pos + 16)? as usize;
    let cd_end = cd_offset.checked_add(cd_size).ok_or_else(|| {
        MeeArchiveError::InvalidZip("Merkezi dizin boyut taşması tespit edildi.".to_string())
    })?;
    if cd_end > eocd_pos {
        return Err(MeeArchiveError::InvalidZip(
            "Merkezi dizin aralığı dosya sınırlarını aşıyor.".to_string(),
        ));
    }

    let mut entries = BTreeMap::new();
    let mut cursor = cd_offset;
    let mut total_uncompressed = 0usize;

    for _ in 0..total_entries {
        if cursor + 46 > cd_end {
            return Err(MeeArchiveError::InvalidZip(
                "Merkezi dizin kaydı eksik veya kesilmiş.".to_string(),
            ));
        }
        if read_u32_le(raw, cursor)? != CD_SIGNATURE {
            return Err(MeeArchiveError::InvalidZip(
                "Merkezi dizin başlık imzası geçersiz.".to_string(),
            ));
        }
        let cd_method = read_u16_le(raw, cursor + 10)?;
        let cd_crc = read_u32_le(raw, cursor + 16)?;
        let cd_comp_size = read_u32_le(raw, cursor + 20)? as usize;
        let cd_uncomp_size = read_u32_le(raw, cursor + 24)? as usize;
        let name_len = read_u16_le(raw, cursor + 28)? as usize;
        let extra_len = read_u16_le(raw, cursor + 30)? as usize;
        let comment_len = read_u16_le(raw, cursor + 32)? as usize;
        let local_offset = read_u32_le(raw, cursor + 42)? as usize;

        let name_start = cursor + 46;
        let name_end = name_start + name_len;
        let next_cursor = name_end + extra_len + comment_len;
        if next_cursor > cd_end {
            return Err(MeeArchiveError::InvalidZip(
                "Merkezi dizin dosya adı/ek alanı sınır dışı.".to_string(),
            ));
        }

        let name = std::str::from_utf8(&raw[name_start..name_end])
            .map_err(|_| {
                MeeArchiveError::InvalidZip(
                    "Arşiv içindeki dosya adı geçerli UTF-8 değil.".to_string(),
                )
            })?
            .to_string();

        validate_archive_path(&name, true)?;
        cursor = next_cursor;

        // Klasör girdilerini (örn. "assets/") atla; yol güvenliği zaten doğrulandı.
        if name.ends_with('/') {
            continue;
        }

        if cd_method != ZIP_METHOD_STORED {
            return Err(MeeArchiveError::UnsupportedCompression(cd_method));
        }
        if cd_comp_size != cd_uncomp_size {
            return Err(MeeArchiveError::InvalidZip(format!(
                "'{name}' dosyasında sıkıştırılmış ve açık boyut eşleşmiyor."
            )));
        }

        total_uncompressed = total_uncompressed
            .checked_add(cd_uncomp_size)
            .ok_or_else(|| {
                MeeArchiveError::InvalidZip("Arşiv toplam boyut taşması.".to_string())
            })?;
        if total_uncompressed > MAX_TOTAL_UNCOMPRESSED_BYTES {
            return Err(MeeArchiveError::InvalidZip(
                "Arşivin toplam boyutu güvenlik sınırını (128 MB) aşıyor.".to_string(),
            ));
        }

        if local_offset + 30 > cd_offset {
            return Err(MeeArchiveError::InvalidZip(format!(
                "'{name}' için yerel dosya başlığı konumu geçersiz."
            )));
        }
        if read_u32_le(raw, local_offset)? != LFH_SIGNATURE {
            return Err(MeeArchiveError::InvalidZip(format!(
                "'{name}' için yerel dosya başlığı imzası bozuk."
            )));
        }

        let lfh_flags = read_u16_le(raw, local_offset + 6)?;
        let lfh_method = read_u16_le(raw, local_offset + 8)?;
        if lfh_method != ZIP_METHOD_STORED {
            return Err(MeeArchiveError::UnsupportedCompression(lfh_method));
        }
        let lfh_crc = read_u32_le(raw, local_offset + 14)?;
        let lfh_comp_size = read_u32_le(raw, local_offset + 18)? as usize;
        let lfh_uncomp_size = read_u32_le(raw, local_offset + 22)? as usize;
        let lfh_name_len = read_u16_le(raw, local_offset + 26)? as usize;
        let lfh_extra_len = read_u16_le(raw, local_offset + 28)? as usize;

        if (lfh_flags & ZIP_FLAG_DATA_DESCRIPTOR) == 0
            && (lfh_crc != cd_crc
                || lfh_comp_size != cd_comp_size
                || lfh_uncomp_size != cd_uncomp_size)
        {
            return Err(MeeArchiveError::InvalidZip(format!(
                "'{name}' dosyasının yerel ve merkezi dizin üstbilgileri uyuşmuyor."
            )));
        }

        let data_start = local_offset
            .checked_add(30)
            .and_then(|v| v.checked_add(lfh_name_len))
            .and_then(|v| v.checked_add(lfh_extra_len))
            .ok_or_else(|| {
                MeeArchiveError::InvalidZip(format!("'{name}' veri başlangıç taşması."))
            })?;
        let data_end = data_start.checked_add(cd_comp_size).ok_or_else(|| {
            MeeArchiveError::InvalidZip(format!("'{name}' veri boyutu taşması."))
        })?;
        if data_end > cd_offset {
            return Err(MeeArchiveError::InvalidZip(format!(
                "'{name}' dosya verisi kesilmiş veya sınır dışı."
            )));
        }

        let file_bytes = &raw[data_start..data_end];
        let actual_crc = crc32_ieee(file_bytes);
        if actual_crc != cd_crc {
            return Err(MeeArchiveError::CrcMismatch {
                name,
                expected: cd_crc,
                actual: actual_crc,
            });
        }

        if entries.insert(name.clone(), file_bytes.to_vec()).is_some() {
            return Err(MeeArchiveError::InvalidZip(format!(
                "Arşivde yinelenen dosya yolu tespit edildi: '{name}'."
            )));
        }
    }

    Ok(entries)
}

// ============================================================================
// Genel API: Dışa Aktarma ve İçe Aktarma
// ============================================================================

/// İfade ve klasörleri zengin medya destekli standart `.mee` ZIP arşivine paketler.
pub fn export_mee_archive(
    phrases: &[PhraseRecord],
    folders: &[FolderRecord],
    app_version: &str,
) -> Result<Vec<u8>, MeeArchiveError> {
    let folder_names: HashMap<i64, String> =
        folders.iter().map(|f| (f.id, f.name.clone())).collect();
    let mut assets: BTreeMap<String, Vec<u8>> = BTreeMap::new();

    let exported_phrases: Vec<MeeArchivePhrase> = phrases
        .iter()
        .filter(|p| p.deleted_at.is_none())
        .map(|p| {
            let content_html = p
                .content_html
                .as_deref()
                .map(|html| extract_data_uri_images(html, &mut assets));
            MeeArchivePhrase {
                abbr: p.abbr.clone(),
                content: p.content.clone(),
                content_html,
                description: p.description.clone(),
                folder: p.folder_id.and_then(|fid| folder_names.get(&fid).cloned()),
                tags: p.tags.clone(),
                case_mode: p.case_mode,
            }
        })
        .collect();

    let manifest = MeeManifest {
        format: MEE_ARCHIVE_FORMAT.to_string(),
        schema_version: MEE_SCHEMA_VERSION,
        app_version: app_version.to_string(),
        created_at: Utc::now().to_rfc3339(),
        phrase_count: exported_phrases.len(),
        asset_count: assets.len(),
    };

    let library = MeeLibraryDoc {
        format: "meetin".to_string(),
        version: MEE_SCHEMA_VERSION,
        folders: folders.iter().map(|f| f.name.clone()).collect(),
        phrases: exported_phrases,
    };

    let manifest_bytes = serde_json::to_vec_pretty(&manifest)?;
    let library_bytes = serde_json::to_vec_pretty(&library)?;

    let mut zip_entries: Vec<(String, Vec<u8>)> = Vec::with_capacity(2 + assets.len());
    zip_entries.push(("manifest.json".to_string(), manifest_bytes));
    zip_entries.push(("library.json".to_string(), library_bytes));
    for (path, bytes) in assets {
        zip_entries.push((path, bytes));
    }

    write_zip_stored(&zip_entries)
}

/// Standart `.mee` ZIP arşivini doğrular, `assets/` görsellerini `data:` URI olarak geri yükler
/// ve içe aktarılabilir ifade listesini döndürür.
pub fn import_mee_archive(raw: &[u8]) -> Result<Vec<ImportedPhraseItem>, MeeArchiveError> {
    let entries = read_zip_entries(raw)?;

    let manifest_bytes = entries
        .get("manifest.json")
        .ok_or_else(|| MeeArchiveError::MissingEntry("manifest.json".to_string()))?;
    let manifest: MeeManifest =
        serde_json::from_slice(manifest_bytes).map_err(|source| MeeArchiveError::InvalidJson {
            entry: "manifest.json".to_string(),
            source,
        })?;

    if manifest.format != MEE_ARCHIVE_FORMAT {
        return Err(MeeArchiveError::InvalidFormat(manifest.format));
    }
    if manifest.schema_version != MEE_SCHEMA_VERSION {
        return Err(MeeArchiveError::UnsupportedSchemaVersion(
            manifest.schema_version,
        ));
    }

    let library_bytes = entries
        .get("library.json")
        .ok_or_else(|| MeeArchiveError::MissingEntry("library.json".to_string()))?;
    let library: MeeLibraryDoc =
        serde_json::from_slice(library_bytes).map_err(|source| MeeArchiveError::InvalidJson {
            entry: "library.json".to_string(),
            source,
        })?;

    if !starts_with_ignore_ascii_case(&library.format, "meetin") {
        return Err(MeeArchiveError::InvalidFormat(library.format));
    }

    let mut out = Vec::with_capacity(library.phrases.len());
    for phrase in library.phrases {
        if phrase.abbr.trim().is_empty() || phrase.content.trim().is_empty() {
            continue;
        }
        let restored_html = match phrase.content_html {
            Some(html) => Some(restore_asset_images(&html, &entries)?),
            None => None,
        };
        out.push(ImportedPhraseItem {
            abbr: phrase.abbr.trim().to_string(),
            content: phrase.content,
            content_html: restored_html,
            description: phrase.description,
            folder: phrase.folder.filter(|f| !f.trim().is_empty()),
            tags: phrase.tags,
            case_mode: phrase.case_mode,
        });
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::formats::{export_meetin_json, parse_import_file};

    const SAMPLE_PNG_1X1_B64: &str =
        "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=";

    fn sample_folder() -> FolderRecord {
        FolderRecord {
            id: 10,
            parent_id: None,
            name: "Şablonlar".to_string(),
            color: Some("c2".to_string()),
            sort_order: 0,
        }
    }

    fn sample_rich_phrase(html: Option<String>) -> PhraseRecord {
        PhraseRecord {
            id: 42,
            folder_id: Some(10),
            folder_ids: vec![10],
            abbr: "imza".to_string(),
            content: "Saygılarımla,\nEkip".to_string(),
            content_html: html,
            description: "Logolu imza".to_string(),
            case_mode: CaseMode::KeepOriginal,
            shortcut: None,
            enabled: true,
            use_count: 5,
            last_used_at: None,
            created_at: String::new(),
            updated_at: String::new(),
            deleted_at: None,
            tags: vec!["imza".to_string(), "görsel".to_string()],
        }
    }

    #[test]
    fn crc32_matches_standard_ieee_vector() {
        assert_eq!(crc32_ieee(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn base64_round_trip_works() {
        let raw = b"Meetin zengin medya arsivi \x00\xff\xfe!";
        let enc = base64_encode(raw);
        let dec = base64_decode(&enc).expect("base64 çözülmeli");
        assert_eq!(dec, raw);
    }

    #[test]
    fn rich_phrase_with_embedded_images_round_trips_losslessly() {
        let data_uri = format!("data:image/png;base64,{SAMPLE_PNG_1X1_B64}");
        let svg_b64 = base64_encode(b"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"2\" height=\"2\"/>");
        let svg_uri = format!("data:image/svg+xml;base64,{svg_b64}");
        let original_html = format!(
            "<b>Saygılarımla</b><br><img src=\"{data_uri}\" alt=\"logo\"><img src=\"{svg_uri}\" alt=\"rozet\">"
        );

        let folders = vec![sample_folder()];
        let phrases = vec![sample_rich_phrase(Some(original_html.clone()))];

        let archive_bytes = export_mee_archive(&phrases, &folders, "1.2.0").expect("ZIP üretilmeli");
        assert!(is_zip_signature(&archive_bytes));

        // Arşiv yapısını doğrula: manifest.json, library.json ve 2 adet assets/* dosyası olmalı
        let entries = read_zip_entries(&archive_bytes).expect("ZIP okunmalı");
        assert!(entries.contains_key("manifest.json"));
        assert!(entries.contains_key("library.json"));

        let manifest: MeeManifest =
            serde_json::from_slice(entries.get("manifest.json").unwrap()).unwrap();
        assert_eq!(manifest.format, MEE_ARCHIVE_FORMAT);
        assert_eq!(manifest.schema_version, MEE_SCHEMA_VERSION);
        assert_eq!(manifest.app_version, "1.2.0");
        assert_eq!(manifest.phrase_count, 1);
        assert_eq!(manifest.asset_count, 2);

        let lib_str = std::str::from_utf8(entries.get("library.json").unwrap()).unwrap();
        assert!(!lib_str.contains("data:image/"));
        assert!(lib_str.contains("assets/"));

        // parse_import_file üzerinden kayıpsız gidiş-dönüş eşitliği
        let imported = parse_import_file("yedek.mee", &archive_bytes).expect("içe aktarılmalı");
        assert_eq!(imported.len(), 1);
        assert_eq!(imported[0].abbr, "imza");
        assert_eq!(imported[0].content, "Saygılarımla,\nEkip");
        assert_eq!(imported[0].description, "Logolu imza");
        assert_eq!(imported[0].folder.as_deref(), Some("Şablonlar"));
        assert_eq!(imported[0].tags, vec!["imza".to_string(), "görsel".to_string()]);
        assert_eq!(imported[0].case_mode, CaseMode::KeepOriginal);
        assert_eq!(
            imported[0].content_html.as_deref(),
            Some(original_html.as_str())
        );
    }

    #[test]
    fn plain_json_legacy_mee_and_meetin_remain_readable() {
        let folders = vec![sample_folder()];
        let phrases = vec![sample_rich_phrase(Some("<b>Merhaba</b>".to_string()))];
        let legacy_json = export_meetin_json(&phrases, &folders).unwrap();

        let from_mee = parse_import_file("eski.mee", legacy_json.as_bytes()).unwrap();
        assert_eq!(from_mee.len(), 1);
        assert_eq!(from_mee[0].abbr, "imza");
        assert_eq!(from_mee[0].content_html.as_deref(), Some("<b>Merhaba</b>"));

        let from_meetin = parse_import_file("eski.meetin", legacy_json.as_bytes()).unwrap();
        assert_eq!(from_meetin.len(), 1);
        assert_eq!(from_meetin[0].folder.as_deref(), Some("Şablonlar"));
    }

    #[test]
    fn corrupt_zip_returns_clear_turkish_error() {
        let err = import_mee_archive(b"PK\x03\x04bozuk-icerik").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("bozuk") || msg.contains("ZIP"), "mesaj: {msg}");

        let err2 = parse_import_file("bozuk.mee", b"PK\x03\x04yarim-dosya").unwrap_err();
        let msg2 = err2.to_string();
        assert!(msg2.contains("ZIP") || msg2.contains("bozuk"), "mesaj: {msg2}");
    }

    #[test]
    fn corrupted_crc32_is_rejected_with_turkish_error() {
        let folders = vec![sample_folder()];
        let phrases = vec![sample_rich_phrase(Some("<i>Test</i>".to_string()))];
        let mut archive = export_mee_archive(&phrases, &folders, "1.2.0").unwrap();

        // İlk dosyanın (manifest.json) veri baytlarından birini boz
        let manifest_marker = b"meetin-archive";
        let pos = archive
            .windows(manifest_marker.len())
            .position(|w| w == manifest_marker)
            .expect("manifest içeriği bulunmalı");
        archive[pos] ^= 0xFF;

        let err = import_mee_archive(&archive).unwrap_err();
        let msg = err.to_string();
        assert!(
            matches!(err, MeeArchiveError::CrcMismatch { .. }),
            "beklenen CrcMismatch, gelen: {err:?}"
        );
        assert!(msg.contains("CRC32"), "mesaj: {msg}");
    }

    #[test]
    fn path_traversal_entry_is_blocked_with_turkish_error() {
        // Kötü niyetli `../kotu.txt` girdisi içeren sahte ZIP oluştur
        let name = b"../kotu.txt";
        let data = b"yasak";
        let crc = crc32_ieee(data);
        let size = data.len() as u32;
        let mut raw = Vec::new();

        // LFH
        raw.extend_from_slice(&LFH_SIGNATURE.to_le_bytes());
        raw.extend_from_slice(&ZIP_VERSION_20.to_le_bytes());
        raw.extend_from_slice(&ZIP_FLAG_UTF8.to_le_bytes());
        raw.extend_from_slice(&ZIP_METHOD_STORED.to_le_bytes());
        raw.extend_from_slice(&0u16.to_le_bytes());
        raw.extend_from_slice(&0u16.to_le_bytes());
        raw.extend_from_slice(&crc.to_le_bytes());
        raw.extend_from_slice(&size.to_le_bytes());
        raw.extend_from_slice(&size.to_le_bytes());
        raw.extend_from_slice(&(name.len() as u16).to_le_bytes());
        raw.extend_from_slice(&0u16.to_le_bytes());
        raw.extend_from_slice(name);
        raw.extend_from_slice(data);

        let cd_offset = raw.len() as u32;
        // CD
        raw.extend_from_slice(&CD_SIGNATURE.to_le_bytes());
        raw.extend_from_slice(&ZIP_VERSION_20.to_le_bytes());
        raw.extend_from_slice(&ZIP_VERSION_20.to_le_bytes());
        raw.extend_from_slice(&ZIP_FLAG_UTF8.to_le_bytes());
        raw.extend_from_slice(&ZIP_METHOD_STORED.to_le_bytes());
        raw.extend_from_slice(&0u16.to_le_bytes());
        raw.extend_from_slice(&0u16.to_le_bytes());
        raw.extend_from_slice(&crc.to_le_bytes());
        raw.extend_from_slice(&size.to_le_bytes());
        raw.extend_from_slice(&size.to_le_bytes());
        raw.extend_from_slice(&(name.len() as u16).to_le_bytes());
        raw.extend_from_slice(&0u16.to_le_bytes());
        raw.extend_from_slice(&0u16.to_le_bytes());
        raw.extend_from_slice(&0u16.to_le_bytes());
        raw.extend_from_slice(&0u16.to_le_bytes());
        raw.extend_from_slice(&0u32.to_le_bytes());
        raw.extend_from_slice(&0u32.to_le_bytes());
        raw.extend_from_slice(name);

        let cd_size = (raw.len() as u32) - cd_offset;
        // EOCD
        raw.extend_from_slice(&EOCD_SIGNATURE.to_le_bytes());
        raw.extend_from_slice(&0u16.to_le_bytes());
        raw.extend_from_slice(&0u16.to_le_bytes());
        raw.extend_from_slice(&1u16.to_le_bytes());
        raw.extend_from_slice(&1u16.to_le_bytes());
        raw.extend_from_slice(&cd_size.to_le_bytes());
        raw.extend_from_slice(&cd_offset.to_le_bytes());
        raw.extend_from_slice(&0u16.to_le_bytes());

        let err = import_mee_archive(&raw).unwrap_err();
        assert!(
            matches!(err, MeeArchiveError::PathTraversal(_)),
            "beklenen PathTraversal, gelen: {err:?}"
        );
        assert!(err.to_string().contains("yol geçişi"));
    }

    #[test]
    fn unsupported_schema_version_and_missing_manifest_return_turkish_errors() {
        let bad_manifest = MeeManifest {
            format: MEE_ARCHIVE_FORMAT.to_string(),
            schema_version: 99,
            app_version: "9.0.0".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            phrase_count: 0,
            asset_count: 0,
        };
        let zip_bad_ver = write_zip_stored(&[
            (
                "manifest.json".to_string(),
                serde_json::to_vec(&bad_manifest).unwrap(),
            ),
            (
                "library.json".to_string(),
                br#"{"format":"meetin","version":2,"folders":[],"phrases":[]}"#.to_vec(),
            ),
        ])
        .unwrap();

        let err = import_mee_archive(&zip_bad_ver).unwrap_err();
        assert!(matches!(err, MeeArchiveError::UnsupportedSchemaVersion(99)));
        assert!(err.to_string().contains("şema sürümü"));

        let zip_no_manifest = write_zip_stored(&[(
            "library.json".to_string(),
            br#"{"format":"meetin","version":2,"folders":[],"phrases":[]}"#.to_vec(),
        )])
        .unwrap();

        let err_missing = import_mee_archive(&zip_no_manifest).unwrap_err();
        assert!(matches!(err_missing, MeeArchiveError::MissingEntry(_)));
        assert!(err_missing.to_string().contains("manifest.json"));
    }

    #[test]
    fn missing_referenced_asset_returns_clear_turkish_error() {
        let manifest = MeeManifest {
            format: MEE_ARCHIVE_FORMAT.to_string(),
            schema_version: MEE_SCHEMA_VERSION,
            app_version: "1.2.0".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            phrase_count: 1,
            asset_count: 0,
        };
        let lib = r#"{
            "format": "meetin",
            "version": 2,
            "folders": [],
            "phrases": [{
                "abbr": "img",
                "content": "Görsel",
                "content_html": "<img src=\"assets/eksik.png\">"
            }]
        }"#;
        let zip = write_zip_stored(&[
            (
                "manifest.json".to_string(),
                serde_json::to_vec(&manifest).unwrap(),
            ),
            ("library.json".to_string(), lib.as_bytes().to_vec()),
        ])
        .unwrap();

        let err = import_mee_archive(&zip).unwrap_err();
        assert!(matches!(err, MeeArchiveError::MissingAsset(_)));
        assert!(err.to_string().contains("assets/eksik.png"));
    }
}
