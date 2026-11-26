//! Seçim balonu pencere yönetimi ("+ Ekle" kabarcığı, §4.1, M3.2).

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use tauri::{AppHandle, Manager, PhysicalPosition};

use crate::windows::mascot;

pub const BUBBLE_LABEL: &str = "bubble";

pub struct BubbleState {
    pub pending_text: Mutex<Option<String>>,
    pub enabled: AtomicBool,
    pub visible: AtomicBool,
    pub generation: AtomicU64,
}

impl Default for BubbleState {
    fn default() -> Self {
        Self {
            pending_text: Mutex::new(None),
            enabled: AtomicBool::new(true),
            visible: AtomicBool::new(false),
            generation: AtomicU64::new(0),
        }
    }
}

/// Sonraki zamanlayıcı nesil kimliğini üretir (önceki bekleyen balonları iptal eder).
pub fn bump_generation(state: &BubbleState) -> u64 {
    state.generation.fetch_add(1, Ordering::SeqCst) + 1
}

/// İmlecin yanında "+ Ekle" balonunu odak çalmadan gösterir; 3 sn sonra kendiliğinden gizler.
pub fn show_bubble_near(
    app: &AppHandle,
    state: &Arc<BubbleState>,
    text: String,
    x: i32,
    y: i32,
    expected_gen: u64,
) {
    if !state.enabled.load(Ordering::Relaxed) {
        return;
    }
    if state.generation.load(Ordering::SeqCst) != expected_gen {
        return;
    }

    if let Ok(mut guard) = state.pending_text.lock() {
        *guard = Some(text);
    }

    let Some(win) = app.get_webview_window(BUBBLE_LABEL) else {
        return;
    };

    let scale = win.scale_factor().unwrap_or(1.0);
    let offset_x = (12.0 * scale).round() as i32;
    let offset_y = (46.0 * scale).round() as i32;
    let bw = (104.0 * scale).round() as i32;
    let bh = (42.0 * scale).round() as i32;

    let mut target_x = x + offset_x;
    let mut target_y = y - offset_y;

    if let Ok(Some(mon)) = win.primary_monitor() {
        let mp = mon.position();
        let ms = mon.size();
        let max_x = (mp.x + ms.width as i32 - bw).max(mp.x);
        let max_y = (mp.y + ms.height as i32 - bh).max(mp.y);
        target_x = target_x.clamp(mp.x, max_x);
        target_y = target_y.clamp(mp.y, max_y);
    }

    #[cfg(windows)]
    mascot::apply_no_activate(&win, true);

    let _ = win.set_position(PhysicalPosition::new(target_x, target_y));
    state.visible.store(true, Ordering::SeqCst);
    let _ = win.show();

    // 3 sn içinde tıklanmazsa sessizce kaybolur (§4.1)
    let app_clone = app.clone();
    let state_clone = Arc::clone(state);
    thread::Builder::new()
        .name("meetin-bubble-ttl".to_string())
        .spawn(move || {
            thread::sleep(Duration::from_millis(3000));
            if state_clone.generation.load(Ordering::SeqCst) == expected_gen {
                state_clone.visible.store(false, Ordering::SeqCst);
                if let Some(w) = app_clone.get_webview_window(BUBBLE_LABEL) {
                    let _ = w.hide();
                }
            }
        })
        .ok();
}

/// Açık seçimi/balonu gizler ve bekleyen 300 ms / 3 sn zamanlayıcıları iptal eder.
pub fn hide_bubble(app: &AppHandle, state: &BubbleState) {
    bump_generation(state);
    if state.visible.swap(false, Ordering::SeqCst) {
        if let Some(win) = app.get_webview_window(BUBBLE_LABEL) {
            let _ = win.hide();
        }
    }
}

pub fn take_pending_text(state: &BubbleState) -> Option<String> {
    state.pending_text.lock().ok().and_then(|mut g| g.take())
}
