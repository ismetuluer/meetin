//! Güncelleme bütünlük (SHA-256) ve kriptografik imza (ECDSA P-256 / Windows CNG) doğrulayıcısı.
//!
//! Harici kripto crate'i gerektirmeden çevrimdışı (`--offline`) derlenir:
//! - SHA-256: `io::mee_archive::Sha256Hasher` ile sabit bellekli akış özetleme.
//! - ECDSA P-256: Windows CNG (`BCryptVerifySignature`) ile donanım/işletim sistemi destekli doğrulama.

use std::fmt::Write as _;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use crate::io::mee_archive::{base64_decode, sha256_digest, Sha256Hasher};

/// Standart X.509 SubjectPublicKeyInfo (SPKI) DER ön eki — ECDSA prime256v1 (NIST P-256), sıkıştırılmamış nokta (0x04).
const SPKI_P256_PREFIX: [u8; 27] = [
    0x30, 0x59, // SEQUENCE (89 bayt)
    0x30, 0x13, // SEQUENCE (19 bayt) AlgorithmIdentifier
    0x06, 0x07, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x02, 0x01, // OID 1.2.840.10045.2.1 (id-ecPublicKey)
    0x06, 0x08, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x03, 0x01, 0x07, // OID 1.2.840.10045.3.1.7 (prime256v1)
    0x03, 0x42, 0x00, 0x04, // BIT STRING (66 bayt), 0 kullanılmayan bit, 0x04 sıkıştırılmamış EC noktası
];

/// Windows CNG `BCRYPT_ECDSA_PUBLIC_P256_MAGIC` ("ECS1", little-endian).
const BCRYPT_ECDSA_PUBLIC_P256_MAGIC_BYTES: [u8; 4] = *b"ECS1";

/// 32 baytlık SHA-256 özetini küçük harfli 64 karakterlik hex dizgesine çevirir.
pub fn digest_to_hex(digest: &[u8; 32]) -> String {
    let mut hex = String::with_capacity(64);
    for &b in digest {
        let _ = write!(&mut hex, "{b:02x}");
    }
    hex
}

/// ASCII hex karakterlerini küçük harfe normalleştirir (yerel ayar bağımsız ASCII dönüştürme).
pub fn normalize_hex_lower(raw: &str) -> String {
    let trimmed = raw.trim();
    let mut out = String::with_capacity(trimmed.len());
    for b in trimmed.bytes() {
        let norm = if b.is_ascii_uppercase() { b + 32 } else { b };
        out.push(norm as char);
    }
    out
}

fn hex_nibble(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// 64 karakterlik SHA-256 hex dizgesini 32 baytlık diziye çözümler.
pub fn parse_sha256_hex(raw: &str) -> Result<[u8; 32], String> {
    let trimmed = raw.trim();
    let bytes = trimmed.as_bytes();
    if bytes.len() != 64 {
        return Err(format!(
            "Geçersiz SHA-256 özeti uzunluğu: {} karakter (64 bekleniyor).",
            bytes.len()
        ));
    }
    let mut out = [0u8; 32];
    for i in 0..32 {
        let hi = hex_nibble(bytes[i * 2])
            .ok_or_else(|| "SHA-256 özeti geçersiz onaltılık karakter içeriyor.".to_string())?;
        let lo = hex_nibble(bytes[i * 2 + 1])
            .ok_or_else(|| "SHA-256 özeti geçersiz onaltılık karakter içeriyor.".to_string())?;
        out[i] = (hi << 4) | lo;
    }
    Ok(out)
}

/// İki 32 baytlık özeti sabit zamanlı olarak bayt bayt karşılaştırır.
pub fn constant_time_eq_32(a: &[u8; 32], b: &[u8; 32]) -> bool {
    let mut diff = 0u8;
    for i in 0..32 {
        diff |= a[i] ^ b[i];
    }
    diff == 0
}

/// Bir dosyanın SHA-256 özetini 64 KiB bloklarla akış halinde hesaplar.
pub fn sha256_file(path: &Path) -> Result<[u8; 32], String> {
    let mut file =
        File::open(path).map_err(|e| format!("Dosya SHA-256 doğrulaması için açılamadı: {e}"))?;
    let mut hasher = Sha256Hasher::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buf)
            .map_err(|e| format!("Dosya okunurken hata oluştu: {e}"))?;
        if read == 0 {
            break;
        }
        hasher.update(&buf[..read]);
    }
    Ok(hasher.finalize())
}

/// İndirilen dosyanın SHA-256 özetini manifestteki beklenen hex değerle bayt bayt doğrular.
pub fn verify_file_sha256(path: &Path, expected_hex: &str) -> Result<String, String> {
    let expected = parse_sha256_hex(expected_hex)?;
    let actual = sha256_file(path)?;
    if !constant_time_eq_32(&expected, &actual) {
        return Err(format!(
            "SHA-256 bütünlük doğrulaması başarısız oldu (beklenen: {}, hesaplanan: {}). Güncelleme güvenlik nedeniyle iptal edildi.",
            digest_to_hex(&expected),
            digest_to_hex(&actual)
        ));
    }
    Ok(digest_to_hex(&actual))
}

/// Manifest imzalama ve doğrulama için kanonik metin yükünü oluşturur (`version\nsha256\ndownload_url`).
pub fn canonical_signing_payload(version: &str, sha256: &str, download_url: &str) -> String {
    format!(
        "{}\n{}\n{}",
        version.trim(),
        normalize_hex_lower(sha256),
        download_url.trim()
    )
}

/// PEM (`-----BEGIN PUBLIC KEY-----`) veya ham Base64 biçimindeki ECDSA P-256 açık anahtarını
/// Windows CNG `BCRYPT_ECCPUBLIC_BLOB` (72 bayt) biçimine dönüştürür.
pub fn parse_p256_public_key_to_cng_blob(raw_key: &str) -> Result<[u8; 72], String> {
    let mut b64 = String::with_capacity(raw_key.len());
    for line in raw_key.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("-----") {
            continue;
        }
        b64.push_str(trimmed);
    }

    let decoded = base64_decode(&b64)
        .ok_or_else(|| "Açık anahtar geçerli Base64 biçiminde değil.".to_string())?;

    let xy: &[u8] = if decoded.len() == 91 && decoded[..27] == SPKI_P256_PREFIX {
        // Standart X.509 SPKI DER (91 bayt)
        &decoded[27..91]
    } else if decoded.len() == 65 && decoded[0] == 0x04 {
        // SEC1 sıkıştırılmamış nokta (0x04 || X || Y)
        &decoded[1..65]
    } else if decoded.len() == 64 {
        // Ham koordinatlar (X || Y)
        &decoded[0..64]
    } else if decoded.len() == 72
        && decoded[0..4] == BCRYPT_ECDSA_PUBLIC_P256_MAGIC_BYTES
        && u32::from_le_bytes([decoded[4], decoded[5], decoded[6], decoded[7]]) == 32
    {
        // Doğrudan Windows CNG BCRYPT_ECCPUBLIC_BLOB
        &decoded[8..72]
    } else {
        return Err(format!(
            "Desteklenmeyen ECDSA P-256 açık anahtar biçimi ({} bayt).",
            decoded.len()
        ));
    };

    let mut blob = [0u8; 72];
    blob[0..4].copy_from_slice(&BCRYPT_ECDSA_PUBLIC_P256_MAGIC_BYTES);
    blob[4..8].copy_from_slice(&32u32.to_le_bytes());
    blob[8..72].copy_from_slice(xy);
    Ok(blob)
}

/// 64 baytlık X||Y koordinatlarını standart X.509 SPKI DER (91 bayt) olarak kodlar.
#[cfg(test)]
pub fn encode_p256_spki_der(xy: &[u8; 64]) -> [u8; 91] {
    let mut out = [0u8; 91];
    out[..27].copy_from_slice(&SPKI_P256_PREFIX);
    out[27..91].copy_from_slice(xy);
    out
}

/// Base64 kodlu ECDSA P-256 imzasını IEEE P1363 (64 bayt `r || s`) biçimine çözümler.
/// Hem ham 64 bayt IEEE P1363 (.NET / Windows CNG) hem de ASN.1 DER (OpenSSL) imzalarını destekler.
pub fn decode_ecdsa_p256_signature(signature_b64: &str) -> Result<[u8; 64], String> {
    let decoded = base64_decode(signature_b64.trim())
        .ok_or_else(|| "İmza geçerli Base64 biçiminde değil.".to_string())?;
    if decoded.len() == 64 {
        let mut out = [0u8; 64];
        out.copy_from_slice(&decoded);
        return Ok(out);
    }
    if decoded.len() >= 8 && decoded[0] == 0x30 {
        return der_ecdsa_to_p1363(&decoded);
    }
    Err(format!(
        "Geçersiz ECDSA P-256 imza uzunluğu ({} bayt).",
        decoded.len()
    ))
}

fn read_der_integer_32(data: &[u8], cursor: &mut usize) -> Result<[u8; 32], String> {
    if *cursor + 2 > data.len() || data[*cursor] != 0x02 {
        return Err("ASN.1 DER imza tamsayı etiketi geçersiz.".to_string());
    }
    let len = data[*cursor + 1] as usize;
    *cursor += 2;
    if len == 0 || *cursor + len > data.len() {
        return Err("ASN.1 DER imza tamsayı uzunluğu geçersiz.".to_string());
    }
    let mut slice = &data[*cursor..*cursor + len];
    *cursor += len;

    // İşaret baytı (0x00 ön eki) varsa atla
    while slice.len() > 32 && slice[0] == 0x00 {
        slice = &slice[1..];
    }
    if slice.len() > 32 {
        return Err("ASN.1 DER imza bileşeni 32 baytı aşıyor.".to_string());
    }

    let mut out = [0u8; 32];
    out[32 - slice.len()..].copy_from_slice(slice);
    Ok(out)
}

fn der_ecdsa_to_p1363(der: &[u8]) -> Result<[u8; 64], String> {
    if der.len() < 8 || der[0] != 0x30 {
        return Err("ASN.1 DER imza dizisi geçersiz.".to_string());
    }
    let seq_len = der[1] as usize;
    if seq_len + 2 != der.len() {
        return Err("ASN.1 DER imza uzunluğu dosya sonuyla eşleşmiyor.".to_string());
    }
    let mut cursor = 2usize;
    let r = read_der_integer_32(der, &mut cursor)?;
    let s = read_der_integer_32(der, &mut cursor)?;
    if cursor != der.len() {
        return Err("ASN.1 DER imzasında beklenmeyen fazla baytlar var.".to_string());
    }
    let mut out = [0u8; 64];
    out[..32].copy_from_slice(&r);
    out[32..64].copy_from_slice(&s);
    Ok(out)
}

/// Kanonik yükün ECDSA P-256 (SHA-256) imzasını verilen açık anahtarla doğrular.
pub fn verify_ecdsa_p256_signature(
    payload: &[u8],
    signature_b64: &str,
    public_key_str: &str,
) -> Result<bool, String> {
    let pub_blob = parse_p256_public_key_to_cng_blob(public_key_str)?;
    let sig_p1363 = decode_ecdsa_p256_signature(signature_b64)?;
    let hash = sha256_digest(payload);

    #[cfg(windows)]
    {
        win_cng::verify_p256_hash_signature(&pub_blob, &hash, &sig_p1363)
    }
    #[cfg(not(windows))]
    {
        let _ = (pub_blob, sig_p1363, hash);
        Err("ECDSA P-256 imza doğrulaması yalnızca Windows CNG üzerinde desteklenir.".to_string())
    }
}

#[cfg(windows)]
mod win_cng {
    use windows::core::PCWSTR;
    use windows::Win32::Security::Cryptography::{
        BCryptCloseAlgorithmProvider, BCryptDestroyKey, BCryptImportKeyPair,
        BCryptOpenAlgorithmProvider, BCryptVerifySignature, BCRYPT_ALG_HANDLE,
        BCRYPT_ECCPUBLIC_BLOB, BCRYPT_ECDSA_P256_ALGORITHM, BCRYPT_FLAGS, BCRYPT_KEY_HANDLE,
        BCRYPT_OPEN_ALGORITHM_PROVIDER_FLAGS,
    };

    struct AlgGuard(BCRYPT_ALG_HANDLE);
    impl Drop for AlgGuard {
        fn drop(&mut self) {
            if !self.0 .0.is_null() {
                unsafe {
                    let _ = BCryptCloseAlgorithmProvider(self.0, 0);
                }
            }
        }
    }

    struct KeyGuard(BCRYPT_KEY_HANDLE);
    impl Drop for KeyGuard {
        fn drop(&mut self) {
            if !self.0 .0.is_null() {
                unsafe {
                    let _ = BCryptDestroyKey(self.0);
                }
            }
        }
    }

    pub fn verify_p256_hash_signature(
        pub_blob: &[u8; 72],
        sha256_hash: &[u8; 32],
        sig_p1363: &[u8; 64],
    ) -> Result<bool, String> {
        let mut alg_handle = BCRYPT_ALG_HANDLE::default();
        let status = unsafe {
            BCryptOpenAlgorithmProvider(
                &mut alg_handle,
                BCRYPT_ECDSA_P256_ALGORITHM,
                PCWSTR::null(),
                BCRYPT_OPEN_ALGORITHM_PROVIDER_FLAGS(0),
            )
        };
        if status.is_err() {
            return Err(format!(
                "Windows CNG ECDSA P-256 sağlayıcısı açılamadı (NTSTATUS={:#x}).",
                status.0
            ));
        }
        let _alg_guard = AlgGuard(alg_handle);

        let mut key_handle = BCRYPT_KEY_HANDLE::default();
        let import_status = unsafe {
            BCryptImportKeyPair(
                alg_handle,
                BCRYPT_KEY_HANDLE::default(),
                BCRYPT_ECCPUBLIC_BLOB,
                &mut key_handle,
                pub_blob,
                0,
            )
        };
        if import_status.is_err() {
            return Err(format!(
                "ECDSA P-256 açık anahtarı içeri aktarılamadı (NTSTATUS={:#x}).",
                import_status.0
            ));
        }
        let _key_guard = KeyGuard(key_handle);

        let verify_status = unsafe {
            BCryptVerifySignature(
                key_handle,
                None,
                sha256_hash,
                sig_p1363,
                BCRYPT_FLAGS(0),
            )
        };
        Ok(verify_status.is_ok())
    }

    /// Yalnızca birim testleri için gerçek bir Windows CNG ECDSA P-256 anahtar çifti üretir ve imzalar.
    #[cfg(test)]
    pub fn generate_test_keypair_and_sign(payload: &[u8]) -> Result<([u8; 72], [u8; 64]), String> {
        use crate::io::mee_archive::sha256_digest;
        use windows::Win32::Security::Cryptography::{
            BCryptExportKey, BCryptFinalizeKeyPair, BCryptGenerateKeyPair, BCryptSignHash,
        };

        let mut alg_handle = BCRYPT_ALG_HANDLE::default();
        let status = unsafe {
            BCryptOpenAlgorithmProvider(
                &mut alg_handle,
                BCRYPT_ECDSA_P256_ALGORITHM,
                PCWSTR::null(),
                BCRYPT_OPEN_ALGORITHM_PROVIDER_FLAGS(0),
            )
        };
        if status.is_err() {
            return Err(format!("CNG açılamadı: {:#x}", status.0));
        }
        let _alg_guard = AlgGuard(alg_handle);

        let mut key_handle = BCRYPT_KEY_HANDLE::default();
        let gen_status = unsafe { BCryptGenerateKeyPair(alg_handle, &mut key_handle, 256, 0) };
        if gen_status.is_err() {
            return Err(format!("Anahtar çifti üretilemedi: {:#x}", gen_status.0));
        }
        let _key_guard = KeyGuard(key_handle);

        let fin_status = unsafe { BCryptFinalizeKeyPair(key_handle, 0) };
        if fin_status.is_err() {
            return Err(format!("Anahtar çifti tamamlanamadı: {:#x}", fin_status.0));
        }

        let mut pub_blob = [0u8; 72];
        let mut pub_len = 0u32;
        let exp_status = unsafe {
            BCryptExportKey(
                key_handle,
                BCRYPT_KEY_HANDLE::default(),
                BCRYPT_ECCPUBLIC_BLOB,
                Some(&mut pub_blob),
                &mut pub_len,
                0,
            )
        };
        if exp_status.is_err() || pub_len != 72 {
            return Err(format!("Açık anahtar dışa aktarılamadı: {:#x}", exp_status.0));
        }

        let hash = sha256_digest(payload);
        let mut sig = [0u8; 64];
        let mut sig_len = 0u32;
        let sign_status = unsafe {
            BCryptSignHash(
                key_handle,
                None,
                &hash,
                Some(&mut sig),
                &mut sig_len,
                BCRYPT_FLAGS(0),
            )
        };
        if sign_status.is_err() || sig_len != 64 {
            return Err(format!("İmza üretilemedi: {:#x}", sign_status.0));
        }

        Ok((pub_blob, sig))
    }
}

#[cfg(all(test, windows))]
pub(crate) fn test_generate_keypair_and_sign(payload: &[u8]) -> Result<([u8; 72], [u8; 64]), String> {
    win_cng::generate_test_keypair_and_sign(payload)
}
