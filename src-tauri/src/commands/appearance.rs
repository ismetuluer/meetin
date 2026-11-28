//! Görünüm komutları: sistem teması, pencere zemini ve tema yapılandırması.

use tauri::{AppHandle, Emitter, State};

use crate::windows::backdrop::{self, WindowBackdropInfo};
use crate::windows::theme::{self, SystemThemeInfo};
use crate::AppState;

#[tauri::command]
pub(crate) fn get_system_theme() -> SystemThemeInfo {
    theme::read_system_theme()
}

#[tauri::command]
pub(crate) fn get_window_backdrop_info() -> WindowBackdropInfo {
    backdrop::current_backdrop_info()
}

#[tauri::command]
pub(crate) fn get_theme_config(state: State<'_, AppState>) -> Result<Option<String>, String> {
    state
        .storage
        .get_setting("theme_config")
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) fn save_theme_config(
    app: AppHandle,
    state: State<'_, AppState>,
    config_json: String,
) -> Result<(), String> {
    state
        .storage
        .set_setting("theme_config", &config_json)
        .map_err(|e| e.to_string())?;
    backdrop::sync_app_windows_with_config(
        &app,
        Some(&config_json),
        theme::read_system_theme().dark,
    );
    let _ = app.emit("theme-config-changed", config_json);
    Ok(())
}
