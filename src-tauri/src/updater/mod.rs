//! Güncelleme (§9): denetim ve kurulum `tauri-plugin-updater` ile yapılır.
//!
//! - Bildirim GitHub'daki en son yayının `latest.json` dosyasıdır (`tauri.conf.json` › `plugins.updater`).
//! - Kurulum paketi minisign ile imzalıdır; açık anahtar uygulamanın içindedir. İmzası tutmayan ya da
//!   imzasız paket eklenti tarafından reddedilir, kurulmaz.
//! - Portable sürüm (R2.4, Saul kararı: bildir, onayla, kendisi kursun): imzalı ZIP eklentiyle
//!   indirilip doğrulanır, içindeki `Meetin.exe` çıkarılır; çalışan EXE `Meetin.old.exe` adını
//!   alır, yenisi yerine konur, yeni EXE başlatılır ve eskisi kapanır. `data\` klasörüne dokunulmaz.
//!   Araya betik ya da yardımcı süreç girmez. Klasör yazılamıyorsa yalnız indirme bağlantısı verilir.
//!
//! Bu modül eklentinin sonucunu arayüz için biçimler ve portable EXE değişimini yapar.

use serde::Serialize;

/// GitHub'daki en son yayın sayfası (portable için yedek indirme adresi).
pub const RELEASES_PAGE: &str = "https://github.com/ismetuluer/meetin/releases/latest";
/// Portable ZIP adresi yalnız bu depodan kabul edilir.
const TRUSTED_DOWNLOAD_PREFIX: &str = "https://github.com/ismetuluer/meetin/releases/download/";

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct UpdateCheckResult {
    pub current_version: String,
    /// Denetim yapılmadıysa `None`; yapıldıysa bilinen en son sürüm.
    pub latest_version: Option<String>,
    pub update_available: bool,
    pub notes: String,
    /// Elle indirme adresi (portable ZIP ya da yayın sayfası).
    pub download_url: String,
    pub is_portable: bool,
    pub webview2_installed: bool,
    /// Tek tıkla kurulabilir mi (kurulu sürüm; ya da imzalı ZIP'i olan, klasörü yazılabilir portable).
    pub can_auto_install: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateInstallResult {
    pub version: String,
}

/// Eklentinin bulduğu güncelleme (yalnız arayüzün ihtiyacı olan alanlar).
pub struct FoundUpdate<'a> {
    pub version: &'a str,
    pub notes: Option<&'a str>,
    pub raw_json: &'a serde_json::Value,
}

/// Henüz denetim yapılmamış durum.
pub fn current_env_info(is_portable: bool, webview2_installed: bool) -> UpdateCheckResult {
    UpdateCheckResult {
        current_version: env!("CARGO_PKG_VERSION").to_string(),
        latest_version: None,
        update_available: false,
        notes: String::new(),
        download_url: String::new(),
        is_portable,
        webview2_installed,
        can_auto_install: false,
    }
}

/// Bildirimdeki portable ZIP adresi; yalnız bu deponun yayınlarından gelen `https` adres kabul edilir.
fn trusted_portable_url(raw_json: &serde_json::Value) -> Option<String> {
    let url = raw_json.get("portable_url")?.as_str()?.trim();
    (url.starts_with(TRUSTED_DOWNLOAD_PREFIX) && !url.contains("..")).then(|| url.to_string())
}

/// Portable güncelleme paketi: güvenilir adres ve imza (`portable_signature`) birlikte gerekir.
pub fn portable_package(raw_json: &serde_json::Value) -> Option<(String, String)> {
    let url = trusted_portable_url(raw_json)?;
    let sig = raw_json.get("portable_signature")?.as_str()?.trim();
    (!sig.is_empty()).then(|| (url, sig.to_string()))
}

/// Denetim sonucunu arayüz için biçimler. `found` yoksa uygulama günceldir.
pub fn result_for_check(
    current_version: &str,
    found: Option<FoundUpdate<'_>>,
    is_portable: bool,
    portable_dir_writable: bool,
    webview2_installed: bool,
) -> UpdateCheckResult {
    let Some(found) = found else {
        return UpdateCheckResult {
            current_version: current_version.to_string(),
            latest_version: Some(current_version.to_string()),
            ..current_env_info(is_portable, webview2_installed)
        };
    };
    let download_url = if is_portable {
        trusted_portable_url(found.raw_json).unwrap_or_else(|| RELEASES_PAGE.to_string())
    } else {
        RELEASES_PAGE.to_string()
    };
    UpdateCheckResult {
        current_version: current_version.to_string(),
        latest_version: Some(found.version.to_string()),
        update_available: true,
        notes: found.notes.unwrap_or_default().trim().to_string(),
        download_url,
        is_portable,
        webview2_installed,
        can_auto_install: if is_portable {
            portable_dir_writable && portable_package(found.raw_json).is_some()
        } else {
            true
        },
    }
}

/// Güncelleme sonrası yeni EXE'ye verilen argüman: `--meetin-after-update=<eski süreç kimliği>`.
pub const AFTER_UPDATE_ARG: &str = "--meetin-after-update=";
const MAX_EXE_BYTES: u64 = 256 * 1024 * 1024;

/// Komut satırından güncelleme sonrası argümanını okur.
pub fn after_update_pid<I: IntoIterator<Item = String>>(args: I) -> Option<u32> {
    args.into_iter().find_map(|a| {
        a.strip_prefix(AFTER_UPDATE_ARG)
            .and_then(|p| p.parse().ok())
    })
}

/// Portable ZIP'ten `Meetin.exe`'yi çıkarır (klasör içinde de olabilir; büyük/küçük harf önemsiz).
pub fn extract_exe_from_zip(bytes: &[u8]) -> Result<Vec<u8>, String> {
    use std::io::Read;
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes))
        .map_err(|e| format!("Güncelleme paketi açılamadı: {e}"))?;
    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| format!("Güncelleme paketi okunamadı: {e}"))?;
        let name = file
            .name()
            .map_err(|e| format!("Güncelleme paketi okunamadı: {e}"))?
            .replace('\\', "/");
        let base = name.rsplit('/').next().unwrap_or("");
        if !file.is_file() || !base.eq_ignore_ascii_case("Meetin.exe") {
            continue;
        }
        if file.size() > MAX_EXE_BYTES {
            return Err("Güncelleme paketindeki Meetin.exe beklenenden büyük.".to_string());
        }
        let mut out = Vec::with_capacity(file.size() as usize);
        file.read_to_end(&mut out)
            .map_err(|e| format!("Meetin.exe çıkarılamadı: {e}"))?;
        if !out.starts_with(b"MZ") {
            return Err("Güncelleme paketindeki Meetin.exe geçerli bir program değil.".to_string());
        }
        return Ok(out);
    }
    Err("Güncelleme paketinde Meetin.exe bulunamadı.".to_string())
}

fn old_exe_path(exe: &std::path::Path) -> std::path::PathBuf {
    exe.with_file_name("Meetin.old.exe")
}

/// Çalışan EXE'yi yenisiyle değiştirir: yeni dosya yanına yazılır, çalışan EXE `Meetin.old.exe`
/// olur (Windows çalışan programın adının değiştirilmesine izin verir), yenisi yerine taşınır.
/// Herhangi bir adım başarısız olursa eski EXE yerine geri konur.
pub fn swap_portable_exe(exe: &std::path::Path, new_bytes: &[u8]) -> Result<(), String> {
    let staged = exe.with_file_name("Meetin.new.exe");
    let old = old_exe_path(exe);
    std::fs::write(&staged, new_bytes).map_err(|e| format!("Yeni sürüm yazılamadı: {e}"))?;
    if old.exists() {
        if let Err(e) = std::fs::remove_file(&old) {
            let _ = std::fs::remove_file(&staged);
            return Err(format!("Önceki yedek EXE silinemedi: {e}"));
        }
    }
    if let Err(e) = std::fs::rename(exe, &old) {
        let _ = std::fs::remove_file(&staged);
        return Err(format!("Çalışan sürüm kenara alınamadı: {e}"));
    }
    if let Err(e) = std::fs::rename(&staged, exe) {
        let _ = std::fs::rename(&old, exe);
        let _ = std::fs::remove_file(&staged);
        return Err(format!("Yeni sürüm yerine konamadı: {e}"));
    }
    Ok(())
}

/// Önceki güncellemeden kalan `Meetin.old.exe`'yi siler (yoksa bir şey yapmaz).
pub fn remove_old_exe(exe: &std::path::Path) -> std::io::Result<bool> {
    let old = old_exe_path(exe);
    if !old.exists() {
        return Ok(false);
    }
    std::fs::remove_file(old).map(|_| true)
}

/// Eski süreç kapanana kadar (en çok `timeout_ms`) bekler.
#[cfg(windows)]
pub fn wait_for_process_exit(pid: u32, timeout_ms: u32) {
    use ::windows::Win32::Foundation::CloseHandle;
    use ::windows::Win32::System::Threading::{
        OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE,
    };
    unsafe {
        if let Ok(h) = OpenProcess(PROCESS_SYNCHRONIZE, false, pid) {
            let _ = WaitForSingleObject(h, timeout_ms);
            let _ = CloseHandle(h);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn guncelse_son_surum_mevcut_surumdur() {
        let r = result_for_check("1.4.2", None, false, false, true);
        assert!(!r.update_available);
        assert_eq!(r.latest_version.as_deref(), Some("1.4.2"));
        assert!(!r.can_auto_install);
    }

    #[test]
    fn kurulu_surum_tek_tikla_kurar() {
        let raw = json!({});
        let r = result_for_check(
            "1.4.2",
            Some(FoundUpdate {
                version: "1.4.3",
                notes: Some("  ### Düzeltildi\n- x \n"),
                raw_json: &raw,
            }),
            false,
            false,
            true,
        );
        assert!(r.update_available && r.can_auto_install);
        assert_eq!(r.latest_version.as_deref(), Some("1.4.3"));
        assert_eq!(r.notes, "### Düzeltildi\n- x");
    }

    fn found<'a>(raw: &'a serde_json::Value) -> Option<FoundUpdate<'a>> {
        Some(FoundUpdate {
            version: "1.4.3",
            notes: None,
            raw_json: raw,
        })
    }

    const ZIP_URL: &str =
        "https://github.com/ismetuluer/meetin/releases/download/v1.4.3/Meetin_1.4.3_portable.zip";

    #[test]
    fn portable_imzali_zip_ve_yazilabilir_klasorle_kendisi_kurar() {
        let raw = json!({ "portable_url": ZIP_URL, "portable_signature": "c2ln" });
        let r = result_for_check("1.4.2", found(&raw), true, true, true);
        assert!(r.update_available && r.can_auto_install);
        assert_eq!(r.download_url, ZIP_URL);
    }

    #[test]
    fn portable_imza_yoksa_ya_da_klasor_yazilamazsa_yalniz_baglanti() {
        let unsigned = json!({ "portable_url": ZIP_URL });
        let r = result_for_check("1.4.2", found(&unsigned), true, true, true);
        assert!(r.update_available && !r.can_auto_install);
        assert_eq!(r.download_url, ZIP_URL);

        let signed = json!({ "portable_url": ZIP_URL, "portable_signature": "c2ln" });
        let r = result_for_check("1.4.2", found(&signed), true, false, true);
        assert!(!r.can_auto_install);
    }

    fn make_zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
        use std::io::Write;
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut w = zip::ZipWriter::new(&mut buf);
            let opts = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);
            for (name, data) in entries {
                w.start_file(*name, opts).unwrap();
                w.write_all(data).unwrap();
            }
            w.finish().unwrap();
        }
        buf.into_inner()
    }

    #[test]
    fn zipten_meetin_exe_cikarilir() {
        let z = make_zip(&[
            ("Meetin/portable.flag", b"x"),
            ("Meetin/Meetin.exe", b"MZ-yeni"),
        ]);
        assert_eq!(extract_exe_from_zip(&z).unwrap(), b"MZ-yeni");
    }

    #[test]
    fn zipte_exe_yoksa_ya_da_gecersizse_hata() {
        let z = make_zip(&[("Meetin/portable.flag", b"x")]);
        assert!(extract_exe_from_zip(&z).is_err());
        let z = make_zip(&[("Meetin/Meetin.exe", b"not-an-exe")]);
        assert!(extract_exe_from_zip(&z).is_err());
        assert!(extract_exe_from_zip(b"zip degil").is_err());
    }

    #[test]
    fn exe_degisimi_eskiyi_kenara_alir_yeniyi_koyar() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("Meetin.exe");
        std::fs::write(&exe, b"MZ-eski").unwrap();
        std::fs::write(dir.path().join("Meetin.old.exe"), b"MZ-daha-eski").unwrap();
        swap_portable_exe(&exe, b"MZ-yeni").unwrap();
        assert_eq!(std::fs::read(&exe).unwrap(), b"MZ-yeni");
        assert_eq!(
            std::fs::read(dir.path().join("Meetin.old.exe")).unwrap(),
            b"MZ-eski"
        );
        assert!(!dir.path().join("Meetin.new.exe").exists());
        assert!(remove_old_exe(&exe).unwrap());
        assert!(!remove_old_exe(&exe).unwrap());
    }

    #[test]
    fn exe_degisimi_basarisizsa_eski_yerinde_kalir() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("Meetin.exe");
        // EXE hiç yok: kenara alma adımı başarısız olur, ortalıkta yarım dosya kalmaz
        assert!(swap_portable_exe(&exe, b"MZ-yeni").is_err());
        assert!(!dir.path().join("Meetin.new.exe").exists());
        assert!(!exe.exists());
    }

    #[test]
    fn guncelleme_sonrasi_argumani_okunur() {
        let args = vec!["meetin.exe".to_string(), format!("{AFTER_UPDATE_ARG}4242")];
        assert_eq!(after_update_pid(args), Some(4242));
        assert_eq!(after_update_pid(vec!["meetin.exe".to_string()]), None);
        assert_eq!(
            after_update_pid(vec![format!("{AFTER_UPDATE_ARG}abc")]),
            None
        );
    }

    /// CI'ın ürettiği köprü bildirimi (eklenti alanları + 1.4.1'in okuduğu alanlar) eklentiyle
    /// okunabilmeli; imza `platforms` içinde olmalı.
    #[test]
    fn kopru_bildirimi_eklentiyle_okunur() {
        let raw = json!({
            "version": "1.4.2",
            "notes": "### Düzeltildi\n- x",
            "pub_date": "2026-11-27T12:00:00Z",
            "platforms": {
                "windows-x86_64": {
                    "signature": "dW50cnVzdGVkIGNvbW1lbnQ=",
                    "url": "https://github.com/ismetuluer/meetin/releases/download/v1.4.2/Meetin_1.4.2_x64-setup.exe"
                }
            },
            "download_url": "https://github.com/ismetuluer/meetin/releases/download/v1.4.2/Meetin_1.4.2_x64-setup.exe",
            "sha256": "00",
            "portable_url": "https://github.com/ismetuluer/meetin/releases/download/v1.4.2/Meetin_1.4.2_portable.zip",
            "portable_sha256": "00"
        });
        let rel: tauri_plugin_updater::RemoteRelease = serde_json::from_value(raw).unwrap();
        assert_eq!(rel.version.to_string(), "1.4.2");
        assert!(rel
            .download_url("windows-x86_64")
            .unwrap()
            .as_str()
            .ends_with("Meetin_1.4.2_x64-setup.exe"));
        assert_eq!(
            rel.signature("windows-x86_64").unwrap(),
            "dW50cnVzdGVkIGNvbW1lbnQ="
        );
    }

    /// `tauri.conf.json` › `plugins.updater` eklentinin beklediği biçimde olmalı; aksi halde
    /// uygulama açılışta eklentiyi kuramaz.
    #[test]
    fn eklenti_yapilandirmasi_gecerli() {
        let conf: serde_json::Value =
            serde_json::from_str(include_str!("../../tauri.conf.json")).unwrap();
        let section = conf["plugins"]["updater"].clone();
        let cfg: tauri_plugin_updater::Config = serde_json::from_value(section.clone()).unwrap();
        assert_eq!(cfg.endpoints.len(), 1);
        assert_eq!(
            cfg.endpoints[0].as_str(),
            "https://github.com/ismetuluer/meetin/releases/latest/download/latest.json"
        );
        // Açık anahtar: base64 içinde minisign açık anahtar metni
        let pubkey = section["pubkey"].as_str().unwrap();
        let decoded = decode_base64(pubkey);
        assert!(decoded.starts_with("untrusted comment: minisign public key"));
    }

    fn decode_base64(input: &str) -> String {
        const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut bits = 0u32;
        let mut n = 0;
        let mut out = Vec::new();
        for c in input.bytes().filter(|c| *c != b'=') {
            let v = TABLE.iter().position(|t| *t == c).unwrap() as u32;
            bits = (bits << 6) | v;
            n += 6;
            if n >= 8 {
                n -= 8;
                out.push((bits >> n) as u8);
                bits &= (1 << n) - 1;
            }
        }
        String::from_utf8(out).unwrap()
    }

    #[test]
    fn yabanci_portable_adresi_kabul_edilmez() {
        for bad in [
            "https://evil.example/Meetin.zip",
            "http://github.com/ismetuluer/meetin/releases/download/v1/x.zip",
            "https://github.com/ismetuluer/meetin/releases/download/../../x.zip",
        ] {
            let raw = json!({ "portable_url": bad });
            let r = result_for_check(
                "1.4.2",
                Some(FoundUpdate {
                    version: "1.4.3",
                    notes: None,
                    raw_json: &raw,
                }),
                true,
                true,
                true,
            );
            assert_eq!(r.download_url, RELEASES_PAGE, "{bad}");
            assert!(!r.can_auto_install);
        }
    }
}
