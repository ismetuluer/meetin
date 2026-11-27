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

    use super::super::clipformats::{
        classify, is_essential_text, FormatKind, RenderWait, CF_ENHMETAFILE, MAX_BACKUP_BYTES,
    };
    use std::sync::{mpsc, Condvar, Mutex, OnceLock};
    use std::time::Instant;
    use windows::core::w;
    use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::Graphics::Gdi::{GetEnhMetaFileBits, SetEnhMetaFileBits, HENHMETAFILE};
    use windows::Win32::System::DataExchange::GetClipboardOwner;
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, RegisterClassW,
        HWND_MESSAGE, MSG, WINDOW_EX_STYLE, WINDOW_STYLE, WM_DESTROYCLIPBOARD,
        WM_RENDERALLFORMATS, WM_RENDERFORMAT, WNDCLASSW,
    };

    const CF_UNICODETEXT: u32 = 13;

    enum Entry {
        Memory(u32, Vec<u8>),
        EnhMetafile(Vec<u8>),
    }

    /// Kullanıcının panosunun yedeği (yalnız bellekte tutulur, diske yazılmaz).
    pub struct ClipboardBackup {
        entries: Vec<Entry>,
    }

    fn open_clipboard_retry() -> Result<(), InjectError> {
        open_clipboard_as(owner_hwnd())
    }

    fn open_clipboard_as(owner: Option<HWND>) -> Result<(), InjectError> {
        for _ in 0..10 {
            if unsafe { OpenClipboard(owner.unwrap_or_default()) }.is_ok() {
                return Ok(());
            }
            thread::sleep(Duration::from_millis(10));
        }
        Err(InjectError::ClipboardBusy)
    }

    unsafe fn alloc_and_copy(data: &[u8]) -> Result<HANDLE, InjectError> {
        let hmem = GlobalAlloc(GMEM_MOVEABLE, data.len().max(1))
            .map_err(|e| InjectError::Win32(e.to_string()))?;
        let ptr = GlobalLock(hmem);
        if ptr.is_null() {
            return Err(InjectError::Win32("GlobalLock başarısız".to_string()));
        }
        copy_nonoverlapping(data.as_ptr(), ptr as *mut u8, data.len());
        let _ = GlobalUnlock(hmem);
        Ok(HANDLE(hmem.0))
    }

    unsafe fn read_memory_format(fmt: u32) -> Option<Vec<u8>> {
        let handle = GetClipboardData(fmt).ok()?;
        let hglobal = HGLOBAL(handle.0);
        let size = GlobalSize(hglobal);
        if size == 0 {
            return None;
        }
        let ptr = GlobalLock(hglobal);
        if ptr.is_null() {
            return None;
        }
        let bytes = std::slice::from_raw_parts(ptr as *const u8, size).to_vec();
        let _ = GlobalUnlock(hglobal);
        Some(bytes)
    }

    unsafe fn read_enh_metafile() -> Option<Vec<u8>> {
        let handle = GetClipboardData(CF_ENHMETAFILE).ok()?;
        let hemf = HENHMETAFILE(handle.0);
        let size = GetEnhMetaFileBits(hemf, None) as usize;
        if size == 0 {
            return None;
        }
        let mut buf = vec![0u8; size];
        let written = GetEnhMetaFileBits(hemf, Some(&mut buf)) as usize;
        (written == size).then_some(buf)
    }

    /// Panoyu biçimlerine göre yedekler: bellek blokları kopyalanır, metafile içeriğiyle
    /// alınır, GDI tutamaçları ve Windows'un türettiği biçimler atlanır (`clipformats`).
    pub fn backup_clipboard() -> Result<ClipboardBackup, InjectError> {
        open_clipboard_retry()?;
        let mut entries = Vec::new();
        unsafe {
            let mut present = Vec::new();
            let mut fmt = EnumClipboardFormats(0);
            while fmt != 0 {
                present.push(fmt);
                fmt = EnumClipboardFormats(fmt);
            }
            let mut total = 0usize;
            for &fmt in &present {
                let entry = match classify(fmt, &present) {
                    FormatKind::Skip => None,
                    FormatKind::Memory => read_memory_format(fmt).map(|b| Entry::Memory(fmt, b)),
                    FormatKind::EnhMetafile => read_enh_metafile().map(Entry::EnhMetafile),
                };
                let Some(entry) = entry else { continue };
                let len = match &entry {
                    Entry::Memory(_, b) | Entry::EnhMetafile(b) => b.len(),
                };
                if total + len > MAX_BACKUP_BYTES && !is_essential_text(fmt) {
                    continue;
                }
                total += len;
                entries.push(entry);
            }
            let _ = CloseClipboard();
        }
        Ok(ClipboardBackup { entries })
    }

    /// Yedeği geri yükler. Geri yüklenen içerik Win+V geçmişine, bulut panosuna ve pano
    /// izleyicilerine (Meetin'in kendi geçmişi dahil) ikinci kez düşmesin diye işaretlenir.
    pub fn restore_clipboard(backup: &ClipboardBackup) -> Result<(), InjectError> {
        open_clipboard_retry()?;
        unsafe {
            let _ = EmptyClipboard();
            let has_marker = |fmt: u32| {
                backup
                    .entries
                    .iter()
                    .any(|e| matches!(e, Entry::Memory(f, _) if *f == fmt))
            };
            for entry in &backup.entries {
                match entry {
                    Entry::Memory(fmt, bytes) => {
                        if let Ok(h) = alloc_and_copy(bytes) {
                            let _ = SetClipboardData(*fmt, h);
                        }
                    }
                    Entry::EnhMetafile(bytes) => {
                        let hemf = SetEnhMetaFileBits(bytes);
                        if !hemf.is_invalid() {
                            let _ = SetClipboardData(CF_ENHMETAFILE, HANDLE(hemf.0));
                        }
                    }
                }
            }
            if !backup.entries.is_empty() {
                for name in [
                    "CanIncludeInClipboardHistory",
                    "CanUploadToCloudClipboard",
                    "ExcludeClipboardContentFromMonitorProcessing",
                ] {
                    let fmt = register_format(name);
                    if fmt != 0 && !has_marker(fmt) {
                        if let Ok(h) = alloc_and_copy(&0u32.to_le_bytes()) {
                            let _ = SetClipboardData(fmt, h);
                        }
                    }
                }
            }
            let _ = CloseClipboard();
        }
        Ok(())
    }

    // ---- Gecikmeli yazma (delayed rendering) ----------------------------------------
    // Meetin metni panoya "istenince verilecek" diye koyar. Hedef program yapıştırırken
    // veriyi istediğinde Windows gizli sahip penceresine WM_RENDERFORMAT gönderir; böylece
    // okumanın ne zaman bittiği tahmin edilmez, bilinir.

    struct Pending {
        formats: Vec<(u32, Vec<u8>)>,
        renders: u32,
    }

    static PENDING: Mutex<Option<Pending>> = Mutex::new(None);
    static RENDERED: Condvar = Condvar::new();

    fn pending_lock() -> std::sync::MutexGuard<'static, Option<Pending>> {
        PENDING.lock().unwrap_or_else(|e| e.into_inner())
    }

    unsafe fn render_format(fmt: u32) {
        let mut guard = pending_lock();
        if let Some(p) = guard.as_mut() {
            if let Some((_, bytes)) = p.formats.iter().find(|(f, _)| *f == fmt) {
                if let Ok(h) = alloc_and_copy(bytes) {
                    let _ = SetClipboardData(fmt, h);
                }
            }
            p.renders += 1;
            RENDERED.notify_all();
        }
    }

    unsafe extern "system" fn owner_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match msg {
            WM_RENDERFORMAT => {
                render_format(wparam.0 as u32);
                LRESULT(0)
            }
            WM_RENDERALLFORMATS => {
                if OpenClipboard(hwnd).is_ok() {
                    if GetClipboardOwner().ok() == Some(hwnd) {
                        let fmts: Vec<u32> = pending_lock()
                            .as_ref()
                            .map(|p| p.formats.iter().map(|(f, _)| *f).collect())
                            .unwrap_or_default();
                        for f in fmts {
                            render_format(f);
                        }
                    }
                    let _ = CloseClipboard();
                }
                LRESULT(0)
            }
            WM_DESTROYCLIPBOARD => {
                pending_lock().take();
                RENDERED.notify_all();
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }

    /// Gizli pano sahibi penceresi (`HWND_MESSAGE`); kendi iş parçacığında mesaj döngüsüyle.
    fn owner_hwnd() -> Option<HWND> {
        static OWNER: OnceLock<Option<isize>> = OnceLock::new();
        let raw = *OWNER.get_or_init(|| {
            let (tx, rx) = mpsc::channel::<Option<isize>>();
            let spawned = thread::Builder::new()
                .name("meetin-clip-owner".to_string())
                .spawn(move || unsafe {
                    let hinst = GetModuleHandleW(None)
                        .map(|m| HINSTANCE(m.0))
                        .unwrap_or_default();
                    let class = w!("MeetinClipboardOwner");
                    let wc = WNDCLASSW {
                        lpfnWndProc: Some(owner_proc),
                        hInstance: hinst,
                        lpszClassName: class,
                        ..Default::default()
                    };
                    let _ = RegisterClassW(&wc);
                    let created = CreateWindowExW(
                        WINDOW_EX_STYLE(0),
                        class,
                        w!(""),
                        WINDOW_STYLE(0),
                        0,
                        0,
                        0,
                        0,
                        HWND_MESSAGE,
                        None,
                        hinst,
                        None,
                    );
                    match created {
                        Ok(h) => {
                            let _ = tx.send(Some(h.0 as isize));
                            let mut msg = MSG::default();
                            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                                DispatchMessageW(&msg);
                            }
                        }
                        Err(_) => {
                            let _ = tx.send(None);
                        }
                    }
                });
            if spawned.is_err() {
                return None;
            }
            let hwnd = rx.recv_timeout(Duration::from_secs(2)).ok().flatten();
            if hwnd.is_none() {
                crate::logging::event(
                    crate::logging::Level::Warn,
                    "clipboard.owner_unavailable",
                    &[],
                );
            }
            hwnd
        });
        raw.map(|r| HWND(r as *mut c_void))
    }

    fn build_payload(plain: &str, html: Option<&str>) -> Vec<(u32, Vec<u8>)> {
        let wide: Vec<u16> = plain.encode_utf16().chain(std::iter::once(0)).collect();
        let text_bytes: Vec<u8> = wide.iter().flat_map(|u| u.to_le_bytes()).collect();
        let mut out = vec![(CF_UNICODETEXT, text_bytes)];
        if let Some(frag) = html {
            let fmt_html = register_format("HTML Format");
            if fmt_html != 0 {
                out.push((fmt_html, rich::build_cf_html(frag)));
            }
            let fmt_rtf = register_format("Rich Text Format");
            if fmt_rtf != 0 {
                out.push((fmt_rtf, rich::html_to_rtf(frag)));
            }
        }
        out
    }

    /// Metni gecikmeli yazma ile panoya koyar; gizlilik işaretleri gerçek veri olarak konur.
    fn offer_delayed(owner: HWND, payload: Vec<(u32, Vec<u8>)>) -> Result<(), InjectError> {
        open_clipboard_as(Some(owner))?;
        unsafe {
            // Önceki sahibimize WM_DESTROYCLIPBOARD burada (eşzamanlı) gider; bekleyen
            // içerik ondan SONRA kurulur.
            let _ = EmptyClipboard();
            let fmts: Vec<u32> = payload.iter().map(|(f, _)| *f).collect();
            *pending_lock() = Some(Pending {
                formats: payload,
                renders: 0,
            });
            for f in fmts {
                let _ = SetClipboardData(f, HANDLE::default());
            }
            for name in [
                "CanIncludeInClipboardHistory",
                "CanUploadToCloudClipboard",
                "ExcludeClipboardContentFromMonitorProcessing",
            ] {
                let fmt = register_format(name);
                if fmt != 0 {
                    if let Ok(h) = alloc_and_copy(&0u32.to_le_bytes()) {
                        let _ = SetClipboardData(fmt, h);
                    }
                }
            }
            let _ = CloseClipboard();
        }
        Ok(())
    }

    /// Hedef program panoyu okuyana (ve kısa bir sessizlik olana) kadar bekler.
    fn wait_for_render() {
        let mut wait = RenderWait::new(Instant::now());
        let mut seen = 0u32;
        let mut guard = pending_lock();
        loop {
            let renders = guard.as_ref().map(|p| p.renders).unwrap_or(seen);
            if renders > seen {
                seen = renders;
                wait.on_render(Instant::now());
            }
            if guard.is_none() || wait.done(Instant::now()) {
                break;
            }
            guard = RENDERED
                .wait_timeout(guard, Duration::from_millis(20))
                .map(|(g, _)| g)
                .unwrap_or_else(|e| e.into_inner().0);
        }
        guard.take();
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
        let paste_res = match owner_hwnd() {
            Some(owner) => {
                offer_delayed(owner, build_payload(plain, html))?;
                let r = send_paste_keys(shortcut);
                if r.is_ok() {
                    wait_for_render();
                }
                r
            }
            None => {
                // Sahip penceresi kurulamadıysa eski yöntem: doğrudan yaz, kısa bekle.
                set_transient_rich(plain, html)?;
                let r = send_paste_keys(shortcut);
                thread::sleep(Duration::from_millis(150));
                r
            }
        };
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
        let before = unsafe { GetClipboardSequenceNumber() };
        let sent_ok = send_ctrl_c().is_ok();
        if sent_ok {
            // Hedef program kopyalayana kadar (en çok 600 ms) bekle.
            let start = Instant::now();
            while unsafe { GetClipboardSequenceNumber() } == before
                && start.elapsed() < Duration::from_millis(600)
            {
                thread::sleep(Duration::from_millis(10));
            }
            thread::sleep(Duration::from_millis(15));
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
