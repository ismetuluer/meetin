//! Pano komutları: geçmiş tercihleri, harf dönüştürme ve pano geçmişi.

use std::sync::atomic::Ordering as AtomicOrdering;
use std::thread;
use std::time::Duration;

use tauri::{AppHandle, Manager, State};

use crate::db::ClipboardHistoryRecord;
use crate::inject;
use crate::matcher::case::{tr_lower, tr_title, tr_upper};
use crate::selection;
use crate::windows::mascot::focus_previous_target;
use crate::AppState;

#[tauri::command]
pub(crate) fn set_clipboard_prefs(
    state: State<'_, AppState>,
    enabled: bool,
    limit: usize,
) -> Result<(), String> {
    let clamped = limit.clamp(1, 200);
    state
        .storage
        .set_setting("clipboard_enabled", if enabled { "true" } else { "false" })
        .map_err(|e| e.to_string())?;
    state
        .storage
        .set_setting("clipboard_limit", &clamped.to_string())
        .map_err(|e| e.to_string())?;
    state
        .clipboard_cfg
        .enabled
        .store(enabled, AtomicOrdering::Relaxed);
    state
        .clipboard_cfg
        .limit
        .store(clamped, AtomicOrdering::Relaxed);
    let _ = state.storage.trim_clipboard_history(clamped);
    Ok(())
}

#[tauri::command]
pub(crate) fn transform_clipboard_text(
    text: String,
    mode: String,
    paste_now: bool,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let transformed = match mode.as_str() {
        "upper" => tr_upper(&text),
        "lower" => tr_lower(&text),
        "title" => tr_title(&text),
        _ => text,
    };
    if paste_now {
        if let Some(win) = app.get_webview_window("main") {
            let _ = win.hide();
        }
        thread::sleep(Duration::from_millis(80));
        let proc_name = focus_previous_target(&state.mascot);
        let shortcut = inject::paste_shortcut_for_process(proc_name.as_deref());
        inject::clipboard::paste_rich_preserving_clipboard(&transformed, None, shortcut)
            .map_err(|e| e.to_string())?;
    } else {
        inject::clipboard::write_unicode_text(&transformed).map_err(|e| e.to_string())?;
    }
    Ok(transformed)
}

#[tauri::command]
pub(crate) fn read_clipboard_text() -> Option<String> {
    selection::read_clipboard_text()
}

#[tauri::command]
pub(crate) fn list_clipboard_history(
    state: State<'_, AppState>,
    limit: Option<usize>,
) -> Result<Vec<ClipboardHistoryRecord>, String> {
    state
        .storage
        .list_clipboard_history(limit.unwrap_or(100))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) fn clear_clipboard_history(state: State<'_, AppState>) -> Result<usize, String> {
    state
        .storage
        .clear_clipboard_history()
        .map_err(|e| e.to_string())
}
