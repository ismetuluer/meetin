//! Genişletme motoru komutları: durum, duraklatma, tetik ve hariç tutulan uygulamalar.

#[cfg(windows)]
use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};

use crate::hook::TriggerConfig;
#[cfg(windows)]
use crate::matcher::case::tr_lower;
use crate::windows::tray;
use crate::AppState;

#[derive(Debug, Serialize, Deserialize)]
pub struct EngineStatus {
    pub paused: bool,
    pub fold_dotted_i: bool,
    pub active_phrase_count: usize,
    pub data_dir: String,
    pub version: String,
}

#[tauri::command]
pub(crate) fn get_engine_status(state: State<'_, AppState>) -> Result<EngineStatus, String> {
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
pub(crate) fn toggle_pause(app: AppHandle, state: State<'_, AppState>) -> bool {
    let paused = state.hook.toggle_paused();
    tray::update_tray_paused(&app, paused);
    let _ = app.emit("engine-status-changed", paused);
    paused
}

#[tauri::command]
pub(crate) fn set_fold_dotted_i(state: State<'_, AppState>, enabled: bool) -> Result<(), String> {
    state
        .storage
        .set_setting("fold_dotted_i", if enabled { "true" } else { "false" })
        .map_err(|e| e.to_string())?;
    state.hook.set_fold_dotted_i(enabled);
    Ok(())
}

#[tauri::command]
pub(crate) fn set_suggest_enabled(state: State<'_, AppState>, enabled: bool) -> Result<(), String> {
    state
        .storage
        .set_setting("suggest_enabled", if enabled { "true" } else { "false" })
        .map_err(|e| e.to_string())?;
    state.hook.set_suggest_enabled(enabled);
    Ok(())
}

#[tauri::command]
pub(crate) fn set_shift_f3_enabled(
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<(), String> {
    state
        .storage
        .set_setting("shift_f3_enabled", if enabled { "true" } else { "false" })
        .map_err(|e| e.to_string())?;
    state.hook.set_shift_f3_enabled(enabled);
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunningAppInfo {
    pub process_name: String,
    pub window_title: String,
}

#[tauri::command]
pub(crate) fn list_running_apps() -> Vec<RunningAppInfo> {
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
pub(crate) fn accept_typing_suggestion(state: State<'_, AppState>, phrase_id: i64) {
    state.hook.accept_suggestion(phrase_id);
}

#[tauri::command]
pub(crate) fn set_trigger_config(
    state: State<'_, AppState>,
    config: TriggerConfig,
) -> Result<(), String> {
    let json = serde_json::to_string(&config).map_err(|e| e.to_string())?;
    state
        .storage
        .set_setting("trigger_config", &json)
        .map_err(|e| e.to_string())?;
    state.hook.set_trigger_config(config);
    Ok(())
}

#[tauri::command]
pub(crate) fn set_excluded_processes(
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
