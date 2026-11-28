//! İfade komutları: oluşturma, düzenleme, çöp kutusu, sürümler, uygulama kuralları, istatistik.

use tauri::{AppHandle, Emitter, State};

use super::shortcuts::apply_global_shortcuts;
use crate::db::{NewPhrase, PhraseRecord, PhraseVersionRecord, UpdatePhrase, UsageStats};
use crate::matcher::AppRule;
use crate::AppState;

#[tauri::command]
pub(crate) fn list_phrases(
    state: State<'_, AppState>,
    include_deleted: bool,
) -> Result<Vec<PhraseRecord>, String> {
    state
        .storage
        .list_phrases(include_deleted)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) fn create_phrase(
    app: AppHandle,
    state: State<'_, AppState>,
    input: NewPhrase,
) -> Result<PhraseRecord, String> {
    let rec = state
        .storage
        .create_phrase(&input)
        .map_err(|e| e.to_string())?;
    state
        .hook
        .reload_matcher(&state.storage)
        .map_err(|e| e.to_string())?;
    if rec.shortcut.is_some() {
        if let Ok(sc) = state.shortcuts.lock().map(|g| g.clone()) {
            let _ = apply_global_shortcuts(&app, &sc, &state.storage);
        }
    }
    let _ = app.emit("phrase-created", &rec.abbr);
    Ok(rec)
}

#[tauri::command]
pub(crate) fn update_phrase(
    app: AppHandle,
    state: State<'_, AppState>,
    input: UpdatePhrase,
) -> Result<PhraseRecord, String> {
    let rec = state
        .storage
        .update_phrase(&input)
        .map_err(|e| e.to_string())?;
    state
        .hook
        .reload_matcher(&state.storage)
        .map_err(|e| e.to_string())?;
    if let Ok(sc) = state.shortcuts.lock().map(|g| g.clone()) {
        let _ = apply_global_shortcuts(&app, &sc, &state.storage);
    }
    Ok(rec)
}

#[tauri::command]
pub(crate) fn delete_phrase(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
) -> Result<(), String> {
    state
        .storage
        .soft_delete_phrase(id)
        .map_err(|e| e.to_string())?;
    state
        .hook
        .reload_matcher(&state.storage)
        .map_err(|e| e.to_string())?;
    if let Ok(sc) = state.shortcuts.lock().map(|g| g.clone()) {
        let _ = apply_global_shortcuts(&app, &sc, &state.storage);
    }
    Ok(())
}

#[tauri::command]
pub(crate) fn restore_phrase(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
) -> Result<(), String> {
    state
        .storage
        .restore_phrase(id)
        .map_err(|e| e.to_string())?;
    state
        .hook
        .reload_matcher(&state.storage)
        .map_err(|e| e.to_string())?;
    if let Ok(sc) = state.shortcuts.lock().map(|g| g.clone()) {
        let _ = apply_global_shortcuts(&app, &sc, &state.storage);
    }
    Ok(())
}

#[tauri::command]
pub(crate) fn move_phrase_folder(
    state: State<'_, AppState>,
    phrase_id: i64,
    target_folder_id: Option<i64>,
    source_folder_id: Option<i64>,
    copy: bool,
) -> Result<PhraseRecord, String> {
    let rec = state
        .storage
        .move_or_copy_phrase_folder(phrase_id, target_folder_id, source_folder_id, copy)
        .map_err(|e| e.to_string())?;
    let _ = state.hook.reload_matcher(&state.storage);
    Ok(rec)
}

#[tauri::command]
pub(crate) fn get_phrase_app_rules(
    state: State<'_, AppState>,
    phrase_id: i64,
) -> Result<Vec<AppRule>, String> {
    state
        .storage
        .get_phrase_app_rules(phrase_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) fn set_phrase_app_rules(
    state: State<'_, AppState>,
    phrase_id: i64,
    rules: Vec<AppRule>,
) -> Result<(), String> {
    state
        .storage
        .set_phrase_app_rules(phrase_id, &rules)
        .map_err(|e| e.to_string())?;
    state
        .hook
        .reload_matcher(&state.storage)
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub(crate) fn list_phrase_versions(
    state: State<'_, AppState>,
    phrase_id: i64,
) -> Result<Vec<PhraseVersionRecord>, String> {
    state
        .storage
        .list_phrase_versions(phrase_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) fn restore_phrase_version(
    state: State<'_, AppState>,
    phrase_id: i64,
    version_id: i64,
) -> Result<PhraseRecord, String> {
    let restored = state
        .storage
        .restore_phrase_version(phrase_id, version_id)
        .map_err(|e| e.to_string())?;
    let _ = state.hook.reload_matcher(&state.storage);
    Ok(restored)
}

#[tauri::command]
pub(crate) fn hard_delete_phrase(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    state
        .storage
        .hard_delete_phrase(id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) fn empty_trash(state: State<'_, AppState>) -> Result<usize, String> {
    state.storage.empty_trash().map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) fn get_usage_stats(state: State<'_, AppState>) -> Result<UsageStats, String> {
    state.storage.get_usage_stats().map_err(|e| e.to_string())
}
