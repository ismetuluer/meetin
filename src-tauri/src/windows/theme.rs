//! Windows açık/koyu mod ve vurgu rengi okuyucusu (§11.1, M6.2).

use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

use crate::windows::backdrop;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemThemeInfo {
    pub dark: bool,
    pub accent_hex: String,
}

impl Default for SystemThemeInfo {
    fn default() -> Self {
        Self {
            dark: false,
            accent_hex: "#0078D4".to_string(),
        }
    }
}

#[cfg(windows)]
fn read_hkcu_dword(subkey: &str, value_name: &str) -> Option<u32> {
    use std::ffi::c_void;
    use std::mem::size_of;
    use windows::core::PCWSTR;
    use windows::Win32::System::Registry::{
        RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD,
    };

    let subkey_w: Vec<u16> = subkey.encode_utf16().chain(std::iter::once(0)).collect();
    let value_w: Vec<u16> = value_name
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let mut data: u32 = 0;
    let mut size: u32 = size_of::<u32>() as u32;

    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey_w.as_ptr()),
            PCWSTR(value_w.as_ptr()),
            RRF_RT_REG_DWORD,
            None,
            Some(&mut data as *mut u32 as *mut c_void),
            Some(&mut size),
        )
    };
    if status.is_ok() {
        Some(data)
    } else {
        None
    }
}

/// Windows açık/koyu modunu ve vurgu rengini okur (§11.1).
pub fn read_system_theme() -> SystemThemeInfo {
    #[cfg(windows)]
    {
        let apps_light = read_hkcu_dword(
            r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize",
            "AppsUseLightTheme",
        )
        .unwrap_or(1);
        let dark = apps_light == 0;

        let accent_hex = read_hkcu_dword(r"Software\Microsoft\Windows\DWM", "AccentColor")
            .map(|abgr| {
                let r = abgr & 0xFF;
                let g = (abgr >> 8) & 0xFF;
                let b = (abgr >> 16) & 0xFF;
                format!("#{r:02X}{g:02X}{b:02X}")
            })
            .unwrap_or_else(|| "#0078D4".to_string());

        SystemThemeInfo { dark, accent_hex }
    }
    #[cfg(not(windows))]
    {
        SystemThemeInfo::default()
    }
}

/// Windows tema veya vurgu rengi değiştiğinde tüm pencerelere `system-theme-changed` yayar.
pub fn start_theme_watcher(app: AppHandle) {
    thread::Builder::new()
        .name("meetin-theme-watcher".to_string())
        .spawn(move || {
            let mut last = read_system_theme();
            loop {
                thread::sleep(Duration::from_secs(2));
                let current = read_system_theme();
                if current != last {
                    last = current.clone();
                    backdrop::sync_app_windows(&app, current.dark);
                    let _ = app.emit("system-theme-changed", current);
                }
            }
        })
        .ok();
}
