//! İçe ve dışa aktarma komutları: CSV, JSON, AHK ve `.mee` arşivi.

use std::collections::HashSet;

use tauri::{AppHandle, Emitter, State};

use crate::db::FolderRecord;
use crate::io::csv::{
    import_csv, preview_csv, ConflictStrategy, CsvColumnMapping, CsvImportSummary, CsvTablePreview,
};
use crate::io::formats::{
    export_ahk, export_csv, export_meetin_json, import_items, parse_import_file, ImportedPhraseItem,
};
use crate::io::mee_archive::export_mee_archive as build_mee_archive;
use crate::AppState;

#[tauri::command]
pub(crate) fn parse_csv_preview(bytes: Vec<u8>) -> Result<CsvTablePreview, String> {
    preview_csv(&bytes).map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) fn import_csv_phrases(
    app: AppHandle,
    state: State<'_, AppState>,
    bytes: Vec<u8>,
    mapping: CsvColumnMapping,
    strategy: ConflictStrategy,
    dry_run: bool,
) -> Result<CsvImportSummary, String> {
    let fold_dotted_i = state
        .storage
        .get_setting("fold_dotted_i")
        .map_err(|e| e.to_string())?
        .map(|v| v == "true")
        .unwrap_or(false);

    let summary = import_csv(
        &state.storage,
        &bytes,
        &mapping,
        strategy,
        fold_dotted_i,
        dry_run,
    )
    .map_err(|e| e.to_string())?;

    if !dry_run {
        state
            .hook
            .reload_matcher(&state.storage)
            .map_err(|e| e.to_string())?;
        let total_added = summary.imported + summary.overwritten + summary.renamed;
        if total_added > 0 {
            let _ = app.emit("file-imported", total_added);
        }
    }

    Ok(summary)
}

#[tauri::command]
pub(crate) fn export_phrases_subset(
    state: State<'_, AppState>,
    phrase_ids: Vec<i64>,
    format: String,
) -> Result<String, String> {
    let phrases = state
        .storage
        .list_phrases_by_ids(&phrase_ids)
        .map_err(|e| e.to_string())?;
    let folders = state.storage.list_folders().map_err(|e| e.to_string())?;
    if format.eq_ignore_ascii_case("csv") {
        export_csv(&phrases, &folders).map_err(|e| e.to_string())
    } else if format.eq_ignore_ascii_case("ahk") {
        Ok(export_ahk(&phrases))
    } else {
        export_meetin_json(&phrases, &folders).map_err(|e| e.to_string())
    }
}

#[tauri::command]
pub(crate) fn export_phrases_subset_mee(
    state: State<'_, AppState>,
    phrase_ids: Vec<i64>,
) -> Result<Vec<u8>, String> {
    let phrases = state
        .storage
        .list_phrases_by_ids(&phrase_ids)
        .map_err(|e| e.to_string())?;
    let all_folders = state.storage.list_folders().map_err(|e| e.to_string())?;
    let used_folder_ids: HashSet<i64> = phrases
        .iter()
        .flat_map(|p| p.folder_id.into_iter().chain(p.folder_ids.iter().copied()))
        .collect();
    let folders: Vec<FolderRecord> = all_folders
        .into_iter()
        .filter(|f| used_folder_ids.contains(&f.id))
        .collect();
    build_mee_archive(&phrases, &folders, env!("CARGO_PKG_VERSION")).map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) fn parse_import_file_cmd(
    filename: String,
    bytes: Vec<u8>,
) -> Result<Vec<ImportedPhraseItem>, String> {
    parse_import_file(&filename, &bytes).map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) fn bulk_create_phrases(
    state: State<'_, AppState>,
    items: Vec<ImportedPhraseItem>,
    conflict_strategy: Option<ConflictStrategy>,
) -> Result<CsvImportSummary, String> {
    let fold = state
        .storage
        .get_setting("fold_dotted_i")
        .ok()
        .flatten()
        .map(|v| v == "true")
        .unwrap_or(false);
    let summary = import_items(
        &state.storage,
        &items,
        conflict_strategy.unwrap_or(ConflictStrategy::SuggestNew),
        fold,
    )
    .map_err(|e| e.to_string())?;
    let _ = state.hook.reload_matcher(&state.storage);
    Ok(summary)
}

#[tauri::command]
pub(crate) fn export_library(state: State<'_, AppState>, format: String) -> Result<String, String> {
    let phrases = state
        .storage
        .list_phrases(false)
        .map_err(|e| e.to_string())?;
    let folders = state.storage.list_folders().map_err(|e| e.to_string())?;
    if format.eq_ignore_ascii_case("csv") {
        export_csv(&phrases, &folders).map_err(|e| e.to_string())
    } else if format.eq_ignore_ascii_case("ahk") {
        Ok(export_ahk(&phrases))
    } else {
        export_meetin_json(&phrases, &folders).map_err(|e| e.to_string())
    }
}

#[tauri::command]
pub(crate) fn export_mee_archive(state: State<'_, AppState>) -> Result<Vec<u8>, String> {
    let phrases = state
        .storage
        .list_phrases(false)
        .map_err(|e| e.to_string())?;
    let folders = state.storage.list_folders().map_err(|e| e.to_string())?;
    build_mee_archive(&phrases, &folders, env!("CARGO_PKG_VERSION")).map_err(|e| e.to_string())
}
