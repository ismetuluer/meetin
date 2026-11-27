//! Doğrulamalı ve imzalı öz-güncelleme çekirdeği, WebView2 kontrolü ve portable yükseltme (Faz D, §9).

pub mod crypto;

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::windows::startup::is_portable_mode;

use self::crypto::{
    canonical_signing_payload, decode_ecdsa_p256_signature, parse_sha256_hex,
    verify_ecdsa_p256_signature, verify_file_sha256,
};

/// Derleme zamanında `MEETIN_UPDATE_PUBLIC_KEY` ortam değişkeniyle gömülebilen
/// varsayılan ECDSA P-256 açık anahtarı (PEM veya Base64 SPKI).
/// Özel anahtar ASLA depoya konmaz; yalnızca açık anahtar dağıtılır.
pub const EMBEDDED_UPDATE_PUBLIC_KEY: Option<&str> = option_env!("MEETIN_UPDATE_PUBLIC_KEY");

pub const SIG_STATUS_VERIFIED: &str = "verified";
pub const SIG_STATUS_UNSIGNED: &str = "unsigned";
pub const SIG_STATUS_INVALID: &str = "invalid";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateManifest {
    pub version: String,
    #[serde(default)]
    pub pub_date: Option<String>,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub download_url: String,
    #[serde(default)]
    pub sha256: Option<String>,
    #[serde(default)]
    pub portable_url: Option<String>,
    #[serde(default)]
    pub portable_sha256: Option<String>,
    #[serde(default)]
    pub signature: Option<String>,
    #[serde(default)]
    pub key_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCheckResult {
    pub current_version: String,
    pub latest_version: Option<String>,
    pub update_available: bool,
    pub notes: String,
    pub download_url: String,
    pub is_portable: bool,
    pub webview2_installed: bool,
    pub signature_status: String,
    pub can_auto_install: bool,
    pub sha256: Option<String>,
    pub pub_date: Option<String>,
    pub key_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInstallResult {
    pub version: String,
    pub mode: String,
    pub signature_status: String,
    pub staged_path: String,
    pub will_restart: bool,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct StagedUpdate {
    pub version: String,
    pub signature_status: String,
    pub verified_sha256: String,
    pub artifact_path: PathBuf,
}

fn parse_semver(raw: &str) -> Option<(u64, u64, u64)> {
    let clean = raw.trim().trim_start_matches('v');
    let main = clean.split('-').next().unwrap_or(clean);
    let mut parts = main.split('.');
    let major = parts.next()?.parse::<u64>().ok()?;
    let minor = parts.next()?.parse::<u64>().ok()?;
    let patch = parts.next()?.parse::<u64>().ok()?;
    Some((major, minor, patch))
}

pub fn is_newer_version(current: &str, candidate: &str) -> bool {
    match (parse_semver(current), parse_semver(candidate)) {
        (Some(cur), Some(cand)) => cand > cur,
        _ => false,
    }
}

/// Yapılandırılmış ECDSA P-256 açık anahtarını çözer (öncelik: çalışma zamanı ortam değişkeni, sonra derleme zamanı gömülü anahtar).
pub fn resolve_configured_public_key() -> Option<String> {
    if let Ok(env_val) = std::env::var("MEETIN_UPDATE_PUBLIC_KEY") {
        let trimmed = env_val.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    EMBEDDED_UPDATE_PUBLIC_KEY
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
}

/// İndirme adresinin güvenli (`https://`) veya test/yerel manifest kapsamında geçerli bir dosya yolu olduğunu denetler.
pub fn validate_download_source(url: &str, allow_local_file: bool) -> Result<(), String> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return Err("Güncelleme indirme adresi boş.".to_string());
    }
    if trimmed.len() >= 8 && trimmed.as_bytes()[..8].eq_ignore_ascii_case(b"https://") {
        return Ok(());
    }
    if allow_local_file {
        let local = strip_file_scheme(trimmed);
        if Path::new(local).is_file() {
            return Ok(());
        }
    }
    Err("Güvenlik hatası: Güncelleme paketi yalnızca HTTPS üzerinden indirilebilir.".to_string())
}

fn strip_file_scheme(raw: &str) -> &str {
    let trimmed = raw.trim();
    if trimmed.len() >= 8 && trimmed.as_bytes()[..8].eq_ignore_ascii_case(b"file:///") {
        &trimmed[8..]
    } else if trimmed.len() >= 7 && trimmed.as_bytes()[..7].eq_ignore_ascii_case(b"file://") {
        &trimmed[7..]
    } else {
        trimmed
    }
}

/// Manifestteki kriptografik imzayı değerlendirir (`"verified" | "unsigned" | "invalid"`).
pub fn evaluate_manifest_signature(
    manifest: &UpdateManifest,
    public_key: Option<&str>,
) -> &'static str {
    let sig_raw = manifest.signature.as_deref().map(str::trim).unwrap_or("");
    if sig_raw.is_empty() {
        return SIG_STATUS_UNSIGNED;
    }

    // İmza mevcutsa biçimi ve kanonik yük alanları mutlaka geçerli olmalı.
    if decode_ecdsa_p256_signature(sig_raw).is_err() {
        return SIG_STATUS_INVALID;
    }
    let sha_raw = manifest.sha256.as_deref().map(str::trim).unwrap_or("");
    if parse_sha256_hex(sha_raw).is_err() || manifest.download_url.trim().is_empty() {
        return SIG_STATUS_INVALID;
    }

    let Some(key_str) = public_key.map(str::trim).filter(|k| !k.is_empty()) else {
        // Yapılandırılmış açık anahtar yoksa doğrulandı iddiasında bulunma.
        return SIG_STATUS_UNSIGNED;
    };

    let payload = canonical_signing_payload(&manifest.version, sha_raw, &manifest.download_url);
    match verify_ecdsa_p256_signature(payload.as_bytes(), sig_raw, key_str) {
        Ok(true) => SIG_STATUS_VERIFIED,
        Ok(false) | Err(_) => SIG_STATUS_INVALID,
    }
}

/// Çalışma moduna göre (NSIS veya Portable) kullanılacak indirme adresi ve beklenen SHA-256 özetini seçer.
pub fn select_artifact_for_mode(
    manifest: &UpdateManifest,
    is_portable: bool,
) -> (String, Option<String>) {
    if is_portable {
        if let Some(p_url) = manifest
            .portable_url
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            let p_sha = manifest
                .portable_sha256
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(ToString::to_string);
            return (p_url.to_string(), p_sha);
        }
    }
    let sha = manifest
        .sha256
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string);
    (manifest.download_url.trim().to_string(), sha)
}

/// EdgeWebView2 çalışma zamanının kurulu olup olmadığını denetler (§9).
pub fn is_webview2_installed() -> bool {
    #[cfg(windows)]
    {
        use std::ffi::c_void;
        use windows::core::PCWSTR;
        use windows::Win32::System::Registry::{
            RegGetValueW, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ,
        };

        let subkeys = [
            r"SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}",
            r"SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}",
            r"Software\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}",
        ];
        let val_w: Vec<u16> = "pv\0".encode_utf16().collect();
        for (idx, sub) in subkeys.iter().enumerate() {
            let sub_w: Vec<u16> = sub.encode_utf16().chain(std::iter::once(0)).collect();
            let root = if idx == 2 {
                HKEY_CURRENT_USER
            } else {
                HKEY_LOCAL_MACHINE
            };
            let mut size = 0u32;
            let status = unsafe {
                RegGetValueW(
                    root,
                    PCWSTR(sub_w.as_ptr()),
                    PCWSTR(val_w.as_ptr()),
                    RRF_RT_REG_SZ,
                    None,
                    None::<*mut c_void>,
                    Some(&mut size),
                )
            };
            if status.is_ok() && size > 2 {
                return true;
            }
        }
        false
    }
    #[cfg(not(windows))]
    {
        true
    }
}

pub fn current_env_info() -> UpdateCheckResult {
    UpdateCheckResult {
        current_version: env!("CARGO_PKG_VERSION").to_string(),
        latest_version: None,
        update_available: false,
        notes: String::new(),
        download_url: String::new(),
        is_portable: is_portable_mode(),
        webview2_installed: is_webview2_installed(),
        signature_status: SIG_STATUS_UNSIGNED.to_string(),
        can_auto_install: false,
        sha256: None,
        pub_date: None,
        key_id: None,
    }
}

pub fn parse_manifest_json(raw: &str) -> Result<UpdateManifest, String> {
    serde_json::from_str(raw.trim_start_matches('\u{FEFF}'))
        .map_err(|e| format!("latest.json ayrıştırılamadı: {e}"))
}

pub fn evaluate_manifest_check(
    manifest: &UpdateManifest,
    current_version: &str,
    is_portable: bool,
    public_key: Option<&str>,
    allow_local_file: bool,
) -> UpdateCheckResult {
    let update_available = is_newer_version(current_version, &manifest.version);
    let signature_status = evaluate_manifest_signature(manifest, public_key);
    let (target_url, target_sha256) = select_artifact_for_mode(manifest, is_portable);

    let url_valid = validate_download_source(&target_url, allow_local_file).is_ok();
    let sha_valid = target_sha256
        .as_deref()
        .is_some_and(|s| parse_sha256_hex(s).is_ok());

    let can_auto_install =
        update_available && signature_status != SIG_STATUS_INVALID && url_valid && sha_valid;

    UpdateCheckResult {
        current_version: current_version.to_string(),
        latest_version: Some(manifest.version.clone()),
        update_available,
        notes: manifest.notes.clone(),
        download_url: target_url,
        is_portable,
        webview2_installed: is_webview2_installed(),
        signature_status: signature_status.to_string(),
        can_auto_install,
        sha256: target_sha256,
        pub_date: manifest.pub_date.clone(),
        key_id: manifest.key_id.clone(),
    }
}

pub fn parse_latest_json_with_key(
    raw: &str,
    current_version: &str,
    is_portable: bool,
    public_key: Option<&str>,
    allow_local_file: bool,
) -> Result<UpdateCheckResult, String> {
    let manifest = parse_manifest_json(raw)?;
    Ok(evaluate_manifest_check(
        &manifest,
        current_version,
        is_portable,
        public_key,
        allow_local_file,
    ))
}

pub fn parse_latest_json(
    raw: &str,
    current_version: &str,
    is_portable: bool,
) -> Result<UpdateCheckResult, String> {
    let pub_key = resolve_configured_public_key();
    parse_latest_json_with_key(raw, current_version, is_portable, pub_key.as_deref(), false)
}

/// İndirilen `latest.json` baytlarını isteğe bağlı UTF-8 BOM'dan arındırarak metne çevirir.
pub fn decode_manifest_bytes(bytes: &[u8]) -> String {
    let raw = match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => String::from_utf8_lossy(bytes).into_owned(),
    };
    raw.trim_start_matches('\u{FEFF}').to_string()
}

/// Manifest kaynağından (yerel dosya veya `https://`) ham JSON metnini ve yerel olup olmadığını okur.
pub fn fetch_manifest_raw(source: &str) -> Result<(String, bool), String> {
    let trimmed = source.trim();
    if trimmed.is_empty() {
        return Err("Güncelleme bildirim adresi boş.".to_string());
    }
    let local_candidate = strip_file_scheme(trimmed);
    let path = Path::new(local_candidate);
    if path.is_file() {
        let bytes = fs::read(path).map_err(|e| e.to_string())?;
        return Ok((decode_manifest_bytes(&bytes), true));
    }
    if trimmed.len() >= 8 && trimmed.as_bytes()[..8].eq_ignore_ascii_case(b"https://") {
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            use std::process::Command;
            use std::time::{SystemTime, UNIX_EPOCH};
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;

            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            let tmp_path = std::env::temp_dir().join(format!(
                "meetin-latest-{}-{}.json",
                std::process::id(),
                nanos
            ));
            let escaped = trimmed.replace('\'', "''");
            let out_escaped = tmp_path.to_string_lossy().replace('\'', "''");
            // `-OutFile` doğrudan UTF-8 baytları diske yazar; `.Content` octet-stream yanıtlarında Byte[] (123...) dönebiliyor.
            let script = format!(
                "$ProgressPreference='SilentlyContinue'; [Net.ServicePointManager]::SecurityProtocol=[Net.SecurityProtocolType]::Tls12; Invoke-WebRequest -UseBasicParsing -TimeoutSec 15 -Uri '{}' -OutFile '{}'",
                escaped, out_escaped
            );
            let out = Command::new("powershell")
                .args(["-NoProfile", "-NonInteractive", "-Command", &script])
                .creation_flags(CREATE_NO_WINDOW)
                .output();

            let result = match out {
                Ok(out) if out.status.success() => fs::read(&tmp_path)
                    .map(|bytes| (decode_manifest_bytes(&bytes), false))
                    .map_err(|e| format!("Güncelleme bildirimi okunamadı: {e}")),
                Ok(out) => {
                    let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
                    Err(if err.is_empty() {
                        "Güncelleme adresinden yanıt alınamadı.".to_string()
                    } else {
                        format!("Güncelleme denetimi başarısız: {err}")
                    })
                }
                Err(e) => Err(format!("Güncelleme denetimi çalıştırılamadı: {e}")),
            };
            let _ = fs::remove_file(&tmp_path);
            return result;
        }
        #[cfg(not(windows))]
        {
            return Err(
                "HTTPS güncelleme indirme yalnızca Windows üzerinde desteklenir.".to_string(),
            );
        }
    }
    Err("Güncelleme bildirim dosyası bulunamadı veya geçerli bir HTTPS adresi değil.".to_string())
}

/// Yerel `latest.json` dosya yolundan veya `https://` GitHub Releases bildiriminden güncelleme durumunu denetler.
pub fn check_manifest_source(
    source: &str,
    current_version: &str,
    is_portable: bool,
) -> Result<UpdateCheckResult, String> {
    let trimmed = source.trim();
    if trimmed.is_empty() {
        return Ok(current_env_info());
    }
    let (raw, is_local) = fetch_manifest_raw(trimmed)?;
    let pub_key = resolve_configured_public_key();
    parse_latest_json_with_key(
        &raw,
        current_version,
        is_portable,
        pub_key.as_deref(),
        is_local,
    )
}

fn sanitize_filename_from_url(url: &str, fallback: &str) -> String {
    let no_query = url.split('?').next().unwrap_or(url);
    let candidate = no_query.rsplit(['/', '\\']).next().unwrap_or("");
    let cleaned: String = candidate
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
        .collect();
    if cleaned.is_empty() || cleaned == "." || cleaned == ".." {
        fallback.to_string()
    } else {
        cleaned
    }
}

fn download_artifact_to(source: &str, dest: &Path, allow_local_file: bool) -> Result<(), String> {
    validate_download_source(source, allow_local_file)?;
    let trimmed = source.trim();
    if allow_local_file {
        let local = strip_file_scheme(trimmed);
        let src_path = Path::new(local);
        if src_path.is_file() {
            fs::copy(src_path, dest)
                .map_err(|e| format!("Yerel güncelleme paketi kopyalanamadı: {e}"))?;
            return Ok(());
        }
    }

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        use std::process::Command;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let uri_esc = trimmed.replace('\'', "''");
        let dest_esc = dest.to_string_lossy().replace('\'', "''");
        let script = format!(
            "$ProgressPreference='SilentlyContinue'; [Net.ServicePointManager]::SecurityProtocol=[Net.SecurityProtocolType]::Tls12; Invoke-WebRequest -UseBasicParsing -TimeoutSec 180 -Uri '{}' -OutFile '{}'",
            uri_esc, dest_esc
        );
        let out = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| format!("Güncelleme paketi indirilemedi: {e}"))?;
        if !out.status.success() {
            let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
            return Err(if err.is_empty() {
                "Güncelleme paketi indirilirken ağ hatası oluştu.".to_string()
            } else {
                format!("Güncelleme paketi indirilemedi: {err}")
            });
        }
        if !dest.is_file() {
            return Err("İndirilen güncelleme paketi diskte bulunamadı.".to_string());
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = dest;
        Err("HTTPS indirme yalnızca Windows üzerinde desteklenir.".to_string())
    }
}

/// Güncelleme paketini geçici klasöre indirir/hazırlar, kriptografik imzayı ve SHA-256 özetini bayt bayt doğrular.
/// Bozuk özet veya geçersiz imzada indirilen dosyayı siler ve hata döner.
pub fn stage_and_verify_update(
    manifest: &UpdateManifest,
    is_portable: bool,
    staging_dir: &Path,
    public_key: Option<&str>,
    allow_local_file: bool,
) -> Result<StagedUpdate, String> {
    let sig_status = evaluate_manifest_signature(manifest, public_key);
    if sig_status == SIG_STATUS_INVALID {
        return Err(
            "Güvenlik hatası: Güncelleme bildiriminin kriptografik imzası geçersiz. Kurulum reddedildi."
                .to_string(),
        );
    }

    let (target_url, target_sha) = select_artifact_for_mode(manifest, is_portable);
    validate_download_source(&target_url, allow_local_file)?;

    let expected_sha = target_sha
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            "Güvenlik hatası: Güncelleme bildiriminde SHA-256 doğrulama özeti eksik.".to_string()
        })?;
    parse_sha256_hex(expected_sha)?;

    fs::create_dir_all(staging_dir)
        .map_err(|e| format!("Geçici güncelleme klasörü oluşturulamadı: {e}"))?;

    let fallback_name = if is_portable {
        "Meetin_portable_update.exe"
    } else {
        "Meetin_setup_update.exe"
    };
    let file_name = sanitize_filename_from_url(&target_url, fallback_name);
    let artifact_path = staging_dir.join(file_name);
    if artifact_path.exists() {
        let _ = fs::remove_file(&artifact_path);
    }

    download_artifact_to(&target_url, &artifact_path, allow_local_file)?;

    match verify_file_sha256(&artifact_path, expected_sha) {
        Ok(verified_sha256) => Ok(StagedUpdate {
            version: manifest.version.clone(),
            signature_status: sig_status.to_string(),
            verified_sha256,
            artifact_path,
        }),
        Err(err) => {
            let _ = fs::remove_file(&artifact_path);
            Err(err)
        }
    }
}

/// NSIS sessiz kurulum argümanlarını güvenli biçimde oluşturur (`/S` zorunludur).
pub fn build_nsis_silent_args(install_dir: Option<&Path>) -> Vec<String> {
    let mut args = vec!["/S".to_string()];
    if let Some(dir) = install_dir {
        let dir_str = dir.to_string_lossy();
        let trimmed = dir_str.trim();
        if !trimmed.is_empty() && !trimmed.contains(['\r', '\n', '"']) {
            // NSIS kuralı: /D= son argüman olmalı ve tırnak içermemelidir.
            args.push(format!("/D={trimmed}"));
        }
    }
    args
}

fn ps_single_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

/// NSIS sessiz kurulum ve otomatik yeniden başlatma PowerShell betiğini üretir.
pub fn build_nsis_restart_script(
    current_pid: u32,
    installer_path: &Path,
    relaunch_exe: &Path,
    install_dir: Option<&Path>,
) -> String {
    let args = build_nsis_silent_args(install_dir);
    let args_ps = args
        .iter()
        .map(|a| ps_single_quote(a))
        .collect::<Vec<_>>()
        .join(", ");
    let installer_ps = ps_single_quote(&installer_path.to_string_lossy());
    let relaunch_ps = ps_single_quote(&relaunch_exe.to_string_lossy());

    format!(
        "$ErrorActionPreference = 'SilentlyContinue'\r\n\
         for ($i = 0; $i -lt 60; $i++) {{\r\n\
           if (-not (Get-Process -Id {current_pid} -ErrorAction SilentlyContinue)) {{ break }}\r\n\
           Start-Sleep -Milliseconds 250\r\n\
         }}\r\n\
         $installer = {installer_ps}\r\n\
         $relaunch = {relaunch_ps}\r\n\
         $nsisArgs = @({args_ps})\r\n\
         if (Test-Path -LiteralPath $installer) {{\r\n\
           Start-Process -FilePath $installer -ArgumentList $nsisArgs -Wait\r\n\
         }}\r\n\
         if (Test-Path -LiteralPath $relaunch) {{\r\n\
           Start-Process -FilePath $relaunch\r\n\
         }}\r\n\
         Remove-Item -LiteralPath $installer -Force -ErrorAction SilentlyContinue\r\n"
    )
}

/// Portable güncelleme sonrası yeni `Meetin.exe`'yi başlatan PowerShell betiğini üretir.
pub fn build_portable_relaunch_script(current_pid: u32, exe_path: &Path) -> String {
    let exe_ps = ps_single_quote(&exe_path.to_string_lossy());
    format!(
        "$ErrorActionPreference = 'SilentlyContinue'\r\n\
         for ($i = 0; $i -lt 60; $i++) {{\r\n\
           if (-not (Get-Process -Id {current_pid} -ErrorAction SilentlyContinue)) {{ break }}\r\n\
           Start-Sleep -Milliseconds 250\r\n\
         }}\r\n\
         $exe = {exe_ps}\r\n\
         if (Test-Path -LiteralPath $exe) {{\r\n\
           Start-Process -FilePath $exe\r\n\
         }}\r\n"
    )
}

#[cfg(windows)]
fn spawn_detached_ps_script(script_path: &Path) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    const DETACHED_PROCESS: u32 = 0x0000_0008;

    Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ])
        .arg(script_path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS)
        .spawn()
        .map_err(|e| format!("Yeniden başlatma betiği çalıştırılamadı: {e}"))?;
    Ok(())
}

/// Portable paket `.zip` ise içinden `Meetin.exe` dosyasını çıkarır; zaten `.exe` ise doğrudan döndürür.
fn resolve_portable_exe_from_artifact(
    artifact_path: &Path,
    staging_dir: &Path,
) -> Result<PathBuf, String> {
    let ext = artifact_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    if ext.eq_ignore_ascii_case("zip") {
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            use std::process::Command;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            let extract_dir = staging_dir.join("portable-extracted");
            if extract_dir.exists() {
                let _ = fs::remove_dir_all(&extract_dir);
            }
            fs::create_dir_all(&extract_dir)
                .map_err(|e| format!("Portable arşiv açma klasörü oluşturulamadı: {e}"))?;
            let zip_ps = ps_single_quote(&artifact_path.to_string_lossy());
            let dst_ps = ps_single_quote(&extract_dir.to_string_lossy());
            let script = format!(
                "$ErrorActionPreference='Stop'; Add-Type -AssemblyName System.IO.Compression.FileSystem; [System.IO.Compression.ZipFile]::ExtractToDirectory({zip_ps}, {dst_ps})"
            );
            let out = Command::new("powershell")
                .args(["-NoProfile", "-NonInteractive", "-Command", &script])
                .creation_flags(CREATE_NO_WINDOW)
                .output()
                .map_err(|e| format!("Portable ZIP arşivi açılamadı: {e}"))?;
            if !out.status.success() {
                let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
                return Err(format!("Portable ZIP arşivi açılamadı: {err}"));
            }
            // Üst düzeyde veya tek alt klasörde Meetin.exe ara
            let direct = extract_dir.join("Meetin.exe");
            if direct.is_file() {
                return Ok(direct);
            }
            if let Ok(entries) = fs::read_dir(&extract_dir) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.is_file()
                        && p.file_name()
                            .and_then(|n| n.to_str())
                            .is_some_and(|n| n.eq_ignore_ascii_case("meetin.exe"))
                    {
                        return Ok(p);
                    }
                    if p.is_dir() {
                        let nested = p.join("Meetin.exe");
                        if nested.is_file() {
                            return Ok(nested);
                        }
                    }
                }
            }
            return Err("Portable ZIP arşivi içinde Meetin.exe bulunamadı.".to_string());
        }
        #[cfg(not(windows))]
        {
            let _ = staging_dir;
            return Err("Portable ZIP açma yalnızca Windows üzerinde desteklenir.".to_string());
        }
    }
    Ok(artifact_path.to_path_buf())
}

/// Portable güncellemede çalışan EXE'yi `Meetin.old.exe` olarak kenara alır ve yenisini kopyalar (§9).
/// `data\` klasörüne kesinlikle dokunmaz.
pub fn prepare_portable_self_update(current_exe: &Path, new_exe: &Path) -> Result<PathBuf, String> {
    if !new_exe.is_file() {
        return Err("Yeni çalıştırılabilir dosya bulunamadı".to_string());
    }
    let parent = current_exe
        .parent()
        .ok_or_else(|| "Geçersiz EXE yolu".to_string())?;
    let backup_old = parent.join("Meetin.old.exe");
    if backup_old.exists() {
        let _ = fs::remove_file(&backup_old);
    }
    if current_exe.exists() {
        fs::rename(current_exe, &backup_old).map_err(|e| e.to_string())?;
    }
    fs::copy(new_exe, current_exe).map_err(|e| e.to_string())?;
    Ok(backup_old)
}

/// Tek tıkla doğrulamalı güncelleme kurulumunu (NSIS sessiz veya Portable) başlatır.
pub fn install_verified_update_from_source(
    source: &str,
    current_version: &str,
    is_portable: bool,
) -> Result<UpdateInstallResult, String> {
    let (raw, is_local) = fetch_manifest_raw(source)?;
    let manifest = parse_manifest_json(&raw)?;
    if !is_newer_version(current_version, &manifest.version) {
        return Err(format!(
            "Meetin zaten güncel (mevcut: v{current_version}, bildirim: v{}).",
            manifest.version
        ));
    }

    let pub_key = resolve_configured_public_key();
    let pid = std::process::id();
    let staging_dir = std::env::temp_dir()
        .join("meetin-updater")
        .join(format!("v{}-{pid}", manifest.version.trim()));

    let staged = stage_and_verify_update(
        &manifest,
        is_portable,
        &staging_dir,
        pub_key.as_deref(),
        is_local,
    )?;

    let current_exe =
        std::env::current_exe().map_err(|e| format!("Mevcut uygulama yolu okunamadı: {e}"))?;

    if is_portable {
        let new_exe = resolve_portable_exe_from_artifact(&staged.artifact_path, &staging_dir)?;
        prepare_portable_self_update(&current_exe, &new_exe)?;

        let script_path = staging_dir.join("relaunch-portable.ps1");
        let script = build_portable_relaunch_script(pid, &current_exe);
        fs::write(&script_path, script.as_bytes())
            .map_err(|e| format!("Yeniden başlatma betiği yazılamadı: {e}"))?;

        #[cfg(windows)]
        spawn_detached_ps_script(&script_path)?;

        Ok(UpdateInstallResult {
            version: staged.version,
            mode: "portable".to_string(),
            signature_status: staged.signature_status,
            staged_path: current_exe.to_string_lossy().into_owned(),
            will_restart: true,
            message: "Portable güncelleme doğrulandı; Meetin yeniden başlatılıyor.".to_string(),
        })
    } else {
        let script_path = staging_dir.join("apply-nsis-update.ps1");
        let script = build_nsis_restart_script(pid, &staged.artifact_path, &current_exe, None);
        fs::write(&script_path, script.as_bytes())
            .map_err(|e| format!("NSIS güncelleme betiği yazılamadı: {e}"))?;

        #[cfg(windows)]
        spawn_detached_ps_script(&script_path)?;

        Ok(UpdateInstallResult {
            version: staged.version,
            mode: "nsis".to_string(),
            signature_status: staged.signature_status,
            staged_path: staged.artifact_path.to_string_lossy().into_owned(),
            will_restart: true,
            message: "Sessiz kurulum doğrulandı; Meetin güncellenip yeniden başlatılıyor."
                .to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::mee_archive::{base64_encode, sha256_digest};
    use crate::updater::crypto::{digest_to_hex, encode_p256_spki_der, verify_file_sha256};
    use tempfile::TempDir;

    #[test]
    fn semver_comparison_works() {
        assert!(is_newer_version("0.5.9", "0.5.10"));
        assert!(is_newer_version("0.5.9", "1.0.0"));
        assert!(is_newer_version("1.2.0", "v1.3.0"));
        assert!(!is_newer_version("0.5.9", "0.5.9"));
        assert!(!is_newer_version("0.5.9", "0.5.8"));
    }

    #[test]
    fn sha256_file_verification_accepts_match_and_rejects_corrupted_file() {
        let dir = TempDir::new().unwrap();
        let pkg = dir.path().join("Meetin_1.3.0_x64-setup.exe");
        let content = b"meetin-installer-payload-binary-bytes-0123456789";
        fs::write(&pkg, content).unwrap();

        let expected_hex = digest_to_hex(&sha256_digest(content));
        let verified = verify_file_sha256(&pkg, &expected_hex).unwrap();
        assert_eq!(verified, expected_hex);

        // Dosyayı 1 bayt boz -> kesinlikle reddedilmeli
        let mut corrupted = content.to_vec();
        corrupted[5] ^= 0xFF;
        fs::write(&pkg, &corrupted).unwrap();
        let err = verify_file_sha256(&pkg, &expected_hex).unwrap_err();
        assert!(err.contains("SHA-256"), "unexpected error: {err}");
    }

    #[cfg(windows)]
    #[test]
    fn real_ecdsa_p256_manifest_signing_and_tamper_detection() {
        let version = "1.3.0";
        let sha256 = digest_to_hex(&sha256_digest(b"sample-release-binary"));
        let download_url =
            "https://github.com/OWNER/meetin/releases/download/v1.3.0/Meetin_1.3.0_x64-setup.exe";
        let payload = canonical_signing_payload(version, &sha256, download_url);

        // Gerçek Windows CNG ECDSA P-256 anahtar çifti üret ve kanonik yükü imzala
        let (pub_blob, sig_bytes) =
            crypto::test_generate_keypair_and_sign(payload.as_bytes()).unwrap();
        let sig_b64 = base64_encode(&sig_bytes);

        // Açık anahtarı standart X.509 SPKI PEM biçimine çevir
        let mut xy = [0u8; 64];
        xy.copy_from_slice(&pub_blob[8..72]);
        let spki_der = encode_p256_spki_der(&xy);
        let pub_pem = format!(
            "-----BEGIN PUBLIC KEY-----\n{}\n-----END PUBLIC KEY-----",
            base64_encode(&spki_der)
        );

        let manifest = UpdateManifest {
            version: version.to_string(),
            pub_date: Some("2026-11-28T10:00:00Z".to_string()),
            notes: "- İmzalı güncelleme".to_string(),
            download_url: download_url.to_string(),
            sha256: Some(sha256.clone()),
            portable_url: None,
            portable_sha256: None,
            signature: Some(sig_b64.clone()),
            key_id: Some("meetin-release-1".to_string()),
        };

        // 1. Geçerli imza -> verified + can_auto_install = true
        let ok_res = evaluate_manifest_check(&manifest, "1.2.0", false, Some(&pub_pem), false);
        assert_eq!(ok_res.signature_status, SIG_STATUS_VERIFIED);
        assert!(ok_res.can_auto_install);

        // 2. Değiştirilmiş sürüm -> invalid + can_auto_install = false
        let mut bad_ver = manifest.clone();
        bad_ver.version = "1.3.1".to_string();
        let bad_ver_res = evaluate_manifest_check(&bad_ver, "1.2.0", false, Some(&pub_pem), false);
        assert_eq!(bad_ver_res.signature_status, SIG_STATUS_INVALID);
        assert!(!bad_ver_res.can_auto_install);

        // 3. Değiştirilmiş SHA-256 -> invalid
        let mut bad_sha = manifest.clone();
        bad_sha.sha256 = Some(digest_to_hex(&sha256_digest(b"different-binary")));
        let bad_sha_res = evaluate_manifest_check(&bad_sha, "1.2.0", false, Some(&pub_pem), false);
        assert_eq!(bad_sha_res.signature_status, SIG_STATUS_INVALID);
        assert!(!bad_sha_res.can_auto_install);

        // 4. Değiştirilmiş indirme adresi -> invalid
        let mut bad_url = manifest.clone();
        bad_url.download_url = "https://evil.example/Meetin_1.3.0.exe".to_string();
        let bad_url_res = evaluate_manifest_check(&bad_url, "1.2.0", false, Some(&pub_pem), false);
        assert_eq!(bad_url_res.signature_status, SIG_STATUS_INVALID);
        assert!(!bad_url_res.can_auto_install);

        // 5. Sahte/bozuk imza baytı -> invalid
        let mut forged_sig = sig_bytes;
        forged_sig[0] ^= 0x01;
        let mut bad_sig = manifest.clone();
        bad_sig.signature = Some(base64_encode(&forged_sig));
        let bad_sig_res = evaluate_manifest_check(&bad_sig, "1.2.0", false, Some(&pub_pem), false);
        assert_eq!(bad_sig_res.signature_status, SIG_STATUS_INVALID);
        assert!(!bad_sig_res.can_auto_install);

        // 6. Farklı anahtar çiftinin açık anahtarı -> invalid
        let (other_pub_blob, _) = crypto::test_generate_keypair_and_sign(b"other").unwrap();
        let other_pub_b64 = base64_encode(&other_pub_blob);
        let wrong_key_res =
            evaluate_manifest_check(&manifest, "1.2.0", false, Some(&other_pub_b64), false);
        assert_eq!(wrong_key_res.signature_status, SIG_STATUS_INVALID);
        assert!(!wrong_key_res.can_auto_install);
    }

    #[test]
    fn parses_latest_json_v2_unsigned_and_enforces_https() {
        let sha = digest_to_hex(&sha256_digest(b"nsis-binary"));
        let p_sha = digest_to_hex(&sha256_digest(b"portable-zip"));
        let json = format!(
            r#"{{
                "version": "1.3.0",
                "pub_date": "2026-11-28T10:00:00Z",
                "notes": "- Sessiz güncelleme eklendi",
                "download_url": "https://example.com/Meetin_1.3.0_x64-setup.exe",
                "sha256": "{sha}",
                "portable_url": "https://example.com/Meetin_1.3.0_portable.zip",
                "portable_sha256": "{p_sha}"
            }}"#
        );

        let nsis_res = parse_latest_json_with_key(&json, "1.2.0", false, None, false).unwrap();
        assert!(nsis_res.update_available);
        assert_eq!(nsis_res.latest_version.as_deref(), Some("1.3.0"));
        assert_eq!(nsis_res.signature_status, SIG_STATUS_UNSIGNED);
        assert!(nsis_res.can_auto_install);
        assert!(nsis_res.download_url.ends_with("setup.exe"));

        let portable_res = parse_latest_json_with_key(&json, "1.2.0", true, None, false).unwrap();
        assert!(portable_res.update_available);
        assert_eq!(portable_res.signature_status, SIG_STATUS_UNSIGNED);
        assert!(portable_res.can_auto_install);
        assert!(portable_res.download_url.ends_with("portable.zip"));
        assert_eq!(portable_res.sha256.as_deref(), Some(p_sha.as_str()));

        // HTTP (güvensiz) indirme adresi otomatik kuruluma izin vermez
        let insecure_json = format!(
            r#"{{
                "version": "1.3.0",
                "download_url": "http://example.com/Meetin_1.3.0_x64-setup.exe",
                "sha256": "{sha}"
            }}"#
        );
        let insecure =
            parse_latest_json_with_key(&insecure_json, "1.2.0", false, None, false).unwrap();
        assert!(insecure.update_available);
        assert!(!insecure.can_auto_install);
    }

    #[test]
    fn builds_nsis_silent_args_and_restart_script_safely() {
        let default_args = build_nsis_silent_args(None);
        assert_eq!(default_args, vec!["/S".to_string()]);

        let custom_args = build_nsis_silent_args(Some(Path::new(r"C:\Program Files\Meetin")));
        assert_eq!(
            custom_args,
            vec!["/S".to_string(), r"/D=C:\Program Files\Meetin".to_string()]
        );

        let script = build_nsis_restart_script(
            4242,
            Path::new(r"C:\Temp\Meetin's\Meetin_1.3.0_x64-setup.exe"),
            Path::new(r"C:\Users\Test\AppData\Local\Meetin\meetin.exe"),
            None,
        );
        assert!(script.contains("4242"));
        assert!(script.contains("'/S'"));
        assert!(script.contains("Meetin''s"));
        assert!(script.contains("Start-Process -FilePath $installer -ArgumentList $nsisArgs -Wait"));
        assert!(script.contains("Start-Process -FilePath $relaunch"));
    }

    #[test]
    fn portable_self_update_swaps_exe_and_preserves_data_dir() {
        let dir = TempDir::new().unwrap();
        let current_exe = dir.path().join("Meetin.exe");
        let new_exe = dir.path().join("Meetin_new.exe");
        let data_dir = dir.path().join("data");
        fs::create_dir_all(&data_dir).unwrap();
        let db_marker = data_dir.join("meetin.db");
        fs::write(&db_marker, b"db-data").unwrap();
        fs::write(&current_exe, b"old-bin").unwrap();
        fs::write(&new_exe, b"new-bin").unwrap();

        let old_backup = prepare_portable_self_update(&current_exe, &new_exe).unwrap();
        assert_eq!(fs::read(&old_backup).unwrap(), b"old-bin");
        assert_eq!(fs::read(&current_exe).unwrap(), b"new-bin");
        assert_eq!(fs::read(&db_marker).unwrap(), b"db-data");
    }

    #[cfg(windows)]
    #[test]
    fn staged_verified_portable_upgrade_rejects_corrupted_and_succeeds_when_valid() {
        let dir = TempDir::new().unwrap();
        let source_exe = dir.path().join("Meetin_1.3.0.exe");
        let payload_bytes = b"verified-portable-meetin-1.3.0-executable";
        fs::write(&source_exe, payload_bytes).unwrap();
        let sha256 = digest_to_hex(&sha256_digest(payload_bytes));
        let source_str = source_exe.to_string_lossy().into_owned();

        let canonical = canonical_signing_payload("1.3.0", &sha256, &source_str);
        let (pub_blob, sig_bytes) =
            crypto::test_generate_keypair_and_sign(canonical.as_bytes()).unwrap();
        let pub_b64 = base64_encode(&pub_blob);
        let sig_b64 = base64_encode(&sig_bytes);

        let manifest = UpdateManifest {
            version: "1.3.0".to_string(),
            pub_date: Some("2026-11-28T10:00:00Z".to_string()),
            notes: "Portable test".to_string(),
            download_url: source_str.clone(),
            sha256: Some(sha256.clone()),
            portable_url: Some(source_str),
            portable_sha256: Some(sha256),
            signature: Some(sig_b64),
            key_id: Some("test-key".to_string()),
        };

        let staging = dir.path().join("staging");
        let staged =
            stage_and_verify_update(&manifest, true, &staging, Some(&pub_b64), true).unwrap();
        assert_eq!(staged.signature_status, SIG_STATUS_VERIFIED);
        assert!(staged.artifact_path.is_file());

        // Portable dizinde Meetin.exe'yi yükselt ve data klasörünü koru
        let app_dir = dir.path().join("portable-app");
        let data_dir = app_dir.join("data");
        fs::create_dir_all(&data_dir).unwrap();
        fs::write(data_dir.join("meetin.db"), b"user-phrases-sqlite").unwrap();
        let running_exe = app_dir.join("Meetin.exe");
        fs::write(&running_exe, b"old-1.2.0").unwrap();

        let old_backup = prepare_portable_self_update(&running_exe, &staged.artifact_path).unwrap();
        assert_eq!(fs::read(&old_backup).unwrap(), b"old-1.2.0");
        assert_eq!(fs::read(&running_exe).unwrap(), payload_bytes);
        assert_eq!(
            fs::read(data_dir.join("meetin.db")).unwrap(),
            b"user-phrases-sqlite"
        );

        // Paket bozulursa stage_and_verify_update reddedip geçici dosyayı silmeli
        fs::write(&source_exe, b"tampered-binary").unwrap();
        let staging_bad = dir.path().join("staging-bad");
        let err = stage_and_verify_update(&manifest, true, &staging_bad, Some(&pub_b64), true)
            .unwrap_err();
        assert!(err.contains("SHA-256"), "unexpected err: {err}");
        assert!(!staging_bad.join("Meetin_1.3.0.exe").exists());
    }

    #[test]
    fn decode_manifest_bytes_strips_bom_and_parses_turkish_json() {
        let json = "\u{FEFF}{\"version\":\"1.3.0\",\"notes\":\"Türkçe sürüm notu: şğıöçü\",\"download_url\":\"https://example.com/setup.exe\",\"sha256\":\"\"}";
        let decoded = decode_manifest_bytes(json.as_bytes());
        assert!(decoded.starts_with('{'));
        let parsed = parse_manifest_json(&decoded).unwrap();
        assert_eq!(parsed.version, "1.3.0");
        assert!(parsed.notes.contains("şğıöçü"));
    }
}
