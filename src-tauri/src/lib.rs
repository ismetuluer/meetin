//! Meetin çekirdek kütüphanesi (§10).

pub mod db;
pub mod hook;
pub mod inject;
pub mod io;
pub mod matcher;
pub mod selection;
pub mod updater;
pub mod vars;
pub mod windows;

use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use std::sync::atomic::Ordering as AtomicOrdering;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, RunEvent, State};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use crate::db::backup::BackupFileInfo;
use crate::db::{
    ClipboardHistoryRecord, FolderRecord, NewPhrase, PhraseRecord, PhraseVersionRecord, Storage,
    UpdatePhrase, UsageStats,
};
use crate::hook::{resolve_with_storage, HookCallbacks, HookController, TriggerConfig};
use crate::inject::clipboard::ClipboardWatcherConfig;
use crate::io::csv::{
    import_csv, preview_csv, ConflictStrategy, CsvColumnMapping, CsvImportSummary, CsvTablePreview,
};
use crate::io::formats::{
    export_ahk, export_csv, export_meetin_json, import_items, parse_import_file, ImportedPhraseItem,
};
use crate::io::mee_archive::export_mee_archive as build_mee_archive;
use crate::matcher::case::{tr_lower, tr_title, tr_upper};
use crate::matcher::AppRule;
use crate::vars::{extract_form_fields, FormField};
use crate::windows::backdrop::{self, WindowBackdropInfo};
use crate::windows::bubble::{self, BubbleState, BUBBLE_LABEL};
use crate::windows::hoop::{
    get_current_hoop_rim, place_hoop_on_screen, toggle_hoop, HoopRimRect, HoopState, HOOP_LABEL,
};
use crate::windows::mascot::{
    self, capture_foreground_target, ensure_mascot_shown, focus_previous_target,
    get_mascot_monitor_bounds as get_monitor_bounds_inner, move_mascot_window_pos,
    resize_mascot_window, toggle_user_visibility, MascotWindowState, MonitorBoundsDto,
    MASCOT_LABEL,
};
use crate::windows::startup::{self, StartupStatus};
use crate::windows::theme::{self, SystemThemeInfo};
use crate::windows::tray;
use crate::updater::{UpdateCheckResult, UpdateInstallResult};

pub const DEFAULT_UPDATE_MANIFEST_URL: &str =
    "https://github.com/ismetuluer/meetin/releases/latest/download/latest.json";

#[derive(Debug, Clone)]
pub struct PendingFillJob {
    pub phrase_id: i64,
    pub abbr: String,
    pub content: String,
    pub content_html: Option<String>,
    pub trigger: Option<char>,
    pub process_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FillIslandPayload {
    pub abbr: String,
    pub fields: Vec<FormField>,
    pub preview: String,
}

fn default_toggle_mascot_shortcut() -> String {
    "Ctrl+Alt+Space".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortcutConfigDto {
    pub pause: String,
    pub search: String,
    pub quick_add: String,
    pub studio: String,
    #[serde(default = "default_toggle_mascot_shortcut")]
    pub toggle_mascot: String,
}

impl Default for ShortcutConfigDto {
    fn default() -> Self {
        Self {
            pause: "Ctrl+Alt+P".to_string(),
            search: "Ctrl+Alt+F".to_string(),
            quick_add: "Ctrl+Alt+E".to_string(),
            studio: "Ctrl+Alt+S".to_string(),
            toggle_mascot: "Ctrl+Alt+Space".to_string(),
        }
    }
}

fn format_shortcut_display(raw: &str) -> String {
    raw.trim()
        .replace("CommandOrControl+", "Ctrl+")
        .replace("commandorcontrol+", "Ctrl+")
}

/// Desteklenen arayüz dili kodunu normalleştirir (`"tr"` veya `"en"`).
pub fn normalize_language_code(raw: &str) -> &'static str {
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
    pub update_manifest_url: String,
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
pub struct RunningAppInfo {
    pub process_name: String,
    pub window_title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MascotPrefsPayload {
    pub sound: bool,
    pub size: String,
    pub sound_baa: bool,
    pub sound_throw: bool,
    pub sound_impact: bool,
}

fn parse_shortcut(raw: &str) -> Result<Shortcut, String> {
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

fn apply_global_shortcuts(
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

pub struct AppState {
    pub storage: Arc<Storage>,
    pub hook: HookController,
    pub mascot: Arc<MascotWindowState>,
    pub bubble: Arc<BubbleState>,
    pub hoop: Arc<HoopState>,
    pub pending_fill: Mutex<Option<PendingFillJob>>,
    pub shortcuts: Mutex<ShortcutConfigDto>,
    pub clipboard_cfg: ClipboardWatcherConfig,
    pub latest_update_check: Mutex<Option<UpdateCheckResult>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EngineStatus {
    pub paused: bool,
    pub fold_dotted_i: bool,
    pub active_phrase_count: usize,
    pub data_dir: String,
    pub version: String,
}

#[tauri::command]
fn get_engine_status(state: State<'_, AppState>) -> Result<EngineStatus, String> {
    let fold_dotted_i = state
        .storage
        .get_setting("fold_dotted_i")
        .map_err(|e| e.to_string())?
        .map(|v| v == "true")
        .unwrap_or(false);

    let count = state
        .storage
        .list_phrases(false)
        .map_err(|e| e.to_string())?
        .into_iter()
        .filter(|p| p.enabled)
        .count();

    let data_dir = state
        .storage
        .db_path()
        .parent()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();

    Ok(EngineStatus {
        paused: state.hook.is_paused(),
        fold_dotted_i,
        active_phrase_count: count,
        data_dir,
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

#[tauri::command]
fn toggle_pause(app: AppHandle, state: State<'_, AppState>) -> bool {
    let paused = state.hook.toggle_paused();
    tray::update_tray_paused(&app, paused);
    let _ = app.emit("engine-status-changed", paused);
    paused
}

#[tauri::command]
fn set_fold_dotted_i(state: State<'_, AppState>, enabled: bool) -> Result<(), String> {
    state
        .storage
        .set_setting("fold_dotted_i", if enabled { "true" } else { "false" })
        .map_err(|e| e.to_string())?;
    state.hook.set_fold_dotted_i(enabled);
    Ok(())
}

#[tauri::command]
fn get_system_theme() -> SystemThemeInfo {
    theme::read_system_theme()
}

#[tauri::command]
fn get_window_backdrop_info() -> WindowBackdropInfo {
    backdrop::current_backdrop_info()
}

#[tauri::command]
fn get_theme_config(state: State<'_, AppState>) -> Result<Option<String>, String> {
    state
        .storage
        .get_setting("theme_config")
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn save_theme_config(
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

#[tauri::command]
fn get_app_settings(state: State<'_, AppState>) -> Result<AppSettingsDto, String> {
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
    let update_manifest_url = state
        .storage
        .get_setting("update_manifest_url")
        .ok()
        .flatten()
        .filter(|u| !u.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_UPDATE_MANIFEST_URL.to_string());
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
        update_manifest_url,
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
fn open_data_dir(state: State<'_, AppState>) -> Result<(), String> {
    let dir = state
        .storage
        .db_path()
        .parent()
        .ok_or_else(|| "Veri klasörü bulunamadı".to_string())?;
    let _ = std::fs::create_dir_all(dir);
    #[cfg(windows)]
    {
        std::process::Command::new("explorer.exe")
            .arg(dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn set_suggest_enabled(state: State<'_, AppState>, enabled: bool) -> Result<(), String> {
    state
        .storage
        .set_setting("suggest_enabled", if enabled { "true" } else { "false" })
        .map_err(|e| e.to_string())?;
    state.hook.set_suggest_enabled(enabled);
    Ok(())
}

#[tauri::command]
fn set_shift_f3_enabled(state: State<'_, AppState>, enabled: bool) -> Result<(), String> {
    state
        .storage
        .set_setting("shift_f3_enabled", if enabled { "true" } else { "false" })
        .map_err(|e| e.to_string())?;
    state.hook.set_shift_f3_enabled(enabled);
    Ok(())
}

#[tauri::command]
fn set_abbr_suggest_mode(state: State<'_, AppState>, mode: String) -> Result<String, String> {
    let norm = if matches!(mode.as_str(), "initials" | "first_word" | "last_word" | "tag") {
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
fn set_clipboard_prefs(
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
fn list_running_apps() -> Vec<RunningAppInfo> {
    #[cfg(windows)]
    {
        use ::windows::core::PWSTR;
        use ::windows::Win32::Foundation::{CloseHandle, BOOL, HWND, LPARAM};
        use ::windows::Win32::System::Threading::{
            OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
            PROCESS_QUERY_LIMITED_INFORMATION,
        };
        use ::windows::Win32::UI::WindowsAndMessaging::{
            EnumWindows, GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible,
        };

        struct CollectCtx {
            map: HashMap<String, RunningAppInfo>,
        }

        unsafe extern "system" fn enum_cb(hwnd: HWND, lparam: LPARAM) -> BOOL {
            if !IsWindowVisible(hwnd).as_bool() {
                return BOOL(1);
            }
            let mut title_buf = [0u16; 256];
            let title_len = GetWindowTextW(hwnd, &mut title_buf);
            if title_len <= 0 {
                return BOOL(1);
            }
            let title = String::from_utf16_lossy(&title_buf[..title_len as usize])
                .trim()
                .to_string();
            if title.is_empty() {
                return BOOL(1);
            }

            let mut pid: u32 = 0;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            if pid == 0 {
                return BOOL(1);
            }

            if let Ok(handle) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
                let mut path_buf = [0u16; 512];
                let mut len = path_buf.len() as u32;
                let ok = QueryFullProcessImageNameW(
                    handle,
                    PROCESS_NAME_WIN32,
                    PWSTR(path_buf.as_mut_ptr()),
                    &mut len,
                );
                let _ = CloseHandle(handle);
                if ok.is_ok() && len > 0 {
                    let full = String::from_utf16_lossy(&path_buf[..len as usize]);
                    if let Some(proc_name) = full.rsplit(['\\', '/']).next() {
                        let key = tr_lower(proc_name);
                        if key != "meetin.exe" && key != "textinputhost.exe" {
                            let ctx = &mut *(lparam.0 as *mut CollectCtx);
                            ctx.map.entry(key).or_insert_with(|| RunningAppInfo {
                                process_name: proc_name.to_string(),
                                window_title: title,
                            });
                        }
                    }
                }
            }
            BOOL(1)
        }

        let mut ctx = CollectCtx {
            map: HashMap::new(),
        };
        unsafe {
            let _ = EnumWindows(Some(enum_cb), LPARAM(&mut ctx as *mut CollectCtx as isize));
        }
        let mut list: Vec<RunningAppInfo> = ctx.map.into_values().collect();
        list.sort_by_key(|a| tr_lower(&a.process_name));
        list
    }
    #[cfg(not(windows))]
    {
        Vec::new()
    }
}

#[tauri::command]
fn transform_clipboard_text(
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
fn accept_typing_suggestion(state: State<'_, AppState>, phrase_id: i64) {
    state.hook.accept_suggestion(phrase_id);
}

#[tauri::command]
fn get_startup_status() -> StartupStatus {
    startup::get_startup_status()
}

#[tauri::command]
fn set_run_at_login(enabled: bool) -> Result<StartupStatus, String> {
    startup::set_run_at_login(enabled)?;
    Ok(startup::get_startup_status())
}

#[tauri::command]
fn set_admin_autostart(enabled: bool) -> Result<StartupStatus, String> {
    startup::set_admin_autostart(enabled)?;
    Ok(startup::get_startup_status())
}

#[tauri::command]
fn set_auto_check_updates(state: State<'_, AppState>, enabled: bool) -> Result<(), String> {
    state
        .storage
        .set_setting("auto_check_updates", if enabled { "true" } else { "false" })
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_update_env_info(state: State<'_, AppState>) -> UpdateCheckResult {
    if let Ok(guard) = state.latest_update_check.lock() {
        if let Some(cached) = guard.as_ref() {
            return cached.clone();
        }
    }
    updater::current_env_info()
}

fn resolve_manifest_source(storage: &Storage, manifest_source: Option<String>) -> String {
    let raw = manifest_source.unwrap_or_default();
    if raw.trim().is_empty() {
        storage
            .get_setting("update_manifest_url")
            .ok()
            .flatten()
            .filter(|u| !u.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_UPDATE_MANIFEST_URL.to_string())
    } else {
        raw.trim().to_string()
    }
}

#[tauri::command]
fn check_for_updates(
    app: AppHandle,
    state: State<'_, AppState>,
    manifest_source: Option<String>,
) -> Result<UpdateCheckResult, String> {
    let effective = resolve_manifest_source(&state.storage, manifest_source);
    let cur = env!("CARGO_PKG_VERSION");
    let portable = startup::is_portable_mode();
    let res = updater::check_manifest_source(&effective, cur, portable)?;
    if let Ok(mut guard) = state.latest_update_check.lock() {
        *guard = Some(res.clone());
    }
    if res.update_available && res.signature_status != updater::SIG_STATUS_INVALID {
        let _ = app.emit("update-available", res.latest_version.clone().unwrap_or_default());
    }
    Ok(res)
}

#[tauri::command]
fn install_verified_update(
    app: AppHandle,
    state: State<'_, AppState>,
    manifest_source: Option<String>,
) -> Result<UpdateInstallResult, String> {
    let effective = resolve_manifest_source(&state.storage, manifest_source);
    let cur = env!("CARGO_PKG_VERSION");
    let portable = startup::is_portable_mode();

    // Güncellemeden önce veritabanının güvenlik yedeğini al
    let _ = state.storage.create_backup_now();

    let res = updater::install_verified_update_from_source(&effective, cur, portable)?;
    if res.will_restart {
        let app_exit = app.clone();
        thread::Builder::new()
            .name("meetin-upd-exit".to_string())
            .spawn(move || {
                thread::sleep(Duration::from_millis(250));
                app_exit.exit(0);
            })
            .ok();
    }
    Ok(res)
}

#[tauri::command]
fn set_trigger_config(state: State<'_, AppState>, config: TriggerConfig) -> Result<(), String> {
    let json = serde_json::to_string(&config).map_err(|e| e.to_string())?;
    state
        .storage
        .set_setting("trigger_config", &json)
        .map_err(|e| e.to_string())?;
    state.hook.set_trigger_config(config);
    Ok(())
}

#[tauri::command]
fn set_excluded_processes(
    state: State<'_, AppState>,
    processes: Vec<String>,
) -> Result<Vec<String>, String> {
    let cleaned: Vec<String> = processes
        .into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let json = serde_json::to_string(&cleaned).map_err(|e| e.to_string())?;
    state
        .storage
        .set_setting("excluded_processes", &json)
        .map_err(|e| e.to_string())?;
    state.hook.set_excluded_processes(cleaned);
    Ok(state.hook.get_excluded_processes())
}

#[tauri::command]
fn set_mascot_prefs(
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
fn set_shortcuts_config(
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

#[tauri::command]
fn set_user_profile(state: State<'_, AppState>, user_name: String) -> Result<(), String> {
    state
        .storage
        .set_setting("user_name", user_name.trim())
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn set_language(
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
fn complete_onboarding(
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

#[tauri::command]
fn list_backups(state: State<'_, AppState>) -> Result<Vec<BackupFileInfo>, String> {
    state.storage.list_backups().map_err(|e| e.to_string())
}

#[tauri::command]
fn create_backup_now(state: State<'_, AppState>) -> Result<BackupFileInfo, String> {
    state.storage.create_backup_now().map_err(|e| e.to_string())
}

#[tauri::command]
fn restore_backup(
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

#[tauri::command]
fn list_phrases(
    state: State<'_, AppState>,
    include_deleted: bool,
) -> Result<Vec<PhraseRecord>, String> {
    state
        .storage
        .list_phrases(include_deleted)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn create_phrase(
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
fn update_phrase(
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
fn delete_phrase(app: AppHandle, state: State<'_, AppState>, id: i64) -> Result<(), String> {
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
fn restore_phrase(app: AppHandle, state: State<'_, AppState>, id: i64) -> Result<(), String> {
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
fn move_phrase_folder(
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
fn parse_csv_preview(bytes: Vec<u8>) -> Result<CsvTablePreview, String> {
    preview_csv(&bytes).map_err(|e| e.to_string())
}

#[tauri::command]
fn import_csv_phrases(
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
fn toggle_basketball_game(app: AppHandle, state: State<'_, AppState>) -> bool {
    toggle_hoop(&app, &state.hoop)
}

#[tauri::command]
fn get_hoop_rim_rect(app: AppHandle) -> Option<HoopRimRect> {
    get_current_hoop_rim(&app)
}

#[tauri::command]
fn reposition_hoop_random(app: AppHandle, state: State<'_, AppState>) -> Option<HoopRimRect> {
    if !state.hoop.active.load(std::sync::atomic::Ordering::Relaxed) {
        return None;
    }
    place_hoop_on_screen(&app, &state.hoop)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasketScorePayload {
    pub score: u32,
    pub streak: u32,
}

#[tauri::command]
fn notify_basket_scored(app: AppHandle, score: u32, streak: u32) {
    let _ = app.emit("basket-scored", BasketScorePayload { score, streak });
}

#[tauri::command]
fn list_folders(state: State<'_, AppState>) -> Result<Vec<FolderRecord>, String> {
    state.storage.list_folders().map_err(|e| e.to_string())
}

#[tauri::command]
fn read_clipboard_text() -> Option<String> {
    selection::read_clipboard_text()
}

#[tauri::command]
fn set_mascot_mode(app: AppHandle, state: State<'_, AppState>, mode: String) {
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
fn accept_selection_bubble(app: AppHandle, state: State<'_, AppState>) {
    let text = bubble::take_pending_text(&state.bubble).unwrap_or_default();
    bubble::hide_bubble(&app, &state.bubble);
    ensure_mascot_shown(&app, &state.mascot);
    resize_mascot_window(&app, &state.mascot, 460.0, 410.0, true);
    let _ = app.emit("open-add-island", text);
}

#[tauri::command]
fn dismiss_selection_bubble(app: AppHandle, state: State<'_, AppState>) {
    bubble::hide_bubble(&app, &state.bubble);
}

#[tauri::command]
fn open_add_from_squeeze(app: AppHandle, state: State<'_, AppState>) {
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
fn open_studio_with_draft(
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
fn move_mascot_window(app: AppHandle, state: State<'_, AppState>, x: i32, y: i32) {
    move_mascot_window_pos(&app, &state.mascot, x, y);
}

#[tauri::command]
fn get_mascot_monitor_bounds(app: AppHandle) -> MonitorBoundsDto {
    get_monitor_bounds_inner(&app)
}

#[tauri::command]
fn save_mascot_position(state: State<'_, AppState>, x: i32, y: i32) -> Result<(), String> {
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
fn toggle_mascot_visible(app: AppHandle, state: State<'_, AppState>) -> bool {
    toggle_user_visibility(&app, &state.mascot)
}

#[tauri::command]
fn open_studio_window(app: AppHandle) {
    tray::show_main_window(&app);
}

#[tauri::command]
fn toggle_studio_window(app: AppHandle) -> bool {
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
fn quit_app(app: AppHandle, state: State<'_, AppState>) {
    let _ = state.storage.create_backup_now();
    app.exit(0);
}

#[tauri::command]
fn get_phrase_app_rules(state: State<'_, AppState>, phrase_id: i64) -> Result<Vec<AppRule>, String> {
    state
        .storage
        .get_phrase_app_rules(phrase_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn set_phrase_app_rules(
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
fn submit_fill_island(
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
fn cancel_fill_island(app: AppHandle, state: State<'_, AppState>) {
    if let Ok(mut g) = state.pending_fill.lock() {
        *g = None;
    }
    resize_mascot_window(&app, &state.mascot, 100.0, 100.0, false);
}

#[tauri::command]
fn create_folder(
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
fn delete_folder(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    state.storage.delete_folder(id).map_err(|e| e.to_string())?;
    let _ = state.hook.reload_matcher(&state.storage);
    Ok(())
}

#[tauri::command]
fn update_folder(
    state: State<'_, AppState>,
    id: i64,
    name: Option<String>,
    color: Option<String>,
) -> Result<FolderRecord, String> {
    let color_arg = color.map(|c| {
        if c.trim().is_empty() {
            None
        } else {
            Some(c)
        }
    });
    let color_ref = color_arg.as_ref().map(|opt| opt.as_deref());
    state
        .storage
        .update_folder(id, name.as_deref(), color_ref)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn export_phrases_subset(
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
fn export_phrases_subset_mee(
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
fn list_phrase_versions(
    state: State<'_, AppState>,
    phrase_id: i64,
) -> Result<Vec<PhraseVersionRecord>, String> {
    state
        .storage
        .list_phrase_versions(phrase_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn restore_phrase_version(
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
fn hard_delete_phrase(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    state
        .storage
        .hard_delete_phrase(id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn empty_trash(state: State<'_, AppState>) -> Result<usize, String> {
    state.storage.empty_trash().map_err(|e| e.to_string())
}

#[tauri::command]
fn list_clipboard_history(
    state: State<'_, AppState>,
    limit: Option<usize>,
) -> Result<Vec<ClipboardHistoryRecord>, String> {
    state
        .storage
        .list_clipboard_history(limit.unwrap_or(100))
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn clear_clipboard_history(state: State<'_, AppState>) -> Result<usize, String> {
    state
        .storage
        .clear_clipboard_history()
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_usage_stats(state: State<'_, AppState>) -> Result<UsageStats, String> {
    state.storage.get_usage_stats().map_err(|e| e.to_string())
}

#[tauri::command]
fn parse_import_file_cmd(
    filename: String,
    bytes: Vec<u8>,
) -> Result<Vec<ImportedPhraseItem>, String> {
    parse_import_file(&filename, &bytes).map_err(|e| e.to_string())
}

#[tauri::command]
fn bulk_create_phrases(
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
fn export_library(state: State<'_, AppState>, format: String) -> Result<String, String> {
    let phrases = state.storage.list_phrases(false).map_err(|e| e.to_string())?;
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
fn export_mee_archive(state: State<'_, AppState>) -> Result<Vec<u8>, String> {
    let phrases = state.storage.list_phrases(false).map_err(|e| e.to_string())?;
    let folders = state.storage.list_folders().map_err(|e| e.to_string())?;
    build_mee_archive(&phrases, &folders, env!("CARGO_PKG_VERSION")).map_err(|e| e.to_string())
}

#[tauri::command]
fn paste_phrase_from_island(
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

    let _ = state
        .storage
        .record_usage(phrase.id, proc_name.as_deref());
    let _ = app.emit("phrase-expanded", phrase.abbr);
    Ok(())
}

pub fn run() {
    let data_dir = db::runtime_data_dir();
    let db_path = data_dir.join("meetin.db");
    let storage = Arc::new(Storage::open(&db_path).expect("Meetin veritabanı açılamadı"));

    // Günün ilk açılışında otomatik yedek (§7.3)
    let _ = storage.ensure_daily_backup_today();

    let storage_for_exit = Arc::clone(&storage);
    let (hook_controller, hook_starter) = hook::create_controller(Arc::clone(&storage));
    let bubble_state = Arc::new(BubbleState::default());
    let mascot_state = Arc::new(MascotWindowState::default());
    let hoop_state = Arc::new(HoopState::default());
    let shortcuts_init: ShortcutConfigDto = storage
        .get_setting("shortcuts_config")
        .ok()
        .flatten()
        .and_then(|raw| serde_json::from_str::<ShortcutConfigDto>(&raw).ok())
        .unwrap_or_default();

    let clip_enabled = storage
        .get_setting("clipboard_enabled")
        .ok()
        .flatten()
        .map(|v| v == "true")
        .unwrap_or(true);
    let clip_limit = storage
        .get_setting("clipboard_limit")
        .ok()
        .flatten()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(10)
        .clamp(1, 200);
    let clipboard_cfg = ClipboardWatcherConfig {
        enabled: Arc::new(std::sync::atomic::AtomicBool::new(clip_enabled)),
        limit: Arc::new(std::sync::atomic::AtomicUsize::new(clip_limit)),
    };
    let clipboard_cfg_for_setup = clipboard_cfg.clone();

    let bubble_for_select = Arc::clone(&bubble_state);
    let bubble_for_activity = Arc::clone(&bubble_state);
    let shortcuts_for_setup = shortcuts_init.clone();

    let app = tauri::Builder::default()
        .manage(AppState {
            storage: Arc::clone(&storage),
            hook: hook_controller,
            mascot: Arc::clone(&mascot_state),
            bubble: Arc::clone(&bubble_state),
            hoop: Arc::clone(&hoop_state),
            pending_fill: Mutex::new(None),
            shortcuts: Mutex::new(shortcuts_init),
            clipboard_cfg,
            latest_update_check: Mutex::new(None),
        })
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            tray::show_main_window(app);
        }))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state() != ShortcutState::Pressed {
                        return;
                    }
                    let state = app.state::<AppState>();
                    let sc_cfg = state
                        .shortcuts
                        .lock()
                        .map(|g| g.clone())
                        .unwrap_or_default();

                    if parse_shortcut(&sc_cfg.pause).ok().as_ref() == Some(shortcut) {
                        let paused = state.hook.toggle_paused();
                        tray::update_tray_paused(app, paused);
                        let _ = app.emit("engine-status-changed", paused);
                    } else if parse_shortcut(&sc_cfg.toggle_mascot).ok().as_ref() == Some(shortcut) {
                        toggle_user_visibility(app, &state.mascot);
                    } else if parse_shortcut(&sc_cfg.search).ok().as_ref() == Some(shortcut) {
                        capture_foreground_target(app, &state.mascot);
                        ensure_mascot_shown(app, &state.mascot);
                        resize_mascot_window(app, &state.mascot, 560.0, 380.0, true);
                        let _ = app.emit("open-search-island", ());
                    } else if parse_shortcut(&sc_cfg.quick_add).ok().as_ref() == Some(shortcut) {
                        // Seçili metin + kısayol ile doğrudan ekleme adası (§4.2)
                        bubble::hide_bubble(app, &state.bubble);
                        capture_foreground_target(app, &state.mascot);
                        let text = selection::read_selected_text()
                            .or_else(selection::read_clipboard_text)
                            .unwrap_or_default();
                        ensure_mascot_shown(app, &state.mascot);
                        resize_mascot_window(app, &state.mascot, 460.0, 410.0, true);
                        let _ = app.emit("open-add-island", text);
                    } else if parse_shortcut(&sc_cfg.studio).ok().as_ref() == Some(shortcut) {
                        tray::show_main_window(app);
                    } else if let Ok(phrases) = state.storage.list_phrases(false) {
                        // İfadeye özel atanmış kısayol eşleşmesi
                        if let Some(phrase) = phrases.into_iter().find(|p| {
                            p.enabled
                                && p.shortcut
                                    .as_deref()
                                    .and_then(|s| parse_shortcut(s).ok())
                                    .as_ref()
                                    == Some(shortcut)
                        }) {
                            let app_h = app.clone();
                            let _ = paste_phrase_from_island(app_h, state, phrase.id);
                        }
                    }
                })
                .build(),
        )
        .setup(move |app| {
            let app_expand = app.handle().clone();
            let app_select = app.handle().clone();
            let app_activity = app.handle().clone();
            let app_fill = app.handle().clone();
            let app_suggest = app.handle().clone();

            hook_starter.start(HookCallbacks {
                on_expanded: Some(Arc::new(move |notice| {
                    let _ = app_expand.emit("phrase-expanded", notice.abbr);
                })),
                on_suggestions_changed: Some(Arc::new(move |items| {
                    let _ = app_suggest.emit("typing-suggestions", items);
                })),
                on_fill_requested: Some(Arc::new(move |req| {
                    let state = app_fill.state::<AppState>();
                    capture_foreground_target(&app_fill, &state.mascot);
                    if let Ok(mut g) = state.pending_fill.lock() {
                        *g = Some(PendingFillJob {
                            phrase_id: req.matched.phrase_id,
                            abbr: req.matched.abbr.clone(),
                            content: req.matched.expanded_text.clone(),
                            content_html: req.matched.content_html.clone(),
                            trigger: Some(req.matched.trigger),
                            process_name: req.process_name.clone(),
                        });
                    }
                    ensure_mascot_shown(&app_fill, &state.mascot);
                    resize_mascot_window(&app_fill, &state.mascot, 420.0, 340.0, true);
                    let _ = app_fill.emit(
                        "open-fill-island",
                        FillIslandPayload {
                            abbr: req.matched.abbr,
                            fields: req.fields,
                            preview: req.matched.expanded_text,
                        },
                    );
                })),
                on_selection_candidate: Some(Arc::new(move |cand| {
                    let gen_id = bubble::bump_generation(&bubble_for_select);
                    let app_t = app_select.clone();
                    let state_t = Arc::clone(&bubble_for_select);
                    thread::Builder::new()
                        .name("meetin-sel-delay".to_string())
                        .spawn(move || {
                            // ~300 ms bekle; bu arada kullanıcı yazarsa/tıklarsa iptal olur (§4.1)
                            thread::sleep(Duration::from_millis(300));
                            if state_t.generation.load(std::sync::atomic::Ordering::SeqCst)
                                != gen_id
                            {
                                return;
                            }
                            if let Some(text) = selection::read_selected_text() {
                                bubble::show_bubble_near(
                                    &app_t, &state_t, text, cand.x, cand.y, gen_id,
                                );
                            }
                        })
                        .ok();
                })),
                on_user_activity: Some(Arc::new(move |mouse_pos| {
                    if let Some((mx, my)) = mouse_pos {
                        if let Some(bwin) = app_activity.get_webview_window(BUBBLE_LABEL) {
                            if let (Ok(pos), Ok(size)) =
                                (bwin.outer_position(), bwin.outer_size())
                            {
                                if mx >= pos.x
                                    && mx <= pos.x + size.width as i32
                                    && my >= pos.y
                                    && my <= pos.y + size.height as i32
                                {
                                    return;
                                }
                            }
                        }
                    }
                    bubble::hide_bubble(&app_activity, &bubble_for_activity);
                })),
            });

            tray::setup_tray(app.handle())?;

            // Maskot penceresini saydam, no-activate yap ve kayıtlı konumu yükle (§5.1)
            if let Some(mascot_win) = app.get_webview_window(MASCOT_LABEL) {
                let _ = mascot_win.set_background_color(Some(tauri::window::Color(0, 0, 0, 0)));
                #[cfg(windows)]
                mascot::apply_no_activate(&mascot_win, true);
            }
            if let Some(bubble_win) = app.get_webview_window(BUBBLE_LABEL) {
                let _ = bubble_win.set_background_color(Some(tauri::window::Color(0, 0, 0, 0)));
                #[cfg(windows)]
                mascot::apply_no_activate(&bubble_win, true);
            }
            if let Some(hoop_win) = app.get_webview_window(HOOP_LABEL) {
                let _ = hoop_win.set_background_color(Some(tauri::window::Color(0, 0, 0, 0)));
                #[cfg(windows)]
                mascot::apply_no_activate(&hoop_win, true);
            }
            mascot::restore_saved_position(app.handle(), &storage);
            backdrop::sync_app_windows(app.handle(), theme::read_system_theme().dark);
            mascot::start_environment_watcher(app.handle().clone());
            theme::start_theme_watcher(app.handle().clone());
            let _ = storage.purge_expired_trash(30);
            inject::clipboard::start_clipboard_watcher(Arc::clone(&storage), clipboard_cfg_for_setup);

            // Kayıtlı veya varsayılan GitHub güncelleme URL'sinden arka planda otomatik kontrol et
            let auto_check_updates = storage
                .get_setting("auto_check_updates")
                .ok()
                .flatten()
                .map(|v| v != "false")
                .unwrap_or(true);
            let upd_url = storage
                .get_setting("update_manifest_url")
                .ok()
                .flatten()
                .filter(|u| !u.trim().is_empty())
                .unwrap_or_else(|| DEFAULT_UPDATE_MANIFEST_URL.to_string());
            if auto_check_updates {
                let app_upd = app.handle().clone();
                thread::Builder::new()
                    .name("meetin-upd-check".to_string())
                    .spawn(move || {
                        // Arayüz olay dinleyicilerinin bağlanması ve hızlı açılış için kısa bekleme
                        thread::sleep(Duration::from_secs(3));
                        let portable = startup::is_portable_mode();
                        if let Ok(res) = updater::check_manifest_source(
                            &upd_url,
                            env!("CARGO_PKG_VERSION"),
                            portable,
                        ) {
                            if let Ok(mut guard) =
                                app_upd.state::<AppState>().latest_update_check.lock()
                            {
                                *guard = Some(res.clone());
                            }
                            if res.update_available
                                && res.signature_status != updater::SIG_STATUS_INVALID
                            {
                                let _ = app_upd.emit(
                                    "update-available",
                                    res.latest_version.unwrap_or_default(),
                                );
                            }
                        }
                    })
                    .ok();
            }

            // Varsayılan / kayıtlı kısayollar (§12)
            let _ = apply_global_shortcuts(app.handle(), &shortcuts_for_setup, &storage);

            Ok(())
        })
        .on_window_event(|window, event| {
            // Pencere kapatılınca arka planda tepside sessizce kal (§1.1)
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_engine_status,
            toggle_pause,
            set_fold_dotted_i,
            get_system_theme,
            get_window_backdrop_info,
            get_theme_config,
            save_theme_config,
            get_app_settings,
            set_trigger_config,
            set_suggest_enabled,
            set_shift_f3_enabled,
            set_abbr_suggest_mode,
            set_clipboard_prefs,
            list_running_apps,
            transform_clipboard_text,
            accept_typing_suggestion,
            get_startup_status,
            set_run_at_login,
            set_admin_autostart,
            get_update_env_info,
            check_for_updates,
            install_verified_update,
            set_auto_check_updates,
            set_excluded_processes,
            set_mascot_prefs,
            set_shortcuts_config,
            set_user_profile,
            set_language,
            open_data_dir,
            complete_onboarding,
            list_backups,
            create_backup_now,
            restore_backup,
            list_phrases,
            list_folders,
            read_clipboard_text,
            create_phrase,
            update_phrase,
            delete_phrase,
            restore_phrase,
            move_phrase_folder,
            parse_csv_preview,
            import_csv_phrases,
            set_mascot_mode,
            move_mascot_window,
            get_mascot_monitor_bounds,
            save_mascot_position,
            toggle_mascot_visible,
            open_studio_window,
            toggle_studio_window,
            open_studio_with_draft,
            accept_selection_bubble,
            dismiss_selection_bubble,
            open_add_from_squeeze,
            toggle_basketball_game,
            get_hoop_rim_rect,
            reposition_hoop_random,
            notify_basket_scored,
            get_phrase_app_rules,
            set_phrase_app_rules,
            submit_fill_island,
            cancel_fill_island,
            create_folder,
            delete_folder,
            update_folder,
            export_phrases_subset,
            export_phrases_subset_mee,
            list_phrase_versions,
            restore_phrase_version,
            hard_delete_phrase,
            empty_trash,
            list_clipboard_history,
            clear_clipboard_history,
            get_usage_stats,
            parse_import_file_cmd,
            bulk_create_phrases,
            export_library,
            export_mee_archive,
            quit_app,
            paste_phrase_from_island,
        ])
        .build(tauri::generate_context!())
        .expect("Meetin uygulaması oluşturulamadı");

    app.run(move |_app_handle, event| {
        if let RunEvent::Exit = event {
            // Kapanışta tutarlı yedek (§7.3)
            let _ = storage_for_exit.create_backup_now();
        }
    });
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
