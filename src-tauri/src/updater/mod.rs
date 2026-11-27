//! Güncelleme (§9): denetim ve kurulum `tauri-plugin-updater` ile yapılır.
//!
//! - Bildirim GitHub'daki en son yayının `latest.json` dosyasıdır (`tauri.conf.json` › `plugins.updater`).
//! - Kurulum paketi minisign ile imzalıdır; açık anahtar uygulamanın içindedir. İmzası tutmayan ya da
//!   imzasız paket eklenti tarafından reddedilir, kurulmaz.
//! - Portable sürüm kendini kurmaz; yeni ZIP'in adresi gösterilir (R2.4'te karara bağlanacak).
//!
//! Bu modül eklentinin sonucunu arayüz için biçimler; ağ ve dosya işi eklentidedir.

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
    /// Tek tıkla kurulabilir mi (kurulu sürüm + güncelleme var).
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

/// Denetim sonucunu arayüz için biçimler. `found` yoksa uygulama günceldir.
pub fn result_for_check(
    current_version: &str,
    found: Option<FoundUpdate<'_>>,
    is_portable: bool,
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
        can_auto_install: !is_portable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn guncelse_son_surum_mevcut_surumdur() {
        let r = result_for_check("1.4.2", None, false, true);
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
            true,
        );
        assert!(r.update_available && r.can_auto_install);
        assert_eq!(r.latest_version.as_deref(), Some("1.4.3"));
        assert_eq!(r.notes, "### Düzeltildi\n- x");
    }

    #[test]
    fn portable_zip_adresini_gosterir_kurmaz() {
        let raw = json!({
            "portable_url": "https://github.com/ismetuluer/meetin/releases/download/v1.4.3/Meetin_1.4.3_portable.zip"
        });
        let r = result_for_check(
            "1.4.2",
            Some(FoundUpdate {
                version: "1.4.3",
                notes: None,
                raw_json: &raw,
            }),
            true,
            true,
        );
        assert!(r.update_available && !r.can_auto_install);
        assert!(r.download_url.ends_with("Meetin_1.4.3_portable.zip"));
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
            );
            assert_eq!(r.download_url, RELEASES_PAGE, "{bad}");
        }
    }
}
