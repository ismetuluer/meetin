//! Uygulama ayarları, profil, dil, karşılama ve oturum açılışında başlatma komutları.

use std::sync::atomic::Ordering as AtomicOrdering;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};

use super::shortcuts::format_shortcut_display;
use crate::hook::TriggerConfig;
use crate::windows::startup::{self, StartupStatus};
use crate::{AppState, ShortcutConfigDto};

/// Desteklenen arayüz dili kodunu normalleştirir (`"tr"` veya `"en"`).
fn normalize_language_code(raw: &str) -> &'static str {
    let trimmed = raw.trim();
    let bytes = trimmed.as_bytes();
    if bytes.eq_ignore_ascii_case(b"en")
        || (bytes.len() >= 3
            && bytes[..2].eq_ignore_ascii_case(b"en")
            && matches!(bytes[2], b'-' | b'_'))
    {
        "en"
    } else {
        "tr"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettingsDto {
    pub trigger_config: TriggerConfig,
    pub suggest_enabled: bool,
    pub excluded_processes: Vec<String>,
    pub mascot_sound: bool,
    pub mascot_size: String,
    pub user_name: String,
    pub onboarding_done: bool,
    pub shortcuts: ShortcutConfigDto,
    pub language: String,
    pub clipboard_enabled: bool,
    pub clipboard_limit: usize,
    pub abbr_suggest_mode: String,
    pub shift_f3_enabled: bool,
    pub sound_baa: bool,
    pub sound_throw: bool,
    pub sound_impact: bool,
    pub auto_check_updates: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MascotPrefsPayload {
    pub sound: bool,
    pub size: String,
    pub sound_baa: bool,
    pub sound_throw: bool,
    pub sound_impact: bool,
}

#[tauri::command]
pub(crate) fn get_app_settings(state: State<'_, AppState>) -> Result<AppSettingsDto, String> {
    let mascot_sound = state
        .storage
        .get_setting("mascot_sound")
        .ok()
        .flatten()
        .map(|v| v == "true")
        .unwrap_or(true);
    let sound_baa = state
        .storage
        .get_setting("sound_baa")
        .ok()
        .flatten()
        .map(|v| v == "true")
        .unwrap_or(false);
    let sound_throw = state
        .storage
        .get_setting("sound_throw")
        .ok()
        .flatten()
        .map(|v| v == "true")
        .unwrap_or(true);
    let sound_impact = state
        .storage
        .get_setting("sound_impact")
        .ok()
        .flatten()
        .map(|v| v == "true")
        .unwrap_or(true);
    let mascot_size = state
        .storage
        .get_setting("mascot_size")
        .ok()
        .flatten()
        .filter(|s| matches!(s.as_str(), "small" | "normal" | "large"))
        .unwrap_or_else(|| "normal".to_string());
    let user_name = state
        .storage
        .get_setting("user_name")
        .ok()
        .flatten()
        .unwrap_or_default();
    let onboarding_done = state
        .storage
        .get_setting("onboarding_done")
        .ok()
        .flatten()
        .map(|v| v == "true")
        .unwrap_or(false);
    let mut shortcuts = state
        .shortcuts
        .lock()
        .map(|g| g.clone())
        .unwrap_or_default();
    shortcuts.pause = format_shortcut_display(&shortcuts.pause);
    shortcuts.search = format_shortcut_display(&shortcuts.search);
    shortcuts.quick_add = format_shortcut_display(&shortcuts.quick_add);
    shortcuts.studio = format_shortcut_display(&shortcuts.studio);
    shortcuts.toggle_mascot = format_shortcut_display(&shortcuts.toggle_mascot);
    let language = state
        .storage
        .get_setting("language")
        .ok()
        .flatten()
        .map(|v| normalize_language_code(&v).to_string())
        .unwrap_or_else(|| "tr".to_string());
    let clipboard_enabled = state.clipboard_cfg.enabled.load(AtomicOrdering::Relaxed);
    let clipboard_limit = state.clipboard_cfg.limit.load(AtomicOrdering::Relaxed);
    let abbr_suggest_mode = state
        .storage
        .get_setting("abbr_suggest_mode")
        .ok()
        .flatten()
        .filter(|m| matches!(m.as_str(), "initials" | "first_word" | "last_word" | "tag"))
        .unwrap_or_else(|| "initials".to_string());
    let shift_f3_enabled = state.hook.is_shift_f3_enabled();
    let auto_check_updates = state
        .storage
        .get_setting("auto_check_updates")
        .ok()
        .flatten()
        .map(|v| v != "false")
        .unwrap_or(true);

    Ok(AppSettingsDto {
        trigger_config: state.hook.get_trigger_config(),
        suggest_enabled: state.hook.is_suggest_enabled(),
        excluded_processes: state.hook.get_excluded_processes(),
        mascot_sound,
        mascot_size,
        user_name,
        onboarding_done,
        shortcuts,
        language,
        clipboard_enabled,
        clipboard_limit,
        abbr_suggest_mode,
        shift_f3_enabled,
        sound_baa,
        sound_throw,
        sound_impact,
        auto_check_updates,
    })
}

#[tauri::command]
pub(crate) fn set_abbr_suggest_mode(
    state: State<'_, AppState>,
    mode: String,
) -> Result<String, String> {
    let norm = if matches!(
        mode.as_str(),
        "initials" | "first_word" | "last_word" | "tag"
    ) {
        mode
    } else {
        "initials".to_string()
    };
    state
        .storage
        .set_setting("abbr_suggest_mode", &norm)
        .map_err(|e| e.to_string())?;
    Ok(norm)
}

#[tauri::command]
pub(crate) fn get_startup_status() -> StartupStatus {
    startup::get_startup_status()
}

#[tauri::command]
pub(crate) fn set_run_at_login(enabled: bool) -> Result<StartupStatus, String> {
    startup::set_run_at_login(enabled)?;
    Ok(startup::get_startup_status())
}

#[tauri::command]
pub(crate) fn set_admin_autostart(enabled: bool) -> Result<StartupStatus, String> {
    startup::set_admin_autostart(enabled)?;
    Ok(startup::get_startup_status())
}

#[tauri::command]
pub(crate) fn set_mascot_prefs(
    app: AppHandle,
    state: State<'_, AppState>,
    sound: bool,
    size: String,
    sound_baa: Option<bool>,
    sound_throw: Option<bool>,
    sound_impact: Option<bool>,
) -> Result<(), String> {
    let norm_size = if matches!(size.as_str(), "small" | "normal" | "large") {
        size
    } else {
        "normal".to_string()
    };
    let cur_baa = sound_baa.unwrap_or_else(|| {
        state
            .storage
            .get_setting("sound_baa")
            .ok()
            .flatten()
            .map(|v| v == "true")
            .unwrap_or(false)
    });
    let cur_throw = sound_throw.unwrap_or_else(|| {
        state
            .storage
            .get_setting("sound_throw")
            .ok()
            .flatten()
            .map(|v| v == "true")
            .unwrap_or(true)
    });
    let cur_impact = sound_impact.unwrap_or_else(|| {
        state
            .storage
            .get_setting("sound_impact")
            .ok()
            .flatten()
            .map(|v| v == "true")
            .unwrap_or(true)
    });

    state
        .storage
        .set_setting("mascot_sound", if sound { "true" } else { "false" })
        .map_err(|e| e.to_string())?;
    state
        .storage
        .set_setting("mascot_size", &norm_size)
        .map_err(|e| e.to_string())?;
    state
        .storage
        .set_setting("sound_baa", if cur_baa { "true" } else { "false" })
        .map_err(|e| e.to_string())?;
    state
        .storage
        .set_setting("sound_throw", if cur_throw { "true" } else { "false" })
        .map_err(|e| e.to_string())?;
    state
        .storage
        .set_setting("sound_impact", if cur_impact { "true" } else { "false" })
        .map_err(|e| e.to_string())?;

    let _ = app.emit(
        "mascot-prefs-changed",
        MascotPrefsPayload {
            sound,
            size: norm_size,
            sound_baa: cur_baa,
            sound_throw: cur_throw,
            sound_impact: cur_impact,
        },
    );
    Ok(())
}

#[tauri::command]
pub(crate) fn set_user_profile(
    state: State<'_, AppState>,
    user_name: String,
) -> Result<(), String> {
    state
        .storage
        .set_setting("user_name", user_name.trim())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) fn set_language(
    app: AppHandle,
    state: State<'_, AppState>,
    language: String,
) -> Result<String, String> {
    let norm = normalize_language_code(&language);
    state
        .storage
        .set_setting("language", norm)
        .map_err(|e| e.to_string())?;
    let _ = app.emit("language-changed", norm);
    Ok(norm.to_string())
}

#[tauri::command]
pub(crate) fn complete_onboarding(
    app: AppHandle,
    state: State<'_, AppState>,
    user_name: String,
) -> Result<(), String> {
    state
        .storage
        .set_setting("user_name", user_name.trim())
        .map_err(|e| e.to_string())?;
    state
        .storage
        .set_setting("onboarding_done", "true")
        .map_err(|e| e.to_string())?;
    let _ = app.emit("onboarding-wake-mascot", ());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_language_codes_safely() {
        assert_eq!(normalize_language_code("tr"), "tr");
        assert_eq!(normalize_language_code("tr-TR"), "tr");
        assert_eq!(normalize_language_code("Türkçe"), "tr");
        assert_eq!(normalize_language_code("İngilizce"), "tr");
        assert_eq!(normalize_language_code("en"), "en");
        assert_eq!(normalize_language_code("en-US"), "en");
        assert_eq!(normalize_language_code("en_GB"), "en");
        assert_eq!(normalize_language_code(" EN "), "en");
        assert_eq!(normalize_language_code("english"), "tr");
        assert_eq!(normalize_language_code(""), "tr");
        assert_eq!(normalize_language_code("de-DE"), "tr");
    }
}
