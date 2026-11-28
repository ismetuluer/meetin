//! Maskot, seçim baloncuğu ve stüdyo penceresi komutları (§5).

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::selection;
use crate::windows::bubble;
use crate::windows::mascot::{
    capture_foreground_target, ensure_mascot_shown,
    get_mascot_monitor_layout as get_monitor_layout_inner, move_mascot_window_pos,
    resize_mascot_window, toggle_user_visibility, MonitorLayoutDto,
};
use crate::windows::tray;
use crate::AppState;

#[tauri::command]
pub(crate) fn set_mascot_mode(app: AppHandle, state: State<'_, AppState>, mode: String) {
    match mode.as_str() {
        "search" => {
            capture_foreground_target(&app, &state.mascot);
            resize_mascot_window(&app, &state.mascot, 560.0, 380.0, true);
        }
        "add" => {
            capture_foreground_target(&app, &state.mascot);
            resize_mascot_window(&app, &state.mascot, 460.0, 410.0, true);
        }
        "fill" => {
            resize_mascot_window(&app, &state.mascot, 420.0, 340.0, true);
        }
        "pill" => {
            resize_mascot_window(&app, &state.mascot, 290.0, 96.0, false);
        }
        "suggest" => {
            resize_mascot_window(&app, &state.mascot, 360.0, 96.0, false);
        }
        "menu" => {
            resize_mascot_window(&app, &state.mascot, 224.0, 286.0, true);
        }
        _ => {
            resize_mascot_window(&app, &state.mascot, 100.0, 100.0, false);
        }
    }
}

#[tauri::command]
pub(crate) fn accept_selection_bubble(app: AppHandle, state: State<'_, AppState>) {
    let text = bubble::take_pending_text(&state.bubble).unwrap_or_default();
    bubble::hide_bubble(&app, &state.bubble);
    ensure_mascot_shown(&app, &state.mascot);
    resize_mascot_window(&app, &state.mascot, 460.0, 410.0, true);
    let _ = app.emit("open-add-island", text);
}

#[tauri::command]
pub(crate) fn dismiss_selection_bubble(app: AppHandle, state: State<'_, AppState>) {
    bubble::hide_bubble(&app, &state.bubble);
}

#[tauri::command]
pub(crate) fn open_add_from_squeeze(app: AppHandle, state: State<'_, AppState>) {
    let text = selection::read_selected_text()
        .or_else(selection::read_clipboard_text)
        .unwrap_or_default();
    capture_foreground_target(&app, &state.mascot);
    ensure_mascot_shown(&app, &state.mascot);
    resize_mascot_window(&app, &state.mascot, 460.0, 410.0, true);
    let _ = app.emit("open-add-island", text);
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StudioDraftPayload {
    pub abbr: String,
    pub content: String,
    pub description: String,
}

#[tauri::command]
pub(crate) fn open_studio_with_draft(
    app: AppHandle,
    abbr: String,
    content: String,
    description: String,
) {
    tray::show_main_window(&app);
    let _ = app.emit(
        "studio-new-draft",
        StudioDraftPayload {
            abbr,
            content,
            description,
        },
    );
}

#[tauri::command]
pub(crate) fn move_mascot_window(app: AppHandle, state: State<'_, AppState>, x: i32, y: i32) {
    move_mascot_window_pos(&app, &state.mascot, x, y);
}

#[tauri::command]
pub(crate) fn get_mascot_monitor_layout(app: AppHandle) -> Result<MonitorLayoutDto, String> {
    get_monitor_layout_inner(&app)
}

#[tauri::command]
pub(crate) fn save_mascot_position(
    state: State<'_, AppState>,
    x: i32,
    y: i32,
) -> Result<(), String> {
    state
        .storage
        .set_setting("mascot_x", &x.to_string())
        .map_err(|e| e.to_string())?;
    state
        .storage
        .set_setting("mascot_y", &y.to_string())
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub(crate) fn toggle_mascot_visible(app: AppHandle, state: State<'_, AppState>) -> bool {
    toggle_user_visibility(&app, &state.mascot)
}

#[tauri::command]
pub(crate) fn open_studio_window(app: AppHandle) {
    tray::show_main_window(&app);
}

#[tauri::command]
pub(crate) fn toggle_studio_window(app: AppHandle) -> bool {
    if let Some(win) = app.get_webview_window("main") {
        let visible = win.is_visible().unwrap_or(false);
        if visible {
            let _ = win.hide();
            return false;
        }
    }
    tray::show_main_window(&app);
    true
}

#[tauri::command]
pub(crate) fn quit_app(app: AppHandle, state: State<'_, AppState>) {
    let _ = state.storage.create_backup_now();
    app.exit(0);
}
