//! Yedek komutları: listeleme, anında yedek ve geri yükleme (§7.3).

use tauri::{AppHandle, Emitter, State};

use super::shortcuts::apply_global_shortcuts;
use crate::db::backup::BackupFileInfo;
use crate::AppState;

#[tauri::command]
pub(crate) fn list_backups(state: State<'_, AppState>) -> Result<Vec<BackupFileInfo>, String> {
    state.storage.list_backups().map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) fn create_backup_now(state: State<'_, AppState>) -> Result<BackupFileInfo, String> {
    state.storage.create_backup_now().map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) fn restore_backup(
    app: AppHandle,
    state: State<'_, AppState>,
    filename: String,
) -> Result<BackupFileInfo, String> {
    let safety = state
        .storage
        .restore_from_backup(&filename)
        .map_err(|e| e.to_string())?;
    state
        .hook
        .reload_matcher(&state.storage)
        .map_err(|e| e.to_string())?;
    let _ = app.emit("library-restored", ());
    if let Ok(sc) = state.shortcuts.lock().map(|g| g.clone()) {
        let _ = apply_global_shortcuts(&app, &sc, &state.storage);
    }
    Ok(safety)
}
