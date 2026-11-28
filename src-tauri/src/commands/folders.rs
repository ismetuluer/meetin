//! Klasör komutları.

use tauri::State;

use crate::db::FolderRecord;
use crate::AppState;

#[tauri::command]
pub(crate) fn list_folders(state: State<'_, AppState>) -> Result<Vec<FolderRecord>, String> {
    state.storage.list_folders().map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) fn create_folder(
    state: State<'_, AppState>,
    name: String,
    parent_id: Option<i64>,
    color: Option<String>,
) -> Result<FolderRecord, String> {
    state
        .storage
        .create_folder(&name, parent_id, color.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) fn delete_folder(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    state.storage.delete_folder(id).map_err(|e| e.to_string())?;
    let _ = state.hook.reload_matcher(&state.storage);
    Ok(())
}

#[tauri::command]
pub(crate) fn update_folder(
    state: State<'_, AppState>,
    id: i64,
    name: Option<String>,
    color: Option<String>,
) -> Result<FolderRecord, String> {
    let color_arg = color.map(|c| if c.trim().is_empty() { None } else { Some(c) });
    let color_ref = color_arg.as_ref().map(|opt| opt.as_deref());
    state
        .storage
        .update_folder(id, name.as_deref(), color_ref)
        .map_err(|e| e.to_string())
}
