//! Pano yedekleme, yapıştırma ve geri yükleme katmanı (§3.4, M1.7).
//!
//! > 200 karakterlik (veya M4'te biçimli) metinler için:
//! 1. Mevcut panonun tüm biçimleri yedeklenir.
//! 2. Metin `CF_UNICODETEXT` olarak, `ExcludeClipboardContentFromMonitorProcessing`
//!    ve `CanIncludeInClipboardHistory = 0` işaretleriyle panoya konur (Win+V geçmişine girmez).
//! 3. `Ctrl+V` gönderilir.
//! 4. Kısa bir gecikmeden sonra pano eski hâline eksiksiz geri yüklenir.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

use super::InjectError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasteShortcut {
    CtrlV,
    CtrlShiftV,
}

/// Arka plan pano izleyicisinin canlı ayarları.
#[derive(Clone)]
pub struct ClipboardWatcherConfig {
    pub enabled: Arc<AtomicBool>,
    pub limit: Arc<AtomicUsize>,
}

impl Default for ClipboardWatcherConfig {
    fn default() -> Self {
        Self {
            enabled: Arc::new(AtomicBool::new(true)),
            limit: Arc::new(AtomicUsize::new(10)),
        }
    }
}

#[cfg(windows)]
mod win_clip {
    use super::super::rich;
    use super::{InjectError, PasteShortcut};
    use crate::hook::INJECTED_EXTRA_INFO;
    use std::ffi::c_void;
    use std::mem::size_of;
    use std::ptr::copy_nonoverlapping;
    use std::thread;
    use std::time::Duration;
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{CloseHandle, HANDLE, HGLOBAL};
    use windows::Win32::System::DataExchange::{
        CloseClipboard, EmptyClipboard, EnumClipboardFormats, GetClipboardData,
        GetClipboardSequenceNumber, IsClipboardFormatAvailable, OpenClipboard,
        RegisterClipboardFormatW, SetClipboardData,
    };
    use windows::Win32::System::Memory::{
        GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock, GMEM_MOVEABLE,
    };
    use windows::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        MapVirtualKeyW, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP,
        MAPVK_VK_TO_VSC, VIRTUAL_KEY, VK_CONTROL, VK_SHIFT,
    };
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

    const CF_UNICODETEXT: u32 = 13;

    pub struct ClipboardBackup {
        formats: Vec<(u32, Vec<u8>)>,
    }

    fn open_clipboard_retry() -> Result<(), InjectError> {
        for _ in 0..10 {
            if unsafe { OpenClipboard(None) }.is_ok() {
                return Ok(());
            }
            thread::sleep(Duration::from_millis(10));
        }
        Err(InjectError::ClipboardBusy)
    }

    unsafe fn alloc_and_copy(data: &[u8]) -> Result<HANDLE, InjectError> {
        let hmem = GlobalAlloc(GMEM_MOVEABLE, data.len())
            .map_err(|e| InjectError::Win32(e.to_string()))?;
        let ptr = GlobalLock(hmem);
        if ptr.is_null() {
            return Err(InjectError::Win32("GlobalLock başarısız".to_string()));
        }
        copy_nonoverlapping(data.as_ptr(), ptr as *mut u8, data.len());
        let _ = GlobalUnlock(hmem);
        Ok(HANDLE(hmem.0))
    }

    pub fn backup_clipboard() -> Result<ClipboardBackup, InjectError> {
        open_clipboard_retry()?;
        let mut formats = Vec::new();
        unsafe {
            let mut fmt = EnumClipboardFormats(0);
            while fmt != 0 {
                if let Ok(handle) = GetClipboardData(fmt) {
                    let hglobal = HGLOBAL(handle.0);
                    let size = GlobalSize(hglobal);
                    if size > 0 {
                        let ptr = GlobalLock(hglobal);
                        if !ptr.is_null() {
                            let slice = std::slice::from_raw_parts(ptr as *const u8, size);
                            formats.push((fmt, slice.to_vec()));
                            let _ = GlobalUnlock(hglobal);
                        }
                    }
                }
                fmt = EnumClipboardFormats(fmt);
            }
            let _ = CloseClipboard();
        }
        Ok(ClipboardBackup { formats })
    }

    pub fn restore_clipboard(backup: &ClipboardBackup) -> Result<(), InjectError> {
        open_clipboard_retry()?;
        unsafe {
            let _ = EmptyClipboard();
            for (fmt, bytes) in &backup.formats {
                if let Ok(handle) = alloc_and_copy(bytes) {
                    let _ = SetClipboardData(*fmt, handle);
                }
            }
            let _ = CloseClipboard();
        }
        Ok(())
    }

    fn register_format(name: &str) -> u32 {
        let wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
        unsafe { RegisterClipboardFormatW(PCWSTR(wide.as_ptr())) }
    }

    pub fn set_transient_rich(plain: &str, html: Option<&str>) -> Result<(), InjectError> {
        open_clipboard_retry()?;
        unsafe {
            let _ = EmptyClipboard();

            // UTF-16LE + null sonlandırıcı
            let wide: Vec<u16> = plain.encode_utf16().chain(std::iter::once(0)).collect();
            let byte_slice =
                std::slice::from_raw_parts(wide.as_ptr() as *const u8, wide.len() * 2);
            let h_text = alloc_and_copy(byte_slice)?;
            let _ = SetClipboardData(CF_UNICODETEXT, h_text);

            // Biçimli içerik varsa HTML Format ve RTF ekle (§3.4)
            if let Some(html_frag) = html {
                let fmt_html = register_format("HTML Format");
                if fmt_html != 0 {
                    let cf_html = rich::build_cf_html(html_frag);
                    if let Ok(h) = alloc_and_copy(&cf_html) {
                        let _ = SetClipboardData(fmt_html, h);
                    }
                }
                let fmt_rtf = register_format("Rich Text Format");
                if fmt_rtf != 0 {
                    let rtf = rich::html_to_rtf(html_frag);
                    if let Ok(h) = alloc_and_copy(&rtf) {
                        let _ = SetClipboardData(fmt_rtf, h);
                    }
                }
            }

            // Windows pano geçmişine (Win+V) ve bulut senkronizasyonuna girmesini engelle
            let zero_dword = 0u32.to_le_bytes();
            let fmt_history = register_format("CanIncludeInClipboardHistory");
            if fmt_history != 0 {
                if let Ok(h) = alloc_and_copy(&zero_dword) {
                    let _ = SetClipboardData(fmt_history, h);
                }
            }
            let fmt_cloud = register_format("CanUploadToCloudClipboard");
            if fmt_cloud != 0 {
                if let Ok(h) = alloc_and_copy(&zero_dword) {
                    let _ = SetClipboardData(fmt_cloud, h);
                }
            }
            let fmt_exclude = register_format("ExcludeClipboardContentFromMonitorProcessing");
            if fmt_exclude != 0 {
                if let Ok(h) = alloc_and_copy(&zero_dword) {
                    let _ = SetClipboardData(fmt_exclude, h);
                }
            }

            let _ = CloseClipboard();
        }
        Ok(())
    }

    pub fn send_paste_keys(shortcut: PasteShortcut) -> Result<(), InjectError> {
        let vk_v = VIRTUAL_KEY(0x56); // 'V'
        let scan_ctrl = unsafe { MapVirtualKeyW(VK_CONTROL.0 as u32, MAPVK_VK_TO_VSC) as u16 };
        let scan_shift = unsafe { MapVirtualKeyW(VK_SHIFT.0 as u32, MAPVK_VK_TO_VSC) as u16 };
        let scan_v = unsafe { MapVirtualKeyW(vk_v.0 as u32, MAPVK_VK_TO_VSC) as u16 };

        let make_key = |vk: VIRTUAL_KEY, scan: u16, flags| INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    wScan: scan,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: INJECTED_EXTRA_INFO,
                },
            },
        };

        let mut inputs = Vec::with_capacity(6);
        inputs.push(make_key(VK_CONTROL, scan_ctrl, Default::default()));
        if shortcut == PasteShortcut::CtrlShiftV {
            inputs.push(make_key(VK_SHIFT, scan_shift, Default::default()));
        }
        inputs.push(make_key(vk_v, scan_v, Default::default()));
        inputs.push(make_key(vk_v, scan_v, KEYEVENTF_KEYUP));
        if shortcut == PasteShortcut::CtrlShiftV {
            inputs.push(make_key(VK_SHIFT, scan_shift, KEYEVENTF_KEYUP));
        }
        inputs.push(make_key(VK_CONTROL, scan_ctrl, KEYEVENTF_KEYUP));

        let sent = unsafe { SendInput(&inputs, size_of::<INPUT>() as i32) };
        if sent as usize != inputs.len() {
            return Err(InjectError::Win32("Yapıştırma tuşu gönderilemedi".to_string()));
        }
        let _ = std::ptr::null::<c_void>();
        Ok(())
    }

    pub fn paste_rich_preserving_clipboard(
        plain: &str,
        html: Option<&str>,
        shortcut: PasteShortcut,
    ) -> Result<(), InjectError> {
        let backup = backup_clipboard()?;
        set_transient_rich(plain, html)?;
        let paste_res = send_paste_keys(shortcut);
        // Hedef uygulamanın panoyu okuması için kısa bekleme
        thread::sleep(Duration::from_millis(95));
        let restore_res = restore_clipboard(&backup);
        paste_res?;
        restore_res
    }

    pub fn read_unicode_text() -> Option<String> {
        open_clipboard_retry().ok()?;
        let mut result = None;
        unsafe {
            if let Ok(handle) = GetClipboardData(CF_UNICODETEXT) {
                let hglobal = HGLOBAL(handle.0);
                let byte_size = GlobalSize(hglobal);
                if byte_size >= 2 {
                    let ptr = GlobalLock(hglobal);
                    if !ptr.is_null() {
                        let u16_slice =
                            std::slice::from_raw_parts(ptr as *const u16, byte_size / 2);
                        let end = u16_slice
                            .iter()
                            .position(|&c| c == 0)
                            .unwrap_or(u16_slice.len());
                        result = Some(String::from_utf16_lossy(&u16_slice[..end]));
                        let _ = GlobalUnlock(hglobal);
                    }
                }
            }
            let _ = CloseClipboard();
        }
        result
    }

    pub fn send_ctrl_c() -> Result<(), InjectError> {
        let vk_c = VIRTUAL_KEY(0x43); // 'C'
        let scan_ctrl = unsafe { MapVirtualKeyW(VK_CONTROL.0 as u32, MAPVK_VK_TO_VSC) as u16 };
        let scan_c = unsafe { MapVirtualKeyW(vk_c.0 as u32, MAPVK_VK_TO_VSC) as u16 };
        let inputs = [
            INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VK_CONTROL,
                        wScan: scan_ctrl,
                        dwFlags: Default::default(),
                        time: 0,
                        dwExtraInfo: INJECTED_EXTRA_INFO,
                    },
                },
            },
            INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: vk_c,
                        wScan: scan_c,
                        dwFlags: Default::default(),
                        time: 0,
                        dwExtraInfo: INJECTED_EXTRA_INFO,
                    },
                },
            },
            INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: vk_c,
                        wScan: scan_c,
                        dwFlags: KEYEVENTF_KEYUP,
                        time: 0,
                        dwExtraInfo: INJECTED_EXTRA_INFO,
                    },
                },
            },
            INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VK_CONTROL,
                        wScan: scan_ctrl,
                        dwFlags: KEYEVENTF_KEYUP,
                        time: 0,
                        dwExtraInfo: INJECTED_EXTRA_INFO,
                    },
                },
            },
        ];

        let sent = unsafe { SendInput(&inputs, size_of::<INPUT>() as i32) };
        if sent as usize != inputs.len() {
            return Err(InjectError::Win32("Ctrl+C gönderilemedi".to_string()));
        }
        Ok(())
    }

    pub fn read_selection_via_ctrl_c() -> Option<String> {
        let backup = backup_clipboard().ok()?;
        if open_clipboard_retry().is_ok() {
            unsafe {
                let _ = EmptyClipboard();
                let _ = CloseClipboard();
            }
        }
        let sent_ok = send_ctrl_c().is_ok();
        if sent_ok {
            thread::sleep(Duration::from_millis(70));
        }
        let selected = if sent_ok { read_unicode_text() } else { None };
        let _ = restore_clipboard(&backup);
        selected
    }

    pub fn is_clipboard_sensitive() -> bool {
        let exclude_w: Vec<u16> = "ExcludeClipboardContentFromMonitorProcessing\0"
            .encode_utf16()
            .collect();
        let can_hist_w: Vec<u16> = "CanIncludeInClipboardHistory\0".encode_utf16().collect();
        let fmt_exclude = unsafe { RegisterClipboardFormatW(PCWSTR(exclude_w.as_ptr())) };
        let fmt_can_hist = unsafe { RegisterClipboardFormatW(PCWSTR(can_hist_w.as_ptr())) };

        if fmt_exclude != 0 && unsafe { IsClipboardFormatAvailable(fmt_exclude) }.is_ok() {
            return true;
        }
        if fmt_can_hist != 0
            && unsafe { IsClipboardFormatAvailable(fmt_can_hist) }.is_ok()
            && open_clipboard_retry().is_ok()
        {
            let mut deny = false;
            if let Ok(handle) = unsafe { GetClipboardData(fmt_can_hist) } {
                let hmem = HGLOBAL(handle.0);
                let ptr = unsafe { GlobalLock(hmem) } as *const u32;
                if !ptr.is_null() {
                    let size = unsafe { GlobalSize(hmem) };
                    if size >= size_of::<u32>() && unsafe { *ptr } == 0 {
                        deny = true;
                    }
                    unsafe {
                        let _ = GlobalUnlock(hmem);
                    }
                }
            }
            unsafe {
                let _ = CloseClipboard();
            }
            if deny {
                return true;
            }
        }
        false
    }

    pub fn foreground_process_name() -> Option<String> {
        let hwnd = unsafe { GetForegroundWindow() };
        if hwnd.0.is_null() {
            return None;
        }
        let mut pid: u32 = 0;
        unsafe {
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
        }
        if pid == 0 {
            return None;
        }
        let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }.ok()?;
        let mut buf = vec![0u16; 260];
        let mut len = buf.len() as u32;
        let ok = unsafe {
            QueryFullProcessImageNameW(
                handle,
                PROCESS_NAME_WIN32,
                windows::core::PWSTR(buf.as_mut_ptr()),
                &mut len,
            )
        }
        .is_ok();
        unsafe {
            let _ = CloseHandle(handle);
        }
        if !ok || len == 0 {
            return None;
        }
        let full = String::from_utf16_lossy(&buf[..len as usize]);
        std::path::Path::new(&full)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
    }

    pub fn write_unicode_text(text: &str) -> Result<(), InjectError> {
        open_clipboard_retry()?;
        unsafe {
            let _ = EmptyClipboard();
            let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
            let byte_slice =
                std::slice::from_raw_parts(wide.as_ptr() as *const u8, wide.len() * 2);
            let h_text = alloc_and_copy(byte_slice)?;
            let _ = SetClipboardData(CF_UNICODETEXT, h_text);
            let _ = CloseClipboard();
        }
        Ok(())
    }

    pub fn sequence_number() -> u32 {
        unsafe { GetClipboardSequenceNumber() }
    }
}

pub fn paste_rich_preserving_clipboard(
    plain: &str,
    html: Option<&str>,
    shortcut: PasteShortcut,
) -> Result<(), InjectError> {
    #[cfg(windows)]
    {
        win_clip::paste_rich_preserving_clipboard(plain, html, shortcut)
    }
    #[cfg(not(windows))]
    {
        let _ = (plain, html, shortcut);
        Ok(())
    }
}

pub fn read_unicode_text() -> Option<String> {
    #[cfg(windows)]
    {
        win_clip::read_unicode_text()
    }
    #[cfg(not(windows))]
    {
        None
    }
}

pub fn read_selection_via_ctrl_c() -> Option<String> {
    #[cfg(windows)]
    {
        win_clip::read_selection_via_ctrl_c()
    }
    #[cfg(not(windows))]
    {
        None
    }
}

pub fn is_clipboard_sensitive() -> bool {
    #[cfg(windows)]
    {
        win_clip::is_clipboard_sensitive()
    }
    #[cfg(not(windows))]
    {
        false
    }
}

pub fn write_unicode_text(text: &str) -> Result<(), InjectError> {
    #[cfg(windows)]
    {
        win_clip::write_unicode_text(text)
    }
    #[cfg(not(windows))]
    {
        let _ = text;
        Ok(())
    }
}

/// Pano geçmişini arka planda izler ve hassas olmayan metin kopyalarını SQLite'a kaydeder (§5.1).
pub fn start_clipboard_watcher(
    storage: std::sync::Arc<crate::db::Storage>,
    cfg: ClipboardWatcherConfig,
) {
    #[cfg(windows)]
    {
        std::thread::Builder::new()
            .name("meetin-clipboard-watcher".to_string())
            .spawn(move || {
                let mut last_seq = win_clip::sequence_number();
                loop {
                    std::thread::sleep(std::time::Duration::from_millis(900));
                    let seq = win_clip::sequence_number();
                    if seq == 0 || seq == last_seq {
                        continue;
                    }
                    last_seq = seq;
                    if !cfg.enabled.load(Ordering::Relaxed) {
                        continue;
                    }
                    if win_clip::is_clipboard_sensitive() {
                        continue;
                    }
                    if let Some(text) = win_clip::read_unicode_text() {
                        let proc_name = win_clip::foreground_process_name();
                        let max_items = cfg.limit.load(Ordering::Relaxed).clamp(1, 200);
                        let _ = storage.push_clipboard_history(
                            &text,
                            None,
                            proc_name.as_deref(),
                            max_items,
                        );
                    }
                }
            })
            .ok();
    }
    #[cfg(not(windows))]
    {
        let _ = (storage, cfg);
    }
}
