//! Genel kısayolların ayrıştırılması, kaydı ve ayar komutu (§12).

use std::collections::HashSet;
use std::str::FromStr;

use tauri::{AppHandle, State};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

use crate::db::Storage;
use crate::{AppState, ShortcutConfigDto};

pub(super) fn format_shortcut_display(raw: &str) -> String {
    raw.trim()
        .replace("CommandOrControl+", "Ctrl+")
        .replace("commandorcontrol+", "Ctrl+")
}

pub(crate) fn parse_shortcut(raw: &str) -> Result<Shortcut, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("Kısayol boş olamaz".to_string());
    }
    let normalized = trimmed
        .replace("Ctrl+", "CommandOrControl+")
        .replace("CTRL+", "CommandOrControl+")
        .replace("ctrl+", "CommandOrControl+");
    Shortcut::from_str(&normalized).map_err(|e| format!("Geçersiz kısayol ({trimmed}): {e}"))
}

pub(crate) fn apply_global_shortcuts(
    app: &AppHandle,
    cfg: &ShortcutConfigDto,
    storage: &Storage,
) -> Result<(), String> {
    let pause_sc = parse_shortcut(&cfg.pause)?;
    let search_sc = parse_shortcut(&cfg.search)?;
    let quick_add_sc = parse_shortcut(&cfg.quick_add)?;
    let studio_sc = parse_shortcut(&cfg.studio)?;
    let toggle_sc = parse_shortcut(&cfg.toggle_mascot)?;

    let mut seen = HashSet::new();
    for sc in [&pause_sc, &search_sc, &quick_add_sc, &studio_sc, &toggle_sc] {
        if !seen.insert(*sc) {
            return Err("Aynı kısayol birden fazla eyleme atanamaz".to_string());
        }
    }

    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    for sc in [pause_sc, search_sc, quick_add_sc, studio_sc, toggle_sc] {
        let _ = gs.register(sc);
    }

    // İfadeye özel atanmış kısayolları da kaydet
    if let Ok(phrases) = storage.list_phrases(false) {
        for p in phrases {
            if !p.enabled {
                continue;
            }
            if let Some(sc_str) = p.shortcut.as_deref() {
                if let Ok(sc) = parse_shortcut(sc_str) {
                    if seen.insert(sc) {
                        let _ = gs.register(sc);
                    }
                }
            }
        }
    }
    Ok(())
}

#[tauri::command]
pub(crate) fn set_shortcuts_config(
    app: AppHandle,
    state: State<'_, AppState>,
    shortcuts: ShortcutConfigDto,
) -> Result<(), String> {
    let clean = ShortcutConfigDto {
        pause: format_shortcut_display(&shortcuts.pause),
        search: format_shortcut_display(&shortcuts.search),
        quick_add: format_shortcut_display(&shortcuts.quick_add),
        studio: format_shortcut_display(&shortcuts.studio),
        toggle_mascot: format_shortcut_display(&shortcuts.toggle_mascot),
    };
    apply_global_shortcuts(&app, &clean, &state.storage)?;
    let json = serde_json::to_string(&clean).map_err(|e| e.to_string())?;
    state
        .storage
        .set_setting("shortcuts_config", &json)
        .map_err(|e| e.to_string())?;
    if let Ok(mut g) = state.shortcuts.lock() {
        *g = clean;
    }
    Ok(())
}
