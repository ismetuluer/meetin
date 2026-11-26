//! Güncelleme denetleyicisi, WebView2 kontrolü ve portable öz-güncelleme (§9, M6.9).

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::windows::startup::is_portable_mode;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateManifest {
    pub version: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub pub_date: Option<String>,
    #[serde(default)]
    pub download_url: String,
    #[serde(default)]
    pub sha256: Option<String>,
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
    }
}

pub fn parse_latest_json(
    raw: &str,
    current_version: &str,
    is_portable: bool,
) -> Result<UpdateCheckResult, String> {
    let manifest: UpdateManifest =
        serde_json::from_str(raw.trim_start_matches('\u{FEFF}'))
            .map_err(|e| format!("latest.json ayrıştırılamadı: {e}"))?;
    let update_available = is_newer_version(current_version, &manifest.version);
    Ok(UpdateCheckResult {
        current_version: current_version.to_string(),
        latest_version: Some(manifest.version),
        update_available,
        notes: manifest.notes,
        download_url: manifest.download_url,
        is_portable,
        webview2_installed: is_webview2_installed(),
    })
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
    let path = Path::new(trimmed);
    if path.is_file() {
        let raw = fs::read_to_string(path).map_err(|e| e.to_string())?;
        return parse_latest_json(&raw, current_version, is_portable);
    }
    if trimmed.starts_with("https://") || trimmed.starts_with("http://") {
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            use std::process::Command;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            let escaped = trimmed.replace('\'', "''");
            let script = format!(
                "$ProgressPreference='SilentlyContinue'; [Net.ServicePointManager]::SecurityProtocol=[Net.SecurityProtocolType]::Tls12; (Invoke-WebRequest -UseBasicParsing -TimeoutSec 10 -Uri '{}').Content",
                escaped
            );
            let out = Command::new("powershell")
                .args(["-NoProfile", "-NonInteractive", "-Command", &script])
                .creation_flags(CREATE_NO_WINDOW)
                .output()
                .map_err(|e| format!("Güncelleme denetimi çalıştırılamadı: {e}"))?;
            if out.status.success() {
                let raw = String::from_utf8_lossy(&out.stdout);
                return parse_latest_json(&raw, current_version, is_portable);
            }
            let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
            return Err(if err.is_empty() {
                "Güncelleme adresinden yanıt alınamadı.".to_string()
            } else {
                format!("Güncelleme denetimi başarısız: {err}")
            });
        }
    }
    Err("Güncelleme bildirim dosyası bulunamadı veya geçerli bir HTTPS adresi değil.".to_string())
}

/// Portable güncellemede çalışan EXE'yi `Meetin.old.exe` olarak kenara alır ve yenisini kopyalar (§9).
/// `data\` klasörüne kesinlikle dokunmaz.
pub fn prepare_portable_self_update(
    current_exe: &Path,
    new_exe: &Path,
) -> Result<PathBuf, String> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn semver_comparison_works() {
        assert!(is_newer_version("0.5.9", "0.5.10"));
        assert!(is_newer_version("0.5.9", "1.0.0"));
        assert!(!is_newer_version("0.5.9", "0.5.9"));
        assert!(!is_newer_version("0.5.9", "0.5.8"));
    }

    #[test]
    fn parses_latest_json_and_detects_newer_version() {
        let json = r#"{
            "version": "0.5.10",
            "notes": "- Öneri adası hızlandırıldı",
            "download_url": "https://example.com/Meetin_0.5.10_x64-setup.exe"
        }"#;
        let res = parse_latest_json(json, "0.5.9", false).unwrap();
        assert!(res.update_available);
        assert_eq!(res.latest_version.as_deref(), Some("0.5.10"));
        assert!(res.notes.contains("Öneri adası"));
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
}
