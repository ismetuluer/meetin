//! Basketbol oyunu (pota) komutları.

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};

use crate::windows::hoop::{get_current_hoop_rim, place_hoop_on_screen, toggle_hoop, HoopRimRect};
use crate::AppState;

#[tauri::command]
pub(crate) fn toggle_basketball_game(app: AppHandle, state: State<'_, AppState>) -> bool {
    toggle_hoop(&app, &state.hoop)
}

#[tauri::command]
pub(crate) fn get_hoop_rim_rect(app: AppHandle) -> Option<HoopRimRect> {
    get_current_hoop_rim(&app)
}

#[tauri::command]
pub(crate) fn reposition_hoop_random(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Option<HoopRimRect> {
    if !state.hoop.active.load(std::sync::atomic::Ordering::Relaxed) {
        return None;
    }
    place_hoop_on_screen(&app, &state.hoop)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasketScorePayload {
    pub score: u32,
    pub streak: u32,
}

#[tauri::command]
pub(crate) fn notify_basket_scored(app: AppHandle, score: u32, streak: u32) {
    let _ = app.emit("basket-scored", BasketScorePayload { score, streak });
}
