//! Meetin çekirdek kütüphanesi (§10).

mod commands;
pub mod db;
pub mod hook;
pub mod inject;
pub mod io;
pub mod logging;
mod webview2;

/// Açılışta, arayüz kurulmadan önce gösterilen yerel Windows iletişim kutuları.
pub(crate) mod native_dialog {
    #[cfg(windows)]
    fn show(text: &str, flags: windows::Win32::UI::WindowsAndMessaging::MESSAGEBOX_STYLE) -> i32 {
        use windows::core::{w, PCWSTR};
        use windows::Win32::UI::WindowsAndMessaging::MessageBoxW;
        let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
        unsafe { MessageBoxW(None, PCWSTR(wide.as_ptr()), w!("Meetin"), flags).0 }
    }

    /// Evet/Hayır sorusu; Evet ise `true`.
    pub fn ask(text: &str) -> bool {
        #[cfg(windows)]
        {
            use windows::Win32::UI::WindowsAndMessaging::{IDYES, MB_ICONQUESTION, MB_YESNO};
            show(text, MB_YESNO | MB_ICONQUESTION) == IDYES.0
        }
        #[cfg(not(windows))]
        {
            let _ = text;
            false
        }
    }

    pub fn error(text: &str) {
        #[cfg(windows)]
        {
            use windows::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK};
            show(text, MB_OK | MB_ICONERROR);
        }
        #[cfg(not(windows))]
        let _ = text;
    }
}
pub mod matcher;
pub mod selection;
pub mod updater;
pub mod vars;
pub mod windows;

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager, RunEvent};
use tauri_plugin_global_shortcut::ShortcutState;

use crate::commands::island::paste_phrase_from_island;
use crate::commands::shortcuts::{apply_global_shortcuts, parse_shortcut};
use crate::commands::update::{remove_stale_update_source, run_update_check};
use crate::db::Storage;
use crate::hook::{HookCallbacks, HookController};
use crate::inject::clipboard::ClipboardWatcherConfig;
use crate::updater::UpdateCheckResult;
use crate::vars::FormField;
use crate::windows::backdrop;
use crate::windows::bubble::{self, BubbleState, BUBBLE_LABEL};
use crate::windows::hoop::{HoopState, HOOP_LABEL};
use crate::windows::mascot::{
    self, capture_foreground_target, ensure_mascot_shown, resize_mascot_window,
    toggle_user_visibility, MascotWindowState, MASCOT_LABEL,
};
use crate::windows::startup;
use crate::windows::theme;
use crate::windows::tray;

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

/// Kapanışta yapılan kayıt işleri: tutarlı yedek (§7.3), kanca istatistiği, çıkış kaydı.
/// Hem normal çıkışta hem güncelleme kurucusu başlatılmadan hemen önce çağrılır.
fn record_shutdown(storage: &Storage) {
    if let Err(e) = storage.create_backup_now() {
        logging::error_text(logging::Level::Warn, "db.exit_backup_failed", &e);
    }
    let hs = hook::stats();
    logging::event(
        logging::Level::Info,
        "hook.stats",
        &[
            ("calls", logging::Field::Num(hs.calls as i64)),
            ("slow_over_1ms", logging::Field::Num(hs.slow_calls as i64)),
            ("max_us", logging::Field::Num(hs.max_micros as i64)),
        ],
    );
    logging::log_exit();
}

pub fn run() {
    let data_dir = db::runtime_data_dir();
    // Salt okunur USB ya da izinsiz klasör: çökmek yerine anlaşılır uyarı (§14)
    if db::ensure_writable_dir(&data_dir).is_err() {
        native_dialog::error(&format!(
            "Meetin veri klasörüne yazamıyor:\n{}\n\nMeetin klasörünü yazılabilir bir yere \
             (ör. Masaüstü ya da USB belleğin yazılabilir bir klasörü) taşıyıp yeniden deneyin.",
            data_dir.display()
        ));
        return;
    }
    // Kayıt günlüğü her şeyden önce: açılıştaki hatalar da yakalansın (R1.4)
    logging::init(&data_dir);
    // Portable güncelleme sonrası: eski süreç kapanmadan tek örnek denetimi yeni süreci
    // kapatmasın diye önce onu bekle, sonra kenara alınan eski EXE'yi sil (R2.4).
    if let Some(old_pid) = updater::after_update_pid(std::env::args()) {
        #[cfg(windows)]
        updater::wait_for_process_exit(old_pid, 15_000);
        let _ = old_pid;
        logging::event(logging::Level::Info, "update.portable_applied", &[]);
    }
    if startup::is_portable_mode() {
        if let Ok(exe) = std::env::current_exe() {
            match updater::remove_old_exe(&exe) {
                Ok(_) => {}
                Err(e) => {
                    logging::error_text(logging::Level::Warn, "update.old_exe_remove_failed", &e)
                }
            }
        }
    }
    let portable = startup::is_portable_mode();
    logging::log_startup(portable);

    // WebView2 yoksa arayüz açılamaz: kullanıcıya sorup Microsoft'tan kur (R1.5)
    if !webview2::ensure_installed() {
        logging::event(logging::Level::Error, "webview2.missing_exit", &[]);
        return;
    }

    // Portable: veri kullanıcı klasöründe, yedekler EXE yanındaki data\backups'ta (R1.5)
    let portable_backups = if portable {
        let dir = db::portable_backups_dir(&db::exe_dir());
        match db::ensure_writable_dir(&dir) {
            Ok(()) => Some(dir),
            Err(e) => {
                logging::error_text(logging::Level::Warn, "portable.backups_not_writable", &e);
                None
            }
        }
    } else {
        None
    };

    // Portable başka bir bilgisayarda: kullanıcı klasöründe veri yoksa en son yedeği öner
    if let Some(cand) = portable_backups
        .as_deref()
        .and_then(|dir| db::portable_restore_candidate(&data_dir, dir))
    {
        let text = format!(
            "Bu bilgisayarda Meetin verisi yok.\n\nTaşınabilir klasördeki en son yedek \
             ({}) geri yüklensin mi?",
            cand.filename
        );
        if native_dialog::ask(&text) {
            match db::copy_database(
                std::path::Path::new(&cand.path),
                &data_dir.join("meetin.db"),
            ) {
                Ok(()) => {
                    logging::event(logging::Level::Info, "portable.restored_from_backup", &[])
                }
                Err(e) => {
                    logging::error_text(logging::Level::Error, "portable.restore_failed", &e);
                    native_dialog::error(
                        "Yedek geri yüklenemedi. Meetin boş bir kütüphaneyle açılacak.",
                    );
                }
            }
        }
    }

    let db_path = data_dir.join("meetin.db");
    let storage = Arc::new(
        Storage::open(&db_path)
            .expect("Meetin veritabanı açılamadı")
            .with_backups_dir(portable_backups),
    );

    // Günün ilk açılışında otomatik yedek (§7.3)
    if let Err(e) = storage.ensure_daily_backup_today() {
        logging::error_text(logging::Level::Warn, "db.daily_backup_failed", &e);
    }

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
        .plugin(tauri_plugin_updater::Builder::new().build())
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
                    } else if parse_shortcut(&sc_cfg.toggle_mascot).ok().as_ref() == Some(shortcut)
                    {
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
                            if let (Ok(pos), Ok(size)) = (bwin.outer_position(), bwin.outer_size())
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
            inject::clipboard::start_clipboard_watcher(
                Arc::clone(&storage),
                clipboard_cfg_for_setup,
            );

            // Kayıtlı veya varsayılan GitHub güncelleme URL'sinden arka planda otomatik kontrol et
            let auto_check_updates = storage
                .get_setting("auto_check_updates")
                .ok()
                .flatten()
                .map(|v| v != "false")
                .unwrap_or(true);
            remove_stale_update_source(&storage);
            if auto_check_updates {
                let app_upd = app.handle().clone();
                thread::Builder::new()
                    .name("meetin-upd-check".to_string())
                    .spawn(move || {
                        // Açılışta (kısa beklemeyle) ve sonra 24 saatte bir denetlenir (§9).
                        // Olayı kaçıran pencere sonucu `get_update_env_info` ile kendisi sorar.
                        thread::sleep(Duration::from_secs(3));
                        let mut trigger = "startup";
                        loop {
                            let _ =
                                tauri::async_runtime::block_on(run_update_check(&app_upd, trigger));
                            trigger = "daily";
                            thread::sleep(Duration::from_secs(24 * 60 * 60));
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
            commands::engine::get_engine_status,
            commands::engine::toggle_pause,
            commands::engine::set_fold_dotted_i,
            commands::appearance::get_system_theme,
            commands::appearance::get_window_backdrop_info,
            commands::appearance::get_theme_config,
            commands::appearance::save_theme_config,
            commands::settings::get_app_settings,
            commands::engine::set_trigger_config,
            commands::engine::set_suggest_enabled,
            commands::engine::set_shift_f3_enabled,
            commands::settings::set_abbr_suggest_mode,
            commands::clipboard::set_clipboard_prefs,
            commands::engine::list_running_apps,
            commands::clipboard::transform_clipboard_text,
            commands::engine::accept_typing_suggestion,
            commands::settings::get_startup_status,
            commands::settings::set_run_at_login,
            commands::settings::set_admin_autostart,
            commands::update::get_update_env_info,
            commands::update::check_for_updates,
            commands::update::install_verified_update,
            commands::update::set_auto_check_updates,
            commands::engine::set_excluded_processes,
            commands::settings::set_mascot_prefs,
            commands::shortcuts::set_shortcuts_config,
            commands::settings::set_user_profile,
            commands::settings::set_language,
            commands::diagnostics::open_data_dir,
            commands::diagnostics::open_log_dir,
            commands::diagnostics::log_frontend_error,
            commands::settings::complete_onboarding,
            commands::backup::list_backups,
            commands::backup::create_backup_now,
            commands::backup::restore_backup,
            commands::phrases::list_phrases,
            commands::folders::list_folders,
            commands::clipboard::read_clipboard_text,
            commands::phrases::create_phrase,
            commands::phrases::update_phrase,
            commands::phrases::delete_phrase,
            commands::phrases::restore_phrase,
            commands::phrases::move_phrase_folder,
            commands::import_export::parse_csv_preview,
            commands::import_export::import_csv_phrases,
            commands::mascot::set_mascot_mode,
            commands::mascot::move_mascot_window,
            commands::mascot::get_mascot_monitor_layout,
            commands::mascot::save_mascot_position,
            commands::mascot::toggle_mascot_visible,
            commands::mascot::open_studio_window,
            commands::mascot::toggle_studio_window,
            commands::mascot::open_studio_with_draft,
            commands::mascot::accept_selection_bubble,
            commands::mascot::dismiss_selection_bubble,
            commands::mascot::open_add_from_squeeze,
            commands::hoop::toggle_basketball_game,
            commands::hoop::get_hoop_rim_rect,
            commands::hoop::reposition_hoop_random,
            commands::hoop::notify_basket_scored,
            commands::phrases::get_phrase_app_rules,
            commands::phrases::set_phrase_app_rules,
            commands::island::submit_fill_island,
            commands::island::cancel_fill_island,
            commands::folders::create_folder,
            commands::folders::delete_folder,
            commands::folders::update_folder,
            commands::import_export::export_phrases_subset,
            commands::import_export::export_phrases_subset_mee,
            commands::phrases::list_phrase_versions,
            commands::phrases::restore_phrase_version,
            commands::phrases::hard_delete_phrase,
            commands::phrases::empty_trash,
            commands::clipboard::list_clipboard_history,
            commands::clipboard::clear_clipboard_history,
            commands::phrases::get_usage_stats,
            commands::import_export::parse_import_file_cmd,
            commands::import_export::bulk_create_phrases,
            commands::import_export::export_library,
            commands::import_export::export_mee_archive,
            commands::mascot::quit_app,
            commands::island::paste_phrase_from_island,
        ])
        .build(tauri::generate_context!())
        .expect("Meetin uygulaması oluşturulamadı");

    app.run(move |_app_handle, event| {
        if let RunEvent::Exit = event {
            record_shutdown(&storage_for_exit);
        }
    });
}
