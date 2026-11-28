//! Tanılama komutları: günlük ve veri klasörü, arayüz hatalarının günlüğe yazılması.

use tauri::State;

use crate::logging;
use crate::AppState;

/// Kayıt günlüğü klasörünü Dosya Gezgini'nde açar (R1.4).
#[tauri::command]
pub(crate) fn open_log_dir() -> Result<(), String> {
    let dir = logging::log_dir().ok_or_else(|| "Günlük klasörü yok".to_string())?;
    #[cfg(windows)]
    {
        std::process::Command::new("explorer.exe")
            .arg(&dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(not(windows))]
    let _ = dir;
    Ok(())
}

/// Arayüzdeki yakalanmamış JS hatalarını günlüğe yazar (ileti kırpılır, tek satır).
#[tauri::command]
pub(crate) fn log_frontend_error(
    window: String,
    kind: String,
    message: String,
    source: Option<String>,
) {
    logging::event(
        logging::Level::Error,
        "ui.error",
        &[
            ("window", logging::Field::Sys(window)),
            ("kind", logging::Field::Sys(kind)),
            ("msg", logging::Field::Sys(message)),
            ("src", logging::Field::Sys(source.unwrap_or_default())),
        ],
    );
}

#[tauri::command]
pub(crate) fn open_data_dir(state: State<'_, AppState>) -> Result<(), String> {
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
