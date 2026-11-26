//! Yüzen maskot + ada penceresi yönetimi (§5.1, §5.2, §3.6, M2.3–M2.6).
//!
//! - `WS_EX_NOACTIVATE` ile maskot/hap modlarında etkin pencere odağını çalmaz.
//! - Arama adası açılmadan önce önceki etkin pencere (`prev_hwnd`) ve işlem adı
//!   kaydedilir; Enter ile yapıştırma öncesi odak o pencereye iade edilir.
//! - Tam ekran uygulamalarda (video, sunum, oyun) kendiliğinden gizlenir.
//! - Etkin pencere yükseltilmiş (yönetici) yetkideyse kalkan rozeti bildirilir.

use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{
    AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, PhysicalPosition, WebviewWindow,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorBoundsDto {
    pub min_x: i32,
    pub min_y: i32,
    pub max_x: i32,
    pub max_y: i32,
}

/// Maskot penceresinin bulunduğu monitörde gidebileceği fiziksel sınırları döndürür.
pub fn get_mascot_monitor_bounds(app: &AppHandle) -> MonitorBoundsDto {
    let Some(win) = app.get_webview_window(MASCOT_LABEL) else {
        return MonitorBoundsDto {
            min_x: 0,
            min_y: 0,
            max_x: 1800,
            max_y: 960,
        };
    };

    let monitor = win
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| win.primary_monitor().ok().flatten());

    let Some(m) = monitor else {
        return MonitorBoundsDto {
            min_x: 0,
            min_y: 0,
            max_x: 1800,
            max_y: 960,
        };
    };

    let mpos = m.position();
    let msize = m.size();
    let wsize = win
        .outer_size()
        .unwrap_or(tauri::PhysicalSize::new(100, 100));

    MonitorBoundsDto {
        min_x: mpos.x,
        min_y: mpos.y,
        max_x: (mpos.x + msize.width as i32 - wsize.width as i32).max(mpos.x),
        max_y: (mpos.y + msize.height as i32 - wsize.height as i32).max(mpos.y),
    }
}

use crate::db::Storage;
use crate::windows::{backdrop, theme};
use crate::AppState;

pub const MASCOT_LABEL: &str = "mascot";

/// Maskot penceresinin paylaşılan durumu.
#[derive(Default)]
pub struct MascotWindowState {
    pub prev_hwnd: AtomicIsize,
    pub prev_process: Mutex<Option<String>>,
    pub user_hidden: AtomicBool,
    pub fullscreen_hidden: AtomicBool,
    pub anchor_pos: Mutex<Option<(i32, i32)>>,
    pub moved_while_expanded: AtomicBool,
    pub island_backdrop_active: AtomicBool,
}

impl MascotWindowState {
    pub fn is_visible_logical(&self) -> bool {
        !self.user_hidden.load(Ordering::Relaxed)
            && !self.fullscreen_hidden.load(Ordering::Relaxed)
    }
}

/// Maskot penceresinin son kaydedilen konumunu yükler; yoksa ana ekranın sağ alt köşesine yerleştirir.
pub fn restore_saved_position(app: &AppHandle, storage: &Storage) {
    let Some(win) = app.get_webview_window(MASCOT_LABEL) else {
        return;
    };

    let saved_x = storage
        .get_setting("mascot_x")
        .ok()
        .flatten()
        .and_then(|v| v.parse::<i32>().ok());
    let saved_y = storage
        .get_setting("mascot_y")
        .ok()
        .flatten()
        .and_then(|v| v.parse::<i32>().ok());

    if let (Some(x), Some(y)) = (saved_x, saved_y) {
        let _ = win.set_position(tauri::PhysicalPosition::new(x, y));
        return;
    }

    if let Ok(Some(monitor)) = win.primary_monitor() {
        let size = monitor.size();
        let scale = monitor.scale_factor();
        let logical_w = size.width as f64 / scale;
        let logical_h = size.height as f64 / scale;
        let x = (logical_w - 132.0).max(24.0);
        let y = (logical_h - 160.0).max(24.0);
        let _ = win.set_position(LogicalPosition::new(x, y));
    }
}

/// Pencereyi bulunduğu monitör sınırları içinde tutar.
fn clamp_to_monitor(win: &WebviewWindow, x: i32, y: i32, w: i32, h: i32) -> (i32, i32) {
    let monitor = win
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| win.primary_monitor().ok().flatten());

    let Some(m) = monitor else {
        return (x, y);
    };

    let mpos = m.position();
    let msize = m.size();
    let min_x = mpos.x;
    let min_y = mpos.y;
    let max_x = (mpos.x + msize.width as i32 - w).max(min_x);
    let max_y = (mpos.y + msize.height as i32 - h).max(min_y);

    (x.clamp(min_x, max_x), y.clamp(min_y, max_y))
}

/// Maskot penceresini odağı çalmadan ve anında taşır.
pub fn move_mascot_window_pos(app: &AppHandle, state: &MascotWindowState, x: i32, y: i32) {
    let Some(win) = app.get_webview_window(MASCOT_LABEL) else {
        return;
    };

    if state
        .anchor_pos
        .lock()
        .map(|g| g.is_some())
        .unwrap_or(false)
    {
        state.moved_while_expanded.store(true, Ordering::Relaxed);
    }

    #[cfg(windows)]
    if let Ok(hwnd) = win.hwnd() {
        win_impl::move_window_no_activate(hwnd.0 as isize, x, y);
        return;
    }

    let _ = win.set_position(PhysicalPosition::new(x, y));
}

/// Maskot penceresinin boyutunu, monitör sınırını ve odak alma davranışını (`WS_EX_NOACTIVATE`) ayarlar.
pub fn resize_mascot_window(
    app: &AppHandle,
    state: &MascotWindowState,
    width: f64,
    height: f64,
    focusable: bool,
) {
    let Some(win) = app.get_webview_window(MASCOT_LABEL) else {
        return;
    };

    let is_compact = width <= 110.0 && height <= 110.0;

    if !is_compact {
        // Genişlemeden önceki maskot konumunu sakla
        if let Ok(mut anchor) = state.anchor_pos.lock() {
            if anchor.is_none() {
                if let Ok(pos) = win.outer_position() {
                    *anchor = Some((pos.x, pos.y));
                    state.moved_while_expanded.store(false, Ordering::Relaxed);
                }
            }
        }
    }

    #[cfg(windows)]
    apply_no_activate(&win, !focusable);

    let target_role = backdrop::role_for_mascot_size(width, height, focusable);
    let is_island = target_role == backdrop::WindowRole::MascotIsland;
    let was_island = state.island_backdrop_active.swap(is_island, Ordering::Relaxed);
    if is_island {
        let island_dark =
            backdrop::resolve_current_island_dark(app, theme::read_system_theme().dark);
        backdrop::apply_mascot_backdrop(&win, backdrop::WindowRole::MascotIsland, island_dark);
    } else if was_island {
        backdrop::apply_mascot_backdrop(&win, backdrop::WindowRole::MascotCompact, false);
    }

    let _ = win.set_size(LogicalSize::new(width, height));

    let scale = win.scale_factor().unwrap_or(1.0);
    let phys_w = (width * scale).round() as i32;
    let phys_h = (height * scale).round() as i32;

    if is_compact {
        let was_moved = state.moved_while_expanded.swap(false, Ordering::Relaxed);
        let saved_anchor = state.anchor_pos.lock().ok().and_then(|mut g| g.take());
        if !was_moved {
            if let Some((ax, ay)) = saved_anchor {
                let (cx, cy) = clamp_to_monitor(&win, ax, ay, phys_w, phys_h);
                let _ = win.set_position(PhysicalPosition::new(cx, cy));
            }
        } else if let Ok(pos) = win.outer_position() {
            let (cx, cy) = clamp_to_monitor(&win, pos.x, pos.y, phys_w, phys_h);
            let _ = win.set_position(PhysicalPosition::new(cx, cy));
        }
    } else if let Ok(pos) = win.outer_position() {
        let (cx, cy) = clamp_to_monitor(&win, pos.x, pos.y, phys_w, phys_h);
        if cx != pos.x || cy != pos.y {
            let _ = win.set_position(PhysicalPosition::new(cx, cy));
        }
    }

    if focusable {
        let _ = win.show();
        let _ = win.set_focus();
    }
}

/// Arama adası açılmadan önce önceki etkin pencereyi yakalar.
pub fn capture_foreground_target(app: &AppHandle, state: &MascotWindowState) {
    #[cfg(windows)]
    {
        let own_mascot = app
            .get_webview_window(MASCOT_LABEL)
            .and_then(|w| w.hwnd().ok())
            .map(|h| h.0 as isize)
            .unwrap_or(0);
        let own_main = app
            .get_webview_window("main")
            .and_then(|w| w.hwnd().ok())
            .map(|h| h.0 as isize)
            .unwrap_or(0);
        win_impl::capture_foreground_target(state, own_mascot, own_main);
    }
    #[cfg(not(windows))]
    {
        let _ = (app, state);
    }
}

/// Arama adasından Enter ile seçim yapıldığında odağı önceki pencereye geri verir.
pub fn focus_previous_target(state: &MascotWindowState) -> Option<String> {
    #[cfg(windows)]
    {
        win_impl::focus_previous_target(state)
    }
    #[cfg(not(windows))]
    {
        let _ = state;
        None
    }
}

/// Kullanıcı maskotu göster/gizle kısayolu veya menüyle değiştirdiğinde çalışır.
pub fn toggle_user_visibility(app: &AppHandle, state: &MascotWindowState) -> bool {
    let was_hidden = state.user_hidden.load(Ordering::Relaxed);
    let now_hidden = !was_hidden;
    state.user_hidden.store(now_hidden, Ordering::Relaxed);

    if let Some(win) = app.get_webview_window(MASCOT_LABEL) {
        if now_hidden || state.fullscreen_hidden.load(Ordering::Relaxed) {
            let _ = win.hide();
        } else {
            let _ = win.show();
            #[cfg(windows)]
            apply_no_activate(&win, true);
        }
    }

    let visible = state.is_visible_logical();
    let _ = app.emit("mascot-visibility-changed", visible);
    visible
}

/// Maskotu görünür yapar (ör. arama adası kısayoluyla açılırken).
pub fn ensure_mascot_shown(app: &AppHandle, state: &MascotWindowState) {
    state.user_hidden.store(false, Ordering::Relaxed);
    if let Some(win) = app.get_webview_window(MASCOT_LABEL) {
        let _ = win.show();
    }
    let _ = app.emit("mascot-visibility-changed", true);
}

/// `WS_EX_NOACTIVATE` bayrağını uygular veya kaldırır.
#[cfg(windows)]
pub fn apply_no_activate(window: &WebviewWindow, no_activate: bool) {
    if let Ok(hwnd) = window.hwnd() {
        win_impl::set_no_activate(hwnd.0 as isize, no_activate);
    }
}

/// Tam ekran uygulamaları ve yönetici yetkili pencereleri izleyen hafif arka plan iş parçacığı.
pub fn start_environment_watcher(app: AppHandle) {
    #[cfg(windows)]
    {
        thread::Builder::new()
            .name("meetin-env-watch".to_string())
            .spawn(move || {
                let mut last_fullscreen = false;
                let mut last_admin = false;

                loop {
                    thread::sleep(Duration::from_millis(800));

                    let state = app.state::<AppState>();
                    let mascot_state: Arc<MascotWindowState> = Arc::clone(&state.mascot);

                    let fs = win_impl::is_foreground_fullscreen();
                    if fs != last_fullscreen {
                        last_fullscreen = fs;
                        mascot_state.fullscreen_hidden.store(fs, Ordering::Relaxed);

                        if !mascot_state.user_hidden.load(Ordering::Relaxed) {
                            if let Some(win) = app.get_webview_window(MASCOT_LABEL) {
                                if fs {
                                    let _ = win.hide();
                                } else {
                                    let _ = win.show();
                                    apply_no_activate(&win, true);
                                }
                            }
                            let _ = app.emit("mascot-visibility-changed", !fs);
                        }
                    }

                    let own_mascot = app
                        .get_webview_window(MASCOT_LABEL)
                        .and_then(|w| w.hwnd().ok())
                        .map(|h| h.0 as isize)
                        .unwrap_or(0);
                    let own_main = app
                        .get_webview_window("main")
                        .and_then(|w| w.hwnd().ok())
                        .map(|h| h.0 as isize)
                        .unwrap_or(0);

                    let admin = win_impl::is_foreground_elevated(own_mascot, own_main);
                    if admin != last_admin {
                        last_admin = admin;
                        let _ = app.emit("admin-window-changed", admin);
                    }
                }
            })
            .ok();
    }
    #[cfg(not(windows))]
    {
        let _ = app;
    }
}

#[cfg(windows)]
mod win_impl {
    use super::*;
    use std::ffi::c_void;
    use std::mem::size_of;
    use windows::core::PWSTR;
    use windows::Win32::Foundation::{CloseHandle, GetLastError, ERROR_ACCESS_DENIED, HANDLE, HWND};
    use windows::Win32::Security::{
        GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY,
    };
    use windows::Win32::System::Threading::{
        OpenProcess, OpenProcessToken, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows::Win32::UI::Shell::{
        SHQueryUserNotificationState, QUNS_BUSY, QUNS_PRESENTATION_MODE,
        QUNS_RUNNING_D3D_FULL_SCREEN,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowLongW, GetWindowThreadProcessId, SetForegroundWindow,
        SetWindowLongW, SetWindowPos, GWL_EXSTYLE, SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER,
        WS_EX_NOACTIVATE,
    };

    pub fn move_window_no_activate(hwnd_raw: isize, x: i32, y: i32) {
        if hwnd_raw == 0 {
            return;
        }
        unsafe {
            let hwnd = HWND(hwnd_raw as *mut c_void);
            let _ = SetWindowPos(
                hwnd,
                HWND::default(),
                x,
                y,
                0,
                0,
                SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOZORDER,
            );
        }
    }

    pub fn set_no_activate(hwnd_raw: isize, no_activate: bool) {
        if hwnd_raw == 0 {
            return;
        }
        unsafe {
            let hwnd = HWND(hwnd_raw as *mut c_void);
            let current = GetWindowLongW(hwnd, GWL_EXSTYLE);
            let flag = WS_EX_NOACTIVATE.0 as i32;
            let next = if no_activate {
                current | flag
            } else {
                current & !flag
            };
            if next != current {
                SetWindowLongW(hwnd, GWL_EXSTYLE, next);
            }
        }
    }

    fn process_name_for_pid(pid: u32) -> Option<String> {
        if pid == 0 {
            return None;
        }
        unsafe {
            let Ok(handle) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
                return None;
            };
            let mut path_buf = [0u16; 512];
            let mut len = path_buf.len() as u32;
            let ok = QueryFullProcessImageNameW(
                handle,
                PROCESS_NAME_WIN32,
                PWSTR(path_buf.as_mut_ptr()),
                &mut len,
            );
            let _ = CloseHandle(handle);
            if ok.is_ok() && len > 0 {
                let full = String::from_utf16_lossy(&path_buf[..len as usize]);
                full.rsplit(['\\', '/']).next().map(|s| s.to_string())
            } else {
                None
            }
        }
    }

    pub fn capture_foreground_target(
        state: &MascotWindowState,
        own_mascot: isize,
        own_main: isize,
    ) {
        unsafe {
            let fg = GetForegroundWindow();
            let raw = fg.0 as isize;
            if raw == 0 || raw == own_mascot || raw == own_main {
                return;
            }
            state.prev_hwnd.store(raw, Ordering::Relaxed);
            let mut pid: u32 = 0;
            GetWindowThreadProcessId(fg, Some(&mut pid));
            let proc_name = process_name_for_pid(pid);
            if let Ok(mut guard) = state.prev_process.lock() {
                *guard = proc_name;
            }
        }
    }

    pub fn focus_previous_target(state: &MascotWindowState) -> Option<String> {
        let raw = state.prev_hwnd.load(Ordering::Relaxed);
        if raw != 0 {
            unsafe {
                let _ = SetForegroundWindow(HWND(raw as *mut c_void));
            }
        }
        state
            .prev_process
            .lock()
            .ok()
            .and_then(|g| g.clone())
    }

    pub fn is_foreground_fullscreen() -> bool {
        unsafe {
            if let Ok(state) = SHQueryUserNotificationState() {
                return state == QUNS_BUSY
                    || state == QUNS_RUNNING_D3D_FULL_SCREEN
                    || state == QUNS_PRESENTATION_MODE;
            }
            false
        }
    }

    pub fn is_foreground_elevated(own_mascot: isize, own_main: isize) -> bool {
        unsafe {
            let fg = GetForegroundWindow();
            let raw = fg.0 as isize;
            if raw == 0 || raw == own_mascot || raw == own_main {
                return false;
            }
            let mut pid: u32 = 0;
            GetWindowThreadProcessId(fg, Some(&mut pid));
            if pid == 0 {
                return false;
            }

            let proc_handle = match OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
                Ok(h) => h,
                Err(_) => {
                    // UIPI nedeniyle ACCESS_DENIED alınıyorsa hedef süreç yüksek yetkili olabilir
                    return GetLastError() == ERROR_ACCESS_DENIED;
                }
            };

            let mut token = HANDLE::default();
            let tok_ok = OpenProcessToken(proc_handle, TOKEN_QUERY, &mut token);
            let _ = CloseHandle(proc_handle);
            if tok_ok.is_err() {
                return GetLastError() == ERROR_ACCESS_DENIED;
            }

            let mut elevation = TOKEN_ELEVATION::default();
            let mut ret_len: u32 = 0;
            let info_ok = GetTokenInformation(
                token,
                TokenElevation,
                Some(&mut elevation as *mut _ as *mut c_void),
                size_of::<TOKEN_ELEVATION>() as u32,
                &mut ret_len,
            );
            let _ = CloseHandle(token);

            info_ok.is_ok() && elevation.TokenIsElevated != 0
        }
    }
}
