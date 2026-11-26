//! Masaüstü Basketbol mini oyunu pota penceresi (§Mini Oyunlar, v0.3.0).

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition};

use crate::windows::mascot;

pub const HOOP_LABEL: &str = "hoop";

/// Pota çemberinin ekrandaki fiziksel piksel koordinatları.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HoopRimRect {
    pub left: f64,
    pub right: f64,
    pub y: f64,
}

#[derive(Default)]
pub struct HoopState {
    pub active: AtomicBool,
    pub seed: AtomicU64,
}

fn rim_from_window_pos(x: i32, y: i32, scale: f64) -> HoopRimRect {
    // SVG potada çember x: 34..136, y: 120 (170x220 pencere içinde)
    HoopRimRect {
        left: x as f64 + 34.0 * scale,
        right: x as f64 + 136.0 * scale,
        y: y as f64 + 120.0 * scale,
    }
}

/// Potayı mevcut monitörün sağ veya sol yarısında rastgele bir yüksekliğe yerleştirir.
pub fn place_hoop_on_screen(app: &AppHandle, state: &HoopState) -> Option<HoopRimRect> {
    let win = app.get_webview_window(HOOP_LABEL)?;
    let monitor = win
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| win.primary_monitor().ok().flatten())?;

    let mp = monitor.position();
    let ms = monitor.size();
    let scale = win.scale_factor().unwrap_or(1.0);
    let hw = (170.0 * scale).round() as i32;
    let hh = (220.0 * scale).round() as i32;

    let step = state.seed.fetch_add(1, Ordering::Relaxed);
    let on_right = step.is_multiple_of(2);
    let margin_x = (36.0 * scale).round() as i32;
    let target_x = if on_right {
        (mp.x + ms.width as i32 - hw - margin_x).max(mp.x)
    } else {
        mp.x + margin_x
    };

    let min_y = mp.y + (90.0 * scale).round() as i32;
    let max_y = (mp.y + ms.height as i32 - hh - (140.0 * scale).round() as i32).max(min_y + 1);
    let span = (max_y - min_y).max(1) as u64;
    // Deterministik ama çeşitli yükseklik dağılımı
    let pseudo = ((step.wrapping_mul(2654435761)) >> 8) % span;
    let target_y = min_y + pseudo as i32;

    #[cfg(windows)]
    mascot::apply_no_activate(&win, true);

    let _ = win.set_position(PhysicalPosition::new(target_x, target_y));
    let _ = win.show();

    Some(rim_from_window_pos(target_x, target_y, scale))
}

/// Açık pota çemberinin güncel ekran koordinatlarını döndürür.
pub fn get_current_hoop_rim(app: &AppHandle) -> Option<HoopRimRect> {
    let win = app.get_webview_window(HOOP_LABEL)?;
    let pos = win.outer_position().ok()?;
    let scale = win.scale_factor().unwrap_or(1.0);
    Some(rim_from_window_pos(pos.x, pos.y, scale))
}

/// Basketbol mini oyununu açar veya kapatır.
pub fn toggle_hoop(app: &AppHandle, state: &HoopState) -> bool {
    let was_active = state.active.load(Ordering::Relaxed);
    let next = !was_active;
    state.active.store(next, Ordering::Relaxed);

    if next {
        let _ = place_hoop_on_screen(app, state);
    } else if let Some(win) = app.get_webview_window(HOOP_LABEL) {
        let _ = win.hide();
    }

    let _ = app.emit("basketball-mode-changed", next);
    next
}
