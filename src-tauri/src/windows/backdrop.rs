//! Windows 11 DWM Mica / Acrylic pencere saydamlığı ve koyu mod senkronizasyonu (Faz A).
//!
//! - Windows 11 (`build >= 22000`) üzerinde `main` penceresine gerçek DWM Mica
//!   (`DWMSBT_MAINWINDOW`, 21H2'de `DWMWA_MICA_EFFECT`) uygular.
//! - Maskot penceresi arama/ekleme/form adasına genişlediğinde `DWMSBT_TRANSIENTWINDOW`
//!   (Acrylic) uygular; koyun (kompakt) moduna döndüğünde saydam arka planı koruyacak
//!   şekilde backdrop'ı temizler.
//! - Windows 10 veya DWM bileşimi kapalı ortamlarda düz Fluent yüzeyine güvenli düşer.

use std::sync::atomic::{AtomicBool, Ordering};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, Runtime, WebviewWindow};

use crate::windows::mascot::MASCOT_LABEL;
use crate::AppState;

/// Windows 11 ilk kararlı sürüm (21H2) derleme numarası.
pub const WIN11_MIN_BUILD: u32 = 22000;
/// `DWMWA_SYSTEMBACKDROP_TYPE` (`DWMSBT_*`) desteğinin geldiği Windows 11 22H2+ derleme eşiği.
pub const WIN11_BACKDROP_TYPE_MIN_BUILD: u32 = 22523;
/// `DWMWA_USE_IMMERSIVE_DARK_MODE` (20) için Windows 10 2004+ derleme eşiği.
pub const WIN10_IMMERSIVE_DARK_MIN_BUILD: u32 = 19041;
/// Windows 11 21H2 (`22000..22523`) için eski Mica DWM özniteliği.
pub const DWMWA_MICA_EFFECT_ID: i32 = 1029;

/// Ana pencereye DWM Mica başarıyla uygulanıp uygulanmadığını izler.
static MAIN_MICA_APPLIED: AtomicBool = AtomicBool::new(false);

/// Pencere rolü (backdrop seçimi için).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WindowRole {
    Main,
    MascotCompact,
    MascotIsland,
    Bubble,
    Hoop,
}

/// DWM sistem arka plan türü.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackdropKind {
    None,
    Mica,
    MicaLegacy,
    Acrylic,
}

/// Kullanıcının tema modu tercihi.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeModeChoice {
    System,
    Light,
    Dark,
}

/// Ada yüzeyi koyuluk tercihi.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IslandSurfaceChoice {
    MatchTheme,
    AlwaysDark,
}

/// Ana pencere ve genişlemiş ada için çözümlenmiş DWM koyu mod bayrakları.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedWindowDark {
    pub main_dark: bool,
    pub island_dark: bool,
}

/// Ön yüzün mevcut pencere saydamlığı yeteneğini sorgulaması için özet DTO.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowBackdropInfo {
    pub windows_build: u32,
    pub is_windows_11: bool,
    pub main_backdrop: BackdropKind,
    pub island_backdrop: BackdropKind,
    pub mica_active: bool,
    pub acrylic_active: bool,
}

/// Verilen Windows derleme numarasının Windows 11 (`build >= 22000`) olup olmadığını döndürür.
pub fn is_windows_11_build(build: u32) -> bool {
    build >= WIN11_MIN_BUILD
}

/// `DWMWA_SYSTEMBACKDROP_TYPE` (Mica / Acrylic) desteğinin olup olmadığını döndürür.
pub fn supports_system_backdrop_type(build: u32) -> bool {
    build >= WIN11_BACKDROP_TYPE_MIN_BUILD
}

/// `DWMWA_USE_IMMERSIVE_DARK_MODE` (20) özniteliğinin desteklenip desteklenmediğini döndürür.
pub fn supports_immersive_dark_mode(build: u32) -> bool {
    build >= WIN10_IMMERSIVE_DARK_MIN_BUILD
}

/// Pencere rolü ve Windows derleme numarasına göre kullanılacak DWM backdrop türünü seçer.
pub fn backdrop_for_role(role: WindowRole, build: u32) -> BackdropKind {
    if !is_windows_11_build(build) {
        return BackdropKind::None;
    }
    match role {
        WindowRole::Main => {
            if supports_system_backdrop_type(build) {
                BackdropKind::Mica
            } else {
                BackdropKind::MicaLegacy
            }
        }
        WindowRole::MascotIsland => {
            if supports_system_backdrop_type(build) {
                BackdropKind::Acrylic
            } else {
                BackdropKind::None
            }
        }
        WindowRole::MascotCompact | WindowRole::Bubble | WindowRole::Hoop => BackdropKind::None,
    }
}

/// Maskot mod adını (`search`, `add`, `fill`, `mascot`, `pill`, `suggest`, `menu`) pencere rolüne eşler.
pub fn role_for_mascot_mode(mode: &str) -> WindowRole {
    match mode {
        "search" | "add" | "fill" => WindowRole::MascotIsland,
        _ => WindowRole::MascotCompact,
    }
}

/// Maskot boyut ve odak durumundan pencere rolünü türetir.
pub fn role_for_mascot_size(width: f64, height: f64, focusable: bool) -> WindowRole {
    if focusable && width >= 400.0 && height >= 300.0 {
        WindowRole::MascotIsland
    } else {
        WindowRole::MascotCompact
    }
}

/// Kayıtlı `theme_config` JSON metninden tema modu ve ada yüzeyi tercihini ayrıştırır.
pub fn parse_theme_preferences(config_json: Option<&str>) -> (ThemeModeChoice, IslandSurfaceChoice) {
    let Some(raw) = config_json else {
        return (ThemeModeChoice::System, IslandSurfaceChoice::MatchTheme);
    };
    let Ok(val) = serde_json::from_str::<serde_json::Value>(raw) else {
        return (ThemeModeChoice::System, IslandSurfaceChoice::MatchTheme);
    };

    let mode = match val.get("mode").and_then(|v| v.as_str()) {
        Some("light") => ThemeModeChoice::Light,
        Some("dark") => ThemeModeChoice::Dark,
        _ => ThemeModeChoice::System,
    };

    let island_surface = match val.get("islandSurface").and_then(|v| v.as_str()) {
        Some("always_dark") => IslandSurfaceChoice::AlwaysDark,
        _ => IslandSurfaceChoice::MatchTheme,
    };

    (mode, island_surface)
}

/// Tema tercihi, ada yüzeyi ayarı ve sistem koyu moduna göre DWM koyuluk değerlerini hesaplar.
pub fn resolve_window_dark(
    mode: ThemeModeChoice,
    island_surface: IslandSurfaceChoice,
    system_dark: bool,
) -> ResolvedWindowDark {
    let main_dark = match mode {
        ThemeModeChoice::Light => false,
        ThemeModeChoice::Dark => true,
        ThemeModeChoice::System => system_dark,
    };
    let island_dark = match island_surface {
        IslandSurfaceChoice::AlwaysDark => true,
        IslandSurfaceChoice::MatchTheme => main_dark,
    };
    ResolvedWindowDark {
        main_dark,
        island_dark,
    }
}

/// Mevcut Windows derleme numarasını döndürür (Windows dışı platformlarda 0).
pub fn current_windows_build() -> u32 {
    #[cfg(windows)]
    {
        win_impl::current_windows_build()
    }
    #[cfg(not(windows))]
    {
        0
    }
}

/// Mevcut sistemin backdrop yetenek bilgisini döndürür.
pub fn current_backdrop_info() -> WindowBackdropInfo {
    let build = current_windows_build();
    let is_win11 = is_windows_11_build(build);
    let main_backdrop = backdrop_for_role(WindowRole::Main, build);
    let island_backdrop = backdrop_for_role(WindowRole::MascotIsland, build);

    #[cfg(windows)]
    let composition_on = win_impl::is_dwm_composition_enabled();
    #[cfg(not(windows))]
    let composition_on = false;

    let mica_active = composition_on
        && main_backdrop != BackdropKind::None
        && (MAIN_MICA_APPLIED.load(Ordering::Relaxed) || is_win11);
    let acrylic_active = composition_on && island_backdrop == BackdropKind::Acrylic;

    WindowBackdropInfo {
        windows_build: build,
        is_windows_11: is_win11,
        main_backdrop,
        island_backdrop,
        mica_active,
        acrylic_active,
    }
}

/// Ana pencereye Windows 11 Mica + DWM koyu mod uygular; Windows 10'da düz Fluent yüzeyine düşer.
pub fn apply_main_backdrop<R: Runtime>(win: &WebviewWindow<R>, dark: bool) -> bool {
    #[cfg(windows)]
    {
        let build = current_windows_build();
        let kind = backdrop_for_role(WindowRole::Main, build);
        let Ok(hwnd) = win.hwnd() else {
            return false;
        };
        let applied = win_impl::apply_backdrop_to_hwnd(
            hwnd.0 as isize,
            kind,
            dark,
            false,
            None,
            build,
        );
        MAIN_MICA_APPLIED.store(applied, Ordering::Relaxed);
        if applied {
            let _ = win.set_background_color(Some(tauri::window::Color(0, 0, 0, 0)));
        }
        applied
    }
    #[cfg(not(windows))]
    {
        let _ = (win, dark);
        false
    }
}

/// Maskot penceresine ada modunda Acrylic uygular; kompakt moda dönerken saydamlığı koruyarak temizler.
pub fn apply_mascot_backdrop<R: Runtime>(
    win: &WebviewWindow<R>,
    role: WindowRole,
    dark: bool,
) -> bool {
    #[cfg(windows)]
    {
        let build = current_windows_build();
        let kind = backdrop_for_role(role, build);
        let Ok(hwnd) = win.hwnd() else {
            return false;
        };
        let is_island = role == WindowRole::MascotIsland && kind == BackdropKind::Acrylic;
        let round_corners = if is_island {
            Some(true)
        } else {
            Some(false)
        };
        win_impl::apply_backdrop_to_hwnd(
            hwnd.0 as isize,
            kind,
            dark,
            !is_island,
            round_corners,
            build,
        )
    }
    #[cfg(not(windows))]
    {
        let _ = (win, role, dark);
        false
    }
}

/// Veritabanındaki kayıtlı tema yapılandırmasını okuyup `main` ve (genişlemişse) `mascot`
/// pencerelerinin DWM koyu mod ve backdrop ayarlarını senkronize eder.
pub fn sync_app_windows<R: Runtime>(app: &AppHandle<R>, system_dark: bool) {
    let config_json = app
        .try_state::<AppState>()
        .and_then(|st| st.storage.get_setting("theme_config").ok().flatten());
    sync_app_windows_with_config(app, config_json.as_deref(), system_dark);
}

/// Verilen `theme_config` JSON'ı ve sistem koyuluk değeriyle pencereleri senkronize eder.
pub fn sync_app_windows_with_config<R: Runtime>(
    app: &AppHandle<R>,
    config_json: Option<&str>,
    system_dark: bool,
) {
    let (mode, island_surface) = parse_theme_preferences(config_json);
    let resolved = resolve_window_dark(mode, island_surface, system_dark);

    if let Some(main_win) = app.get_webview_window("main") {
        apply_main_backdrop(&main_win, resolved.main_dark);
    }

    let island_active = app
        .try_state::<AppState>()
        .map(|st| st.mascot.island_backdrop_active.load(Ordering::Relaxed))
        .unwrap_or(false);

    if island_active {
        if let Some(mascot_win) = app.get_webview_window(MASCOT_LABEL) {
            apply_mascot_backdrop(&mascot_win, WindowRole::MascotIsland, resolved.island_dark);
        }
    }
}

/// Ada için çözümlenmiş DWM koyu mod bilgisini hesaplar.
pub fn resolve_current_island_dark<R: Runtime>(app: &AppHandle<R>, system_dark: bool) -> bool {
    let config_json = app
        .try_state::<AppState>()
        .and_then(|st| st.storage.get_setting("theme_config").ok().flatten());
    let (mode, island_surface) = parse_theme_preferences(config_json.as_deref());
    resolve_window_dark(mode, island_surface, system_dark).island_dark
}

#[cfg(windows)]
mod win_impl {
    use super::*;
    use std::ffi::c_void;
    use std::mem::size_of;
    use std::sync::OnceLock;

    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{BOOL, HWND};
    use windows::Win32::Graphics::Dwm::{
        DwmExtendFrameIntoClientArea, DwmIsCompositionEnabled, DwmSetWindowAttribute,
        DWMSBT_MAINWINDOW, DWMSBT_NONE, DWMSBT_TRANSIENTWINDOW, DWMWA_SYSTEMBACKDROP_TYPE,
        DWMWA_USE_IMMERSIVE_DARK_MODE, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND,
        DWMWCP_ROUND, DWMWINDOWATTRIBUTE, DWM_SYSTEMBACKDROP_TYPE, DWM_WINDOW_CORNER_PREFERENCE,
    };
    use windows::Win32::System::Registry::{RegGetValueW, HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ};
    use windows::Win32::UI::Controls::MARGINS;

    fn read_hklm_string(subkey: &str, value_name: &str) -> Option<String> {
        let subkey_w: Vec<u16> = subkey.encode_utf16().chain(std::iter::once(0)).collect();
        let value_w: Vec<u16> = value_name
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let mut buf = [0u16; 64];
        let mut size = (buf.len() * size_of::<u16>()) as u32;

        let status = unsafe {
            RegGetValueW(
                HKEY_LOCAL_MACHINE,
                PCWSTR(subkey_w.as_ptr()),
                PCWSTR(value_w.as_ptr()),
                RRF_RT_REG_SZ,
                None,
                Some(buf.as_mut_ptr() as *mut c_void),
                Some(&mut size),
            )
        };
        if status.is_ok() && size >= size_of::<u16>() as u32 {
            let u16_len = (size as usize / size_of::<u16>()).min(buf.len());
            let raw = String::from_utf16_lossy(&buf[..u16_len]);
            let trimmed = raw.trim_end_matches('\0').trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
        None
    }

    pub fn current_windows_build() -> u32 {
        static CACHED_BUILD: OnceLock<u32> = OnceLock::new();
        *CACHED_BUILD.get_or_init(|| {
            const SUBKEY: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion";
            read_hklm_string(SUBKEY, "CurrentBuildNumber")
                .or_else(|| read_hklm_string(SUBKEY, "CurrentBuild"))
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(0)
        })
    }

    pub fn is_dwm_composition_enabled() -> bool {
        unsafe { DwmIsCompositionEnabled().map(|b| b.as_bool()).unwrap_or(false) }
    }

    pub fn apply_backdrop_to_hwnd(
        hwnd_raw: isize,
        kind: BackdropKind,
        dark: bool,
        reset_frame_on_none: bool,
        round_corners: Option<bool>,
        build: u32,
    ) -> bool {
        if hwnd_raw == 0 {
            return false;
        }
        let hwnd = HWND(hwnd_raw as *mut c_void);

        unsafe {
            if supports_immersive_dark_mode(build) {
                let dark_flag = BOOL::from(dark);
                let _ = DwmSetWindowAttribute(
                    hwnd,
                    DWMWA_USE_IMMERSIVE_DARK_MODE,
                    &dark_flag as *const BOOL as *const c_void,
                    size_of::<BOOL>() as u32,
                );
            }

            if !is_dwm_composition_enabled() {
                return false;
            }

            if let Some(round) = round_corners {
                if is_windows_11_build(build) {
                    let pref: DWM_WINDOW_CORNER_PREFERENCE = if round {
                        DWMWCP_ROUND
                    } else {
                        DWMWCP_DONOTROUND
                    };
                    let _ = DwmSetWindowAttribute(
                        hwnd,
                        DWMWA_WINDOW_CORNER_PREFERENCE,
                        &pref as *const DWM_WINDOW_CORNER_PREFERENCE as *const c_void,
                        size_of::<DWM_WINDOW_CORNER_PREFERENCE>() as u32,
                    );
                }
            }

            match kind {
                BackdropKind::Mica => {
                    let margins = MARGINS {
                        cxLeftWidth: -1,
                        cxRightWidth: -1,
                        cyTopHeight: -1,
                        cyBottomHeight: -1,
                    };
                    let _ = DwmExtendFrameIntoClientArea(hwnd, &margins);
                    let backdrop: DWM_SYSTEMBACKDROP_TYPE = DWMSBT_MAINWINDOW;
                    DwmSetWindowAttribute(
                        hwnd,
                        DWMWA_SYSTEMBACKDROP_TYPE,
                        &backdrop as *const DWM_SYSTEMBACKDROP_TYPE as *const c_void,
                        size_of::<DWM_SYSTEMBACKDROP_TYPE>() as u32,
                    )
                    .is_ok()
                }
                BackdropKind::MicaLegacy => {
                    let margins = MARGINS {
                        cxLeftWidth: -1,
                        cxRightWidth: -1,
                        cyTopHeight: -1,
                        cyBottomHeight: -1,
                    };
                    let _ = DwmExtendFrameIntoClientArea(hwnd, &margins);
                    let enable: u32 = 1;
                    DwmSetWindowAttribute(
                        hwnd,
                        DWMWINDOWATTRIBUTE(DWMWA_MICA_EFFECT_ID),
                        &enable as *const u32 as *const c_void,
                        size_of::<u32>() as u32,
                    )
                    .is_ok()
                }
                BackdropKind::Acrylic => {
                    let margins = MARGINS {
                        cxLeftWidth: -1,
                        cxRightWidth: -1,
                        cyTopHeight: -1,
                        cyBottomHeight: -1,
                    };
                    let _ = DwmExtendFrameIntoClientArea(hwnd, &margins);
                    let backdrop: DWM_SYSTEMBACKDROP_TYPE = DWMSBT_TRANSIENTWINDOW;
                    DwmSetWindowAttribute(
                        hwnd,
                        DWMWA_SYSTEMBACKDROP_TYPE,
                        &backdrop as *const DWM_SYSTEMBACKDROP_TYPE as *const c_void,
                        size_of::<DWM_SYSTEMBACKDROP_TYPE>() as u32,
                    )
                    .is_ok()
                }
                BackdropKind::None => {
                    if supports_system_backdrop_type(build) {
                        let backdrop: DWM_SYSTEMBACKDROP_TYPE = DWMSBT_NONE;
                        let _ = DwmSetWindowAttribute(
                            hwnd,
                            DWMWA_SYSTEMBACKDROP_TYPE,
                            &backdrop as *const DWM_SYSTEMBACKDROP_TYPE as *const c_void,
                            size_of::<DWM_SYSTEMBACKDROP_TYPE>() as u32,
                        );
                    } else if is_windows_11_build(build) {
                        let disable: u32 = 0;
                        let _ = DwmSetWindowAttribute(
                            hwnd,
                            DWMWINDOWATTRIBUTE(DWMWA_MICA_EFFECT_ID),
                            &disable as *const u32 as *const c_void,
                            size_of::<u32>() as u32,
                        );
                    }
                    if reset_frame_on_none {
                        let zero_margins = MARGINS {
                            cxLeftWidth: 0,
                            cxRightWidth: 0,
                            cyTopHeight: 0,
                            cyBottomHeight: 0,
                        };
                        let _ = DwmExtendFrameIntoClientArea(hwnd, &zero_margins);
                    }
                    false
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_11_build_detection() {
        assert!(!is_windows_11_build(0));
        assert!(!is_windows_11_build(19045)); // Windows 10 22H2
        assert!(!is_windows_11_build(21999));
        assert!(is_windows_11_build(22000)); // Windows 11 21H2
        assert!(is_windows_11_build(22621)); // Windows 11 22H2
        assert!(is_windows_11_build(26100)); // Windows 11 24H2
    }

    #[test]
    fn backdrop_selection_by_window_role_and_os_build() {
        // Windows 10 -> her rol düz Fluent yüzeyine düşer
        for role in [
            WindowRole::Main,
            WindowRole::MascotCompact,
            WindowRole::MascotIsland,
            WindowRole::Bubble,
            WindowRole::Hoop,
        ] {
            assert_eq!(backdrop_for_role(role, 19045), BackdropKind::None);
        }

        // Windows 11 21H2 (22000): main için legacy Mica, ada güvenli fallback
        assert_eq!(
            backdrop_for_role(WindowRole::Main, 22000),
            BackdropKind::MicaLegacy
        );
        assert_eq!(
            backdrop_for_role(WindowRole::MascotIsland, 22000),
            BackdropKind::None
        );
        assert_eq!(
            backdrop_for_role(WindowRole::MascotCompact, 22000),
            BackdropKind::None
        );

        // Windows 11 22H2+ (22621 / 26100): main -> Mica, ada -> Acrylic, diğerleri -> None
        for build in [22523, 22621, 22631, 26100] {
            assert_eq!(backdrop_for_role(WindowRole::Main, build), BackdropKind::Mica);
            assert_eq!(
                backdrop_for_role(WindowRole::MascotIsland, build),
                BackdropKind::Acrylic
            );
            assert_eq!(
                backdrop_for_role(WindowRole::MascotCompact, build),
                BackdropKind::None
            );
            assert_eq!(
                backdrop_for_role(WindowRole::Bubble, build),
                BackdropKind::None
            );
            assert_eq!(
                backdrop_for_role(WindowRole::Hoop, build),
                BackdropKind::None
            );
        }
    }

    #[test]
    fn mascot_mode_and_size_role_mapping() {
        assert_eq!(role_for_mascot_mode("search"), WindowRole::MascotIsland);
        assert_eq!(role_for_mascot_mode("add"), WindowRole::MascotIsland);
        assert_eq!(role_for_mascot_mode("fill"), WindowRole::MascotIsland);

        assert_eq!(role_for_mascot_mode("mascot"), WindowRole::MascotCompact);
        assert_eq!(role_for_mascot_mode("pill"), WindowRole::MascotCompact);
        assert_eq!(role_for_mascot_mode("suggest"), WindowRole::MascotCompact);
        assert_eq!(role_for_mascot_mode("menu"), WindowRole::MascotCompact);

        assert_eq!(
            role_for_mascot_size(560.0, 380.0, true),
            WindowRole::MascotIsland
        );
        assert_eq!(
            role_for_mascot_size(460.0, 410.0, true),
            WindowRole::MascotIsland
        );
        assert_eq!(
            role_for_mascot_size(420.0, 340.0, true),
            WindowRole::MascotIsland
        );
        assert_eq!(
            role_for_mascot_size(100.0, 100.0, false),
            WindowRole::MascotCompact
        );
        assert_eq!(
            role_for_mascot_size(224.0, 286.0, true),
            WindowRole::MascotCompact
        );
        assert_eq!(
            role_for_mascot_size(290.0, 96.0, false),
            WindowRole::MascotCompact
        );
    }

    #[test]
    fn theme_dark_mode_mapping() {
        // Sistem modu açık/koyu takibi
        assert_eq!(
            resolve_window_dark(
                ThemeModeChoice::System,
                IslandSurfaceChoice::MatchTheme,
                false
            ),
            ResolvedWindowDark {
                main_dark: false,
                island_dark: false,
            }
        );
        assert_eq!(
            resolve_window_dark(
                ThemeModeChoice::System,
                IslandSurfaceChoice::MatchTheme,
                true
            ),
            ResolvedWindowDark {
                main_dark: true,
                island_dark: true,
            }
        );

        // Açık tema + ada her zaman koyu
        assert_eq!(
            resolve_window_dark(
                ThemeModeChoice::Light,
                IslandSurfaceChoice::AlwaysDark,
                true
            ),
            ResolvedWindowDark {
                main_dark: false,
                island_dark: true,
            }
        );

        // Koyu tema zorlama
        assert_eq!(
            resolve_window_dark(
                ThemeModeChoice::Dark,
                IslandSurfaceChoice::MatchTheme,
                false
            ),
            ResolvedWindowDark {
                main_dark: true,
                island_dark: true,
            }
        );

        // JSON ayrıştırma
        assert_eq!(
            parse_theme_preferences(None),
            (ThemeModeChoice::System, IslandSurfaceChoice::MatchTheme)
        );
        assert_eq!(
            parse_theme_preferences(Some("bozuk-json")),
            (ThemeModeChoice::System, IslandSurfaceChoice::MatchTheme)
        );
        assert_eq!(
            parse_theme_preferences(Some(
                r#"{"mode":"light","islandSurface":"always_dark"}"#
            )),
            (ThemeModeChoice::Light, IslandSurfaceChoice::AlwaysDark)
        );
        assert_eq!(
            parse_theme_preferences(Some(r#"{"mode":"dark","islandSurface":"match_theme"}"#)),
            (ThemeModeChoice::Dark, IslandSurfaceChoice::MatchTheme)
        );
    }
}
