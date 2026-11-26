//! Sistem tepsisi simgesi ve menüsü (§5.1.1, M1.8).

use tauri::image::Image;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, Runtime};

use crate::windows::{backdrop, theme};
use crate::AppState;

pub const TRAY_ID: &str = "meetin-tray";
const MENU_OPEN: &str = "tray_open";
const MENU_PAUSE: &str = "tray_pause";
const MENU_QUIT: &str = "tray_quit";

const ICON_ACTIVE: &[u8] = include_bytes!("../../icons/tray.png");
const ICON_PAUSED: &[u8] = include_bytes!("../../icons/tray-paused.png");

pub fn setup_tray<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let open_item = MenuItem::with_id(app, MENU_OPEN, "Meetin'i Aç", true, None::<&str>)?;
    let pause_item = MenuItem::with_id(app, MENU_PAUSE, "Duraklat", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, MENU_QUIT, "Çıkış", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open_item, &pause_item, &quit_item])?;

    let icon = Image::from_bytes(ICON_ACTIVE)?;

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .tooltip("Meetin — Etkin")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| match event.id().as_ref() {
            MENU_OPEN => {
                show_main_window(app);
            }
            MENU_PAUSE => {
                let state = app.state::<AppState>();
                let paused = state.hook.toggle_paused();
                update_tray_paused(app, paused);
                let _ = app.emit("engine-status-changed", paused);
            }
            MENU_QUIT => {
                let state = app.state::<AppState>();
                let _ = state.storage.create_backup_now();
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

pub fn show_main_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(win) = app.get_webview_window("main") {
        backdrop::sync_app_windows(app, theme::read_system_theme().dark);
        let _ = win.show();
        let _ = win.unminimize();
        let _ = win.set_focus();
    }
}

pub fn update_tray_paused<R: Runtime>(app: &AppHandle<R>, paused: bool) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let bytes = if paused { ICON_PAUSED } else { ICON_ACTIVE };
        if let Ok(img) = Image::from_bytes(bytes) {
            let _ = tray.set_icon(Some(img));
        }
        let tip = if paused {
            "Meetin — Duraklatıldı"
        } else {
            "Meetin — Etkin"
        };
        let _ = tray.set_tooltip(Some(tip));

        if let (Ok(open_i), Ok(pause_i), Ok(quit_i)) = (
            MenuItem::with_id(app, MENU_OPEN, "Meetin'i Aç", true, None::<&str>),
            MenuItem::with_id(
                app,
                MENU_PAUSE,
                if paused { "Sürdür" } else { "Duraklat" },
                true,
                None::<&str>,
            ),
            MenuItem::with_id(app, MENU_QUIT, "Çıkış", true, None::<&str>),
        ) {
            if let Ok(menu) = Menu::with_items(app, &[&open_i, &pause_i, &quit_i]) {
                let _ = tray.set_menu(Some(menu));
            }
        }
    }
}
