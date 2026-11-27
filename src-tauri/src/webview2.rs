//! WebView2 çalışma zamanı denetimi (R1.5, Saul kararı).
//!
//! Meetin'in arayüzü WebView2 ile çalışır. Windows 10/11'de genelde yüklüdür; yoksa (eski ya da
//! kurumsal bilgisayar) kullanıcıya sorulur ve Microsoft'un resmî "Evergreen" önyükleyicisi
//! indirilip çalıştırılır. Bu, güncelleme denetimi dışındaki tek ağ erişimidir ve yalnız kullanıcı
//! onay verince yapılır. İndirilen dosya çalıştırılmadan önce Authenticode imzası doğrulanır.

use crate::logging::{self, Level};
use crate::native_dialog;

/// Microsoft'un WebView2 Evergreen önyükleyicisinin kalıcı bağlantısı.
const BOOTSTRAPPER_URL: &str = "https://go.microsoft.com/fwlink/p/?LinkId=2124703";

fn is_installed() -> bool {
    tauri::webview_version().is_ok()
}

/// WebView2 yüklüyse ya da kullanıcı onaylayıp kurulum başarılı olursa `true`.
pub fn ensure_installed() -> bool {
    if is_installed() {
        return true;
    }
    logging::event(Level::Warn, "webview2.missing", &[]);
    if !native_dialog::ask(
        "Meetin'in çalışması için Microsoft WebView2 bileşeni gerekiyor ve bu bilgisayarda \
         yüklü değil.\n\nMicrosoft'tan indirilip kurulsun mu?\n(Küçük bir indirme; kurulum \
         birkaç dakika sürebilir.)",
    ) {
        native_dialog::error(
            "WebView2 olmadan Meetin açılamaz. Daha sonra yeniden başlattığınızda tekrar sorulacak.",
        );
        return false;
    }
    match install() {
        Ok(()) if is_installed() => {
            logging::event(Level::Info, "webview2.installed", &[]);
            true
        }
        Ok(()) => {
            logging::event(Level::Error, "webview2.install_incomplete", &[]);
            native_dialog::error(
                "WebView2 kurulumu tamamlanmadı. İnternet bağlantınızı kontrol edip Meetin'i \
                 yeniden başlatın.",
            );
            false
        }
        Err(code) => {
            logging::event(Level::Error, code, &[]);
            native_dialog::error(
                "WebView2 indirilemedi ya da kurulamadı. İnternet bağlantınızı kontrol edip \
                 Meetin'i yeniden başlatın.",
            );
            false
        }
    }
}

#[cfg(windows)]
fn install() -> Result<(), &'static str> {
    use windows::core::{HSTRING, PCWSTR};
    use windows::Win32::System::Com::Urlmon::URLDownloadToFileW;

    let dir = std::env::temp_dir().join(format!("meetin-webview2-{}", std::process::id()));
    std::fs::create_dir_all(&dir).map_err(|_| "webview2.temp_dir_failed")?;
    let exe = dir.join("MicrosoftEdgeWebview2Setup.exe");
    let url = HSTRING::from(BOOTSTRAPPER_URL);
    let target = HSTRING::from(exe.as_os_str());
    let downloaded =
        unsafe { URLDownloadToFileW(None, PCWSTR(url.as_ptr()), PCWSTR(target.as_ptr()), 0, None) };
    let result = (|| {
        downloaded.map_err(|_| "webview2.download_failed")?;
        if !signature_is_valid(&exe) {
            return Err("webview2.signature_invalid");
        }
        let status = std::process::Command::new(&exe)
            .arg("/install")
            .status()
            .map_err(|_| "webview2.run_failed")?;
        if status.success() {
            Ok(())
        } else {
            Err("webview2.installer_failed")
        }
    })();
    let _ = std::fs::remove_dir_all(&dir);
    result
}

#[cfg(not(windows))]
fn install() -> Result<(), &'static str> {
    Err("webview2.unsupported")
}

/// Dosyanın Authenticode imzası geçerli mi (Windows WinVerifyTrust)?
#[cfg(windows)]
fn signature_is_valid(path: &std::path::Path) -> bool {
    use windows::core::{HSTRING, PCWSTR};
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Security::WinTrust::{
        WinVerifyTrust, WINTRUST_ACTION_GENERIC_VERIFY_V2, WINTRUST_DATA, WINTRUST_DATA_0,
        WINTRUST_FILE_INFO, WTD_CHOICE_FILE, WTD_REVOKE_NONE, WTD_STATEACTION_CLOSE,
        WTD_STATEACTION_VERIFY, WTD_UI_NONE,
    };

    let file = HSTRING::from(path.as_os_str());
    let mut file_info = WINTRUST_FILE_INFO {
        cbStruct: std::mem::size_of::<WINTRUST_FILE_INFO>() as u32,
        pcwszFilePath: PCWSTR(file.as_ptr()),
        ..Default::default()
    };
    let mut data = WINTRUST_DATA {
        cbStruct: std::mem::size_of::<WINTRUST_DATA>() as u32,
        dwUIChoice: WTD_UI_NONE,
        fdwRevocationChecks: WTD_REVOKE_NONE,
        dwUnionChoice: WTD_CHOICE_FILE,
        Anonymous: WINTRUST_DATA_0 {
            pFile: &mut file_info,
        },
        dwStateAction: WTD_STATEACTION_VERIFY,
        ..Default::default()
    };
    let mut action = WINTRUST_ACTION_GENERIC_VERIFY_V2;
    let status = unsafe {
        WinVerifyTrust(
            HWND::default(),
            &mut action,
            &mut data as *mut WINTRUST_DATA as *mut core::ffi::c_void,
        )
    };
    data.dwStateAction = WTD_STATEACTION_CLOSE;
    unsafe {
        WinVerifyTrust(
            HWND::default(),
            &mut action,
            &mut data as *mut WINTRUST_DATA as *mut core::ffi::c_void,
        );
    }
    status == 0
}
