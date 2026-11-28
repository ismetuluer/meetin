//! Güncelleme komutları: GitHub denetimi, imzalı kurulum ve portable EXE değişimi (§9).

use tauri::{AppHandle, Emitter, Manager, State};

use crate::db::{self, Storage};
use crate::logging;
use crate::updater::{self, UpdateCheckResult, UpdateInstallResult};
use crate::windows::startup;
use crate::{record_shutdown, AppState};

#[tauri::command]
pub(crate) fn set_auto_check_updates(
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<(), String> {
    state
        .storage
        .set_setting("auto_check_updates", if enabled { "true" } else { "false" })
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) fn get_update_env_info(state: State<'_, AppState>) -> UpdateCheckResult {
    if let Ok(guard) = state.latest_update_check.lock() {
        if let Some(cached) = guard.as_ref() {
            return cached.clone();
        }
    }
    updater::current_env_info(
        startup::is_portable_mode(),
        tauri::webview_version().is_ok(),
    )
}

/// Güncelleme bildirimi her zaman GitHub'daki en son yayından okunur. Eski sürümlerin
/// veritabanına yazdığı `update_manifest_url` ayarı (bir sürüme sabitlenmiş adres)
/// açılışta silinir; arayüzden adres değiştirmek mümkün değildir.
pub(crate) fn remove_stale_update_source(storage: &Storage) {
    match storage.delete_setting("update_manifest_url") {
        Ok(true) => logging::event(logging::Level::Warn, "update.stale_source_removed", &[]),
        Ok(false) => {}
        Err(_) => logging::event(
            logging::Level::Error,
            "update.stale_source_remove_failed",
            &[],
        ),
    }
}

/// Eklentinin güncelleyicisi. Windows'ta kurucu başlatılınca süreç `exit(0)` ile kapanır;
/// `RunEvent::Exit` gelmeyeceği için kapanış kaydı burada yapılır.
fn build_updater(app: &AppHandle) -> Result<tauri_plugin_updater::Updater, String> {
    use tauri_plugin_updater::UpdaterExt;
    let handle = app.clone();
    app.updater_builder()
        .on_before_exit(move || {
            logging::event(logging::Level::Info, "update.installer_started", &[]);
            record_shutdown(&handle.state::<AppState>().storage);
            handle.cleanup_before_exit();
        })
        .build()
        .map_err(|e| e.to_string())
}

/// Denetimi çalıştırır, sonucu günlüğe yazar, önbelleğe alır ve gerekirse olayı yayar.
pub(crate) async fn run_update_check(
    app: &AppHandle,
    trigger: &'static str,
) -> Result<UpdateCheckResult, String> {
    let checked = match build_updater(app) {
        Ok(u) => u.check().await.map_err(|e| e.to_string()),
        Err(e) => Err(e),
    };
    let found = match checked {
        Ok(found) => found,
        Err(e) => {
            logging::event(
                logging::Level::Warn,
                "update.check_failed",
                &[
                    ("trigger", logging::Field::Code(trigger)),
                    ("reason", logging::Field::Sys(e.clone())),
                ],
            );
            return Err(e);
        }
    };
    let res = updater::result_for_check(
        env!("CARGO_PKG_VERSION"),
        found.as_ref().map(|u| updater::FoundUpdate {
            version: &u.version,
            notes: u.body.as_deref(),
            raw_json: &u.raw_json,
        }),
        startup::is_portable_mode(),
        portable_dir_writable(),
        tauri::webview_version().is_ok(),
    );
    logging::event(
        logging::Level::Info,
        "update.checked",
        &[
            ("trigger", logging::Field::Code(trigger)),
            (
                "latest",
                logging::Field::Sys(res.latest_version.clone().unwrap_or_default()),
            ),
            ("available", logging::Field::Bool(res.update_available)),
        ],
    );
    if let Ok(mut guard) = app.state::<AppState>().latest_update_check.lock() {
        *guard = Some(res.clone());
    }
    if res.update_available {
        let _ = app.emit(
            "update-available",
            res.latest_version.clone().unwrap_or_default(),
        );
    }
    Ok(res)
}

#[tauri::command]
pub(crate) async fn check_for_updates(app: AppHandle) -> Result<UpdateCheckResult, String> {
    run_update_check(&app, "manual").await
}

/// İmzası doğrulanan kurulum paketini indirir ve kurar. Başarılı olursa Windows'ta kurucu
/// başlar ve Meetin kapanır (bu işlev dönmez); hata olursa günlüğe yazılıp arayüze iletilir.
#[tauri::command]
pub(crate) async fn install_verified_update(app: AppHandle) -> Result<UpdateInstallResult, String> {
    let fail = |stage: &'static str, e: String| {
        logging::event(
            logging::Level::Warn,
            "update.install_failed",
            &[
                ("stage", logging::Field::Code(stage)),
                ("reason", logging::Field::Sys(e.clone())),
            ],
        );
        e
    };
    let updater = build_updater(&app).map_err(|e| fail("build", e))?;
    let update = updater
        .check()
        .await
        .map_err(|e| fail("check", e.to_string()))?
        .ok_or_else(|| "Kurulacak güncelleme bulunamadı.".to_string())?;
    let version = update.version.clone();
    // Kurulumdan önce veritabanının güvenlik yedeği
    if let Err(e) = app.state::<AppState>().storage.create_backup_now() {
        logging::error_text(logging::Level::Warn, "db.pre_update_backup_failed", &e);
    }
    logging::event(
        logging::Level::Info,
        "update.install_started",
        &[
            ("version", logging::Field::Sys(version.clone())),
            (
                "portable",
                logging::Field::Bool(startup::is_portable_mode()),
            ),
        ],
    );
    if startup::is_portable_mode() {
        install_portable_update(&app, update)
            .await
            .map_err(|(stage, e)| fail(stage, e))?;
        return Ok(UpdateInstallResult { version });
    }
    update
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(|e| fail("download_or_verify", e.to_string()))?;
    Ok(UpdateInstallResult { version })
}

/// Portable klasörü (EXE'nin yanı) yazılabilir mi? Değilse yalnız indirme bağlantısı verilir.
fn portable_dir_writable() -> bool {
    startup::is_portable_mode() && db::ensure_writable_dir(&db::exe_dir()).is_ok()
}

/// Portable güncelleme (R2.4): imzalı ZIP eklentiyle indirilip doğrulanır, EXE yerinde
/// değiştirilir, yeni EXE başlatılır ve bu süreç kapanır. Hata olursa eski EXE yerinde kalır.
async fn install_portable_update(
    app: &AppHandle,
    update: tauri_plugin_updater::Update,
) -> Result<(), (&'static str, String)> {
    if !portable_dir_writable() {
        return Err((
            "portable_readonly",
            "Meetin klasörü yazılamıyor; yeni ZIP'i indirip elle açın.".to_string(),
        ));
    }
    let (url, signature) = updater::portable_package(&update.raw_json).ok_or((
        "portable_package",
        "Bu sürüm için imzalı portable paket yok; yeni ZIP'i indirip elle açın.".to_string(),
    ))?;
    let mut portable = update.clone();
    portable.download_url = url
        .parse()
        .map_err(|_| ("portable_package", "Paket adresi geçersiz.".to_string()))?;
    portable.signature = signature;
    // `download` imzayı uygulamadaki açık anahtarla doğrular; tutmazsa hata döner.
    let zip_bytes = portable
        .download(|_, _| {}, || {})
        .await
        .map_err(|e| ("download_or_verify", e.to_string()))?;
    let new_exe = updater::extract_exe_from_zip(&zip_bytes).map_err(|e| ("extract", e))?;
    let exe = std::env::current_exe().map_err(|e| ("swap", e.to_string()))?;
    updater::swap_portable_exe(&exe, &new_exe).map_err(|e| ("swap", e))?;
    let spawned = std::process::Command::new(&exe)
        .arg(format!(
            "{}{}",
            updater::AFTER_UPDATE_ARG,
            std::process::id()
        ))
        .spawn();
    if let Err(e) = spawned {
        // Yeni sürüm başlatılamadıysa eskisini geri koy; kullanıcı Meetin'siz kalmasın.
        let old = exe.with_file_name("Meetin.old.exe");
        let _ = std::fs::remove_file(&exe);
        let _ = std::fs::rename(&old, &exe);
        return Err(("relaunch", e.to_string()));
    }
    logging::event(logging::Level::Info, "update.portable_swapped", &[]);
    app.exit(0);
    Ok(())
}
