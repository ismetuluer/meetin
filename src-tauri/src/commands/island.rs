//! Ada komutları: alan doldurma ve adadan ifade yapıştırma (§3.3, §5.2).

use std::collections::HashMap;
use std::thread;
use std::time::Duration;

use tauri::{AppHandle, Emitter, State};

use crate::hook::resolve_with_storage;
use crate::inject;
use crate::vars::extract_form_fields;
use crate::windows::mascot::{focus_previous_target, resize_mascot_window};
use crate::{AppState, FillIslandPayload, PendingFillJob};

#[tauri::command]
pub(crate) fn submit_fill_island(
    app: AppHandle,
    state: State<'_, AppState>,
    values: HashMap<String, String>,
) -> Result<(), String> {
    let job = state
        .pending_fill
        .lock()
        .map_err(|_| "Kilit alınamadı".to_string())?
        .take()
        .ok_or_else(|| "Bekleyen form işi bulunamadı".to_string())?;

    let fold = state
        .storage
        .get_setting("fold_dotted_i")
        .ok()
        .flatten()
        .map(|v| v == "true")
        .unwrap_or(false);

    let resolved = resolve_with_storage(&state.storage, &job.content, values.clone(), fold);
    let resolved_html = job
        .content_html
        .as_deref()
        .map(|h| resolve_with_storage(&state.storage, h, values, fold).text);

    // Odağı hedef pencereye döndürüp yapıştır
    resize_mascot_window(&app, &state.mascot, 100.0, 100.0, false);
    let prev_proc = focus_previous_target(&state.mascot);
    let target_proc = job.process_name.or(prev_proc);
    thread::sleep(Duration::from_millis(65));

    if let Err(err) = inject::inject_resolved(
        0,
        &resolved.text,
        resolved_html.as_deref(),
        job.trigger,
        resolved.cursor_left_steps,
        target_proc.as_deref(),
    ) {
        let msg = err.to_string();
        let _ = app.emit("engine-error", &msg);
        return Err(msg);
    }

    let _ = state
        .storage
        .record_usage(job.phrase_id, target_proc.as_deref());
    let _ = app.emit("phrase-expanded", job.abbr);
    Ok(())
}

#[tauri::command]
pub(crate) fn cancel_fill_island(app: AppHandle, state: State<'_, AppState>) {
    if let Ok(mut g) = state.pending_fill.lock() {
        *g = None;
    }
    resize_mascot_window(&app, &state.mascot, 100.0, 100.0, false);
}

#[tauri::command]
pub(crate) fn paste_phrase_from_island(
    app: AppHandle,
    state: State<'_, AppState>,
    phrase_id: i64,
) -> Result<(), String> {
    let phrase = state
        .storage
        .get_phrase(phrase_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("İfade bulunamadı: {phrase_id}"))?;

    // Doldurulacak alan varsa Alan Doldurma adasına geç (§3.3, §5.2)
    let fields = extract_form_fields(&phrase.content);
    if !fields.is_empty() {
        if let Ok(mut g) = state.pending_fill.lock() {
            *g = Some(PendingFillJob {
                phrase_id: phrase.id,
                abbr: phrase.abbr.clone(),
                content: phrase.content.clone(),
                content_html: phrase.content_html.clone(),
                trigger: None,
                process_name: None,
            });
        }
        resize_mascot_window(&app, &state.mascot, 420.0, 340.0, true);
        let _ = app.emit(
            "open-fill-island",
            FillIslandPayload {
                abbr: phrase.abbr,
                fields,
                preview: phrase.content,
            },
        );
        return Ok(());
    }

    let fold = state
        .storage
        .get_setting("fold_dotted_i")
        .ok()
        .flatten()
        .map(|v| v == "true")
        .unwrap_or(false);
    let resolved = resolve_with_storage(&state.storage, &phrase.content, HashMap::new(), fold);
    let resolved_html = phrase
        .content_html
        .as_deref()
        .map(|h| resolve_with_storage(&state.storage, h, HashMap::new(), fold).text);

    // Önce ada penceresini no-activate durumuna al ve odağı hedef pencereye döndür
    resize_mascot_window(&app, &state.mascot, 100.0, 100.0, false);
    let proc_name = focus_previous_target(&state.mascot);
    thread::sleep(Duration::from_millis(65));

    if let Err(err) = inject::inject_resolved(
        0,
        &resolved.text,
        resolved_html.as_deref(),
        None,
        resolved.cursor_left_steps,
        proc_name.as_deref(),
    ) {
        let msg = err.to_string();
        let _ = app.emit("engine-error", &msg);
        return Err(msg);
    }

    let _ = state.storage.record_usage(phrase.id, proc_name.as_deref());
    let _ = app.emit("phrase-expanded", phrase.abbr);
    Ok(())
}
