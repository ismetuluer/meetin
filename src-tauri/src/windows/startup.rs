//! Windows ile otomatik başlatma ve UAC'siz yönetici görevi yönetimi (M6.8).

use serde::{Deserialize, Serialize};

use crate::db::runtime_data_dir;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartupStatus {
    pub run_at_login: bool,
    pub admin_task_enabled: bool,
    pub is_elevated: bool,
    pub is_portable: bool,
}

pub fn format_quoted_exe_path(exe: &std::path::Path) -> String {
    format!("\"{}\"", exe.to_string_lossy())
}

pub fn is_portable_mode() -> bool {
    crate::db::has_portable_flag(&crate::db::exe_dir()) || runtime_data_dir().ends_with("data")
}

#[cfg(windows)]
mod win_startup {
    use std::ffi::c_void;
    use std::mem::size_of;
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::Security::{
        GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY,
    };
    use windows::Win32::System::Registry::{
        RegCloseKey, RegDeleteValueW, RegGetValueW, RegOpenKeyExW, RegSetValueExW, HKEY,
        HKEY_CURRENT_USER, KEY_SET_VALUE, REG_SZ, RRF_RT_REG_SZ,
    };
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    const RUN_SUBKEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
    const RUN_VALUE: &str = "Meetin";
    const ADMIN_TASK_NAME: &str = "MeetinAdmin";
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    pub fn is_run_at_login_enabled() -> bool {
        let sub_w = wide(RUN_SUBKEY);
        let val_w = wide(RUN_VALUE);
        let mut size: u32 = 0;
        let res = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                PCWSTR(sub_w.as_ptr()),
                PCWSTR(val_w.as_ptr()),
                RRF_RT_REG_SZ,
                None,
                None,
                Some(&mut size),
            )
        };
        res.is_ok() && size > 0
    }

    pub fn set_run_at_login(enabled: bool) -> Result<(), String> {
        let sub_w = wide(RUN_SUBKEY);
        let val_w = wide(RUN_VALUE);
        let mut hkey = HKEY::default();
        let open = unsafe {
            RegOpenKeyExW(
                HKEY_CURRENT_USER,
                PCWSTR(sub_w.as_ptr()),
                0,
                KEY_SET_VALUE,
                &mut hkey,
            )
        };
        if open.is_err() {
            return Err(format!("Kayıt defteri açılamadı: {:?}", open));
        }

        let res = if enabled {
            let exe = std::env::current_exe().map_err(|e| e.to_string())?;
            let quoted = super::format_quoted_exe_path(&exe);
            let data_w = wide(&quoted);
            let bytes = unsafe {
                std::slice::from_raw_parts(
                    data_w.as_ptr() as *const u8,
                    data_w.len() * size_of::<u16>(),
                )
            };
            unsafe { RegSetValueExW(hkey, PCWSTR(val_w.as_ptr()), 0, REG_SZ, Some(bytes)) }
        } else {
            unsafe { RegDeleteValueW(hkey, PCWSTR(val_w.as_ptr())) }
        };
        unsafe {
            let _ = RegCloseKey(hkey);
        }
        if enabled && res.is_err() {
            return Err(format!("Başlangıç kaydı yazılamadı: {:?}", res));
        }
        Ok(())
    }

    pub fn is_admin_task_enabled() -> bool {
        Command::new("schtasks")
            .args(["/Query", "/TN", ADMIN_TASK_NAME])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    pub fn set_admin_autostart(enabled: bool) -> Result<(), String> {
        let out = if enabled {
            let exe = std::env::current_exe().map_err(|e| e.to_string())?;
            let quoted = super::format_quoted_exe_path(&exe);
            Command::new("schtasks")
                .args([
                    "/Create",
                    "/TN",
                    ADMIN_TASK_NAME,
                    "/SC",
                    "ONLOGON",
                    "/RL",
                    "HIGHEST",
                    "/TR",
                    &quoted,
                    "/F",
                ])
                .creation_flags(CREATE_NO_WINDOW)
                .output()
                .map_err(|e| e.to_string())?
        } else {
            Command::new("schtasks")
                .args(["/Delete", "/TN", ADMIN_TASK_NAME, "/F"])
                .creation_flags(CREATE_NO_WINDOW)
                .output()
                .map_err(|e| e.to_string())?
        };
        if out.status.success() || !enabled {
            Ok(())
        } else {
            let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
            Err(if err.is_empty() {
                "Yönetici görevi oluşturulamadı (yönetici izni gerekebilir)".to_string()
            } else {
                err
            })
        }
    }

    pub fn is_process_elevated() -> bool {
        let mut token = HANDLE::default();
        let ok = unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) };
        if ok.is_err() {
            return false;
        }
        let mut elev = TOKEN_ELEVATION::default();
        let mut ret_len = 0u32;
        let got = unsafe {
            GetTokenInformation(
                token,
                TokenElevation,
                Some(&mut elev as *mut TOKEN_ELEVATION as *mut c_void),
                size_of::<TOKEN_ELEVATION>() as u32,
                &mut ret_len,
            )
        };
        unsafe {
            let _ = CloseHandle(token);
        }
        got.is_ok() && elev.TokenIsElevated != 0
    }
}

pub fn get_startup_status() -> StartupStatus {
    #[cfg(windows)]
    {
        StartupStatus {
            run_at_login: win_startup::is_run_at_login_enabled(),
            admin_task_enabled: win_startup::is_admin_task_enabled(),
            is_elevated: win_startup::is_process_elevated(),
            is_portable: is_portable_mode(),
        }
    }
    #[cfg(not(windows))]
    {
        StartupStatus {
            run_at_login: false,
            admin_task_enabled: false,
            is_elevated: false,
            is_portable: is_portable_mode(),
        }
    }
}

pub fn set_run_at_login(enabled: bool) -> Result<(), String> {
    #[cfg(windows)]
    {
        win_startup::set_run_at_login(enabled)
    }
    #[cfg(not(windows))]
    {
        let _ = enabled;
        Ok(())
    }
}

pub fn set_admin_autostart(enabled: bool) -> Result<(), String> {
    #[cfg(windows)]
    {
        win_startup::set_admin_autostart(enabled)
    }
    #[cfg(not(windows))]
    {
        let _ = enabled;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn format_quoted_exe_path_wraps_spaces() {
        let quoted = format_quoted_exe_path(Path::new(r"C:\Program Files\Meetin\meetin.exe"));
        assert_eq!(quoted, r#""C:\Program Files\Meetin\meetin.exe""#);
    }
}
