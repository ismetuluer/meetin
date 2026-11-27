//! Windows düşük seviye klavye/fare kancası (`WH_KEYBOARD_LL` + `WH_MOUSE_LL`, §3.1, §7.4, M1.5).
//!
//! - Kanca ayrı iş parçacığında mesaj döngüsüyle çalışır.
//! - Kanca geri çağrısı bloklamaz (< 1 ms); ağır iş (enjeksiyon, DB istatistik)
//!   kanal üzerinden çalışan iş parçacığına aktarılır.
//! - Kendi enjekte ettiğimiz olaylar `INJECTED_EXTRA_INFO` imzasıyla yok sayılır.
//! - Tuş vuruşları ASLA diske veya loga yazılmaz.

pub mod buffer;
pub mod focus;
pub mod gate;

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex, OnceLock, RwLock};
use std::thread;
use std::time::Instant;

use chrono::Local;
use serde::{Deserialize, Serialize};

use crate::db::Storage;
use crate::inject;
use crate::matcher::case::{normalize_abbr, tr_lower};
use crate::matcher::{is_trigger_char, ActiveContext, MatchResult, Matcher, SuggestionItem};
use crate::selection;
use crate::vars::{self, FormField, ResolveContext};

use self::buffer::KeyBuffer;
use self::focus::FocusTracker;
use self::gate::{InjectGate, QueuedKey};

/// Kendi `SendInput` olaylarımızı tanımak için `dwExtraInfo` imzası.
pub const INJECTED_EXTRA_INFO: usize = 0x4D45_4554;

/// Genişletme sırasında bekletilip yeniden gönderilen kullanıcı tuşlarının imzası
/// (bkz. [`gate`]). Kanca bunları normal yazı gibi işler, yeniden kuyruğa almaz.
pub const REPLAYED_EXTRA_INFO: usize = 0x4D45_5250;

/// Varsayılan dışlanan programlar (boş başlar; parola alanları UIA ile sessizce korunur).
pub const DEFAULT_EXCLUDED_PROCESSES: &[&str] = &[];

/// Seçim balonunun gizlendiği ek programlar (§4.1).
pub const DEFAULT_BUBBLE_HIDDEN_PROCESSES: &[&str] = &["photoshop.exe", "meetin.exe"];

/// Genişletme tamamlandığında üst katmana bildirilen olay.
#[derive(Debug, Clone)]
pub struct ExpansionNotice {
    pub phrase_id: i64,
    pub abbr: String,
}

/// Fareyle metin seçimi algılandığında bildirilen aday.
#[derive(Debug, Clone)]
pub struct SelectionCandidate {
    pub x: i32,
    pub y: i32,
    pub process_name: Option<String>,
}

/// `{alan:...}` içeren ifadede form doldurma istendiğinde bildirilen istek.
#[derive(Debug, Clone)]
pub struct FillRequest {
    pub matched: MatchResult,
    pub fields: Vec<FormField>,
    pub process_name: Option<String>,
}

pub type ExpandedCb = Arc<dyn Fn(ExpansionNotice) + Send + Sync>;
pub type SelectionCandidateCb = Arc<dyn Fn(SelectionCandidate) + Send + Sync>;
pub type UserActivityCb = Arc<dyn Fn(Option<(i32, i32)>) + Send + Sync>;
pub type FillRequestedCb = Arc<dyn Fn(FillRequest) + Send + Sync>;
pub type SuggestionsChangedCb = Arc<dyn Fn(Vec<SuggestionItem>) + Send + Sync>;

/// Kanca geri çağrıları.
#[derive(Default)]
pub struct HookCallbacks {
    pub on_expanded: Option<ExpandedCb>,
    pub on_selection_candidate: Option<SelectionCandidateCb>,
    /// `Some((x, y))` fare tıklaması, `None` klavye etkinliğidir.
    pub on_user_activity: Option<UserActivityCb>,
    pub on_fill_requested: Option<FillRequestedCb>,
    pub on_suggestions_changed: Option<SuggestionsChangedCb>,
}

/// Tetik karakterleri yapılandırması (M6.4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TriggerConfig {
    pub space: bool,
    pub enter: bool,
    pub tab: bool,
    pub punctuation: bool,
}

impl Default for TriggerConfig {
    fn default() -> Self {
        Self {
            space: true,
            enter: true,
            tab: true,
            punctuation: true,
        }
    }
}

impl TriggerConfig {
    pub fn matches(&self, c: char) -> bool {
        match c {
            ' ' => self.space,
            '\n' | '\r' => self.enter,
            '\t' => self.tab,
            '.' | ',' | ';' | ':' | '!' | '?' | ')' | '"' => self.punctuation,
            _ => false,
        }
    }
}

/// Kanca durumunun paylaşılan denetleyicisi.
pub struct HookController {
    paused: Arc<AtomicBool>,
    suggest_enabled: Arc<AtomicBool>,
    shift_f3_enabled: Arc<AtomicBool>,
    matcher: Arc<RwLock<Matcher>>,
    excluded: Arc<RwLock<Vec<String>>>,
    trigger_config: Arc<RwLock<TriggerConfig>>,
    tx: mpsc::Sender<WorkerJob>,
}

impl HookController {
    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::Relaxed)
    }

    pub fn set_paused(&self, paused: bool) {
        self.paused.store(paused, Ordering::Relaxed);
    }

    pub fn toggle_paused(&self) -> bool {
        let prev = self.paused.fetch_xor(true, Ordering::Relaxed);
        !prev
    }

    pub fn reload_matcher(&self, storage: &Storage) -> Result<usize, crate::db::DbError> {
        let entries = storage.load_matcher_entries()?;
        let count = entries.len();
        if let Ok(mut m) = self.matcher.write() {
            m.rebuild(entries);
        }
        Ok(count)
    }

    pub fn set_fold_dotted_i(&self, fold: bool) {
        if let Ok(mut m) = self.matcher.write() {
            m.set_fold_dotted_i(fold);
        }
    }

    pub fn set_excluded_processes(&self, list: Vec<String>) {
        if let Ok(mut ex) = self.excluded.write() {
            *ex = list.into_iter().map(|s| tr_lower(&s)).collect();
        }
    }

    pub fn get_excluded_processes(&self) -> Vec<String> {
        self.excluded
            .read()
            .map(|ex| ex.clone())
            .unwrap_or_default()
    }

    pub fn set_trigger_config(&self, cfg: TriggerConfig) {
        if let Ok(mut tc) = self.trigger_config.write() {
            *tc = cfg;
        }
    }

    pub fn get_trigger_config(&self) -> TriggerConfig {
        self.trigger_config
            .read()
            .map(|tc| tc.clone())
            .unwrap_or_default()
    }

    pub fn is_suggest_enabled(&self) -> bool {
        self.suggest_enabled.load(Ordering::Relaxed)
    }

    pub fn set_suggest_enabled(&self, enabled: bool) {
        self.suggest_enabled.store(enabled, Ordering::Relaxed);
        if !enabled {
            let _ = self.tx.send(WorkerJob::SuggestionsChanged {
                items: Vec::new(),
                check_password: false,
            });
        }
    }

    pub fn accept_suggestion(&self, phrase_id: i64) {
        let _ = self.tx.send(WorkerJob::AcceptSuggestion { phrase_id });
    }

    pub fn is_shift_f3_enabled(&self) -> bool {
        self.shift_f3_enabled.load(Ordering::Relaxed)
    }

    pub fn set_shift_f3_enabled(&self, enabled: bool) {
        self.shift_f3_enabled.store(enabled, Ordering::Relaxed);
    }
}

/// İşlem adının dışlanan programlar listesinde olup olmadığını denetler.
pub fn is_process_excluded(process_name: Option<&str>, excluded: &[String]) -> bool {
    let Some(name) = process_name else {
        return false;
    };
    let lower = tr_lower(name);
    excluded
        .iter()
        .any(|e| e.eq_ignore_ascii_case(name) || e == &lower)
}

/// Seçim balonunun bu programda gizlenip gizlenmeyeceğini denetler (§4.1).
pub fn is_bubble_hidden_process(process_name: Option<&str>, excluded: &[String]) -> bool {
    let Some(name) = process_name else {
        return false;
    };
    let lower = tr_lower(name);
    if excluded
        .iter()
        .any(|e| e.eq_ignore_ascii_case(name) || e == &lower)
    {
        return true;
    }
    DEFAULT_BUBBLE_HIDDEN_PROCESSES
        .iter()
        .any(|&p| p.eq_ignore_ascii_case(name) || p == lower)
}

#[derive(Debug, Clone)]
struct LastExpansion {
    hwnd: isize,
    delete_len: usize,
    original_abbr: String,
}

#[derive(Debug)]
enum WorkerJob {
    Expand {
        matched: MatchResult,
        hwnd: isize,
        process_name: Option<String>,
    },
    UndoExpansion {
        delete_len: usize,
        original_abbr: String,
    },
    SelectionCandidate(SelectionCandidate),
    UserActivity(Option<(i32, i32)>),
    SuggestionsChanged {
        items: Vec<SuggestionItem>,
        check_password: bool,
    },
    AcceptSuggestion {
        phrase_id: i64,
    },
    CycleSelectionCase,
}

struct HookShared {
    paused: Arc<AtomicBool>,
    suggest_enabled: Arc<AtomicBool>,
    shift_f3_enabled: Arc<AtomicBool>,
    /// Genişletme sürerken kullanıcı tuşlarını sırayla bekleten kapı (§1.3).
    gate: InjectGate,
    matcher: Arc<RwLock<Matcher>>,
    excluded: Arc<RwLock<Vec<String>>>,
    trigger_config: Arc<RwLock<TriggerConfig>>,
    state: Mutex<HookFastState>,
    tx: mpsc::Sender<WorkerJob>,
    /// Odaklanan denetim parola kutusu mu? (§7.4) Kanca yalnız `Normal` durumda tuş işler.
    focus: FocusTracker,
    /// `meetin-focus` iş parçacığını uyandırır (odak değişti, yeniden sorgula).
    focus_tx: mpsc::Sender<()>,
}

#[derive(Default)]
struct HookFastState {
    last_hwnd: isize,
    buffer: KeyBuffer,
    last_expansion: Option<LastExpansion>,
    mouse_down_pos: Option<(i32, i32)>,
    last_click_up: Option<(Instant, i32, i32)>,
    active_suggestions: Vec<SuggestionItem>,
    active_suggest_hwnd: isize,
    active_suggest_typed: String,
    active_suggest_process: Option<String>,
}

static HOOK_SHARED: OnceLock<Arc<HookShared>> = OnceLock::new();

/// Veritabanından `{ifade:KISALTMA}` başvurularını çözmek için yardımcı arar.
pub fn resolve_with_storage(
    storage: &Storage,
    template: &str,
    field_values: HashMap<String, String>,
    fold_dotted_i: bool,
) -> vars::ResolvedOutput {
    let all_phrases = storage.list_phrases(false).unwrap_or_default();
    let clip = if template.contains("{pano}") {
        selection::read_clipboard_text()
    } else {
        None
    };

    let ctx = ResolveContext {
        now: Local::now(),
        clipboard_text: clip,
        field_values,
        fold_dotted_i,
        phrase_lookup: Box::new(move |abbr_ref| {
            let target = normalize_abbr(abbr_ref, fold_dotted_i);
            all_phrases
                .iter()
                .find(|p| p.enabled && normalize_abbr(&p.abbr, fold_dotted_i) == target)
                .map(|p| p.content.clone())
        }),
    };
    vars::resolve_template(template, &ctx)
}

/// Kanca başlatıcı (`AppState` yönetildikten sonra `.setup` içinde başlatılır).
pub struct HookStarter {
    storage: Arc<Storage>,
    shared: Arc<HookShared>,
    rx: mpsc::Receiver<WorkerJob>,
    focus_rx: mpsc::Receiver<()>,
}

/// Önce `HookController` + `HookStarter` üretir; böylece `AppState` Builder'da `.manage()` edilebilir.
pub fn create_controller(storage: Arc<Storage>) -> (HookController, HookStarter) {
    let paused = Arc::new(AtomicBool::new(false));

    let fold_dotted_i = storage
        .get_setting("fold_dotted_i")
        .ok()
        .flatten()
        .map(|v| v == "true")
        .unwrap_or(false);

    let mut matcher_init = Matcher::new(fold_dotted_i);
    if let Ok(entries) = storage.load_matcher_entries() {
        matcher_init.rebuild(entries);
    }
    let matcher = Arc::new(RwLock::new(matcher_init));

    let excluded_init: Vec<String> = storage
        .get_setting("excluded_processes")
        .ok()
        .flatten()
        .and_then(|raw| serde_json::from_str::<Vec<String>>(&raw).ok())
        .map(|list| list.into_iter().map(|s| tr_lower(&s)).collect())
        .unwrap_or_else(|| {
            DEFAULT_EXCLUDED_PROCESSES
                .iter()
                .map(|s| s.to_string())
                .collect()
        });
    let excluded = Arc::new(RwLock::new(excluded_init));

    let trigger_init: TriggerConfig = storage
        .get_setting("trigger_config")
        .ok()
        .flatten()
        .and_then(|raw| serde_json::from_str::<TriggerConfig>(&raw).ok())
        .unwrap_or_default();
    let trigger_config = Arc::new(RwLock::new(trigger_init));

    let suggest_init = storage
        .get_setting("suggest_enabled")
        .ok()
        .flatten()
        .map(|v| v == "true")
        .unwrap_or(true);
    let suggest_enabled = Arc::new(AtomicBool::new(suggest_init));

    let shift_f3_init = storage
        .get_setting("shift_f3_enabled")
        .ok()
        .flatten()
        .map(|v| v == "true")
        .unwrap_or(true);
    let shift_f3_enabled = Arc::new(AtomicBool::new(shift_f3_init));

    let (tx, rx) = mpsc::channel::<WorkerJob>();
    let (focus_tx, focus_rx) = mpsc::channel::<()>();

    let shared = Arc::new(HookShared {
        paused: Arc::clone(&paused),
        suggest_enabled: Arc::clone(&suggest_enabled),
        shift_f3_enabled: Arc::clone(&shift_f3_enabled),
        gate: InjectGate::default(),
        matcher: Arc::clone(&matcher),
        excluded: Arc::clone(&excluded),
        trigger_config: Arc::clone(&trigger_config),
        state: Mutex::new(HookFastState::default()),
        tx: tx.clone(),
        focus: FocusTracker::default(),
        focus_tx,
    });

    let controller = HookController {
        paused,
        suggest_enabled,
        shift_f3_enabled,
        matcher,
        excluded,
        trigger_config,
        tx,
    };

    let starter = HookStarter {
        storage,
        shared,
        rx,
        focus_rx,
    };
    (controller, starter)
}

impl HookStarter {
    pub fn start(self, callbacks: HookCallbacks) {
        let _ = HOOK_SHARED.set(Arc::clone(&self.shared));
        let worker_storage = self.storage;
        let worker_shared = self.shared;
        let rx = self.rx;
        let focus_rx = self.focus_rx;
        let on_expanded = callbacks.on_expanded;
        let on_selection_candidate = callbacks.on_selection_candidate;
        let on_user_activity = callbacks.on_user_activity;
        let on_fill_requested = callbacks.on_fill_requested;
        let on_suggestions_changed = callbacks.on_suggestions_changed;

        thread::Builder::new()
            .name("meetin-expander".to_string())
            .spawn(move || {
                while let Ok(job) = rx.recv() {
                    match job {
                        WorkerJob::Expand {
                            matched,
                            hwnd,
                            process_name,
                        } => {
                            // Kapı, tetiği yutan kancada açıldı; arm bitince (continue dahil)
                            // bekleyen kullanıcı tuşları sırayla gönderilir (§1.3).
                            let gate_guard = worker_shared.gate.open(inject::replay_keys);

                            // Parola kutusundaysa ASLA genişletme; yutulan tetik tuşunu ilet (§7.4)
                            #[cfg(windows)]
                            if win_impl::is_focused_control_password() {
                                if matched.trigger != '\0' {
                                    let _ = inject::send_single_trigger(matched.trigger);
                                }
                                continue;
                            }

                            // Doldurulacak alan ({alan:...}) varsa kısaltmayı silip Alan Adasını aç (§3.3, §5.2)
                            let fields = vars::extract_form_fields(&matched.expanded_text);
                            if !fields.is_empty() {
                                let _ = inject::inject_undo(matched.abbr_char_len, "");
                                if let Some(cb) = &on_fill_requested {
                                    cb(FillRequest {
                                        matched,
                                        fields,
                                        process_name,
                                    });
                                }
                                continue;
                            }

                            let fold = worker_shared
                                .matcher
                                .read()
                                .map(|m| m.fold_dotted_i())
                                .unwrap_or(false);
                            let resolved = resolve_with_storage(
                                &worker_storage,
                                &matched.expanded_text,
                                HashMap::new(),
                                fold,
                            );
                            let resolved_html = matched.content_html.as_deref().map(|h| {
                                resolve_with_storage(&worker_storage, h, HashMap::new(), fold).text
                            });
                            let trigger_opt = if matched.trigger == '\0' {
                                None
                            } else {
                                Some(matched.trigger)
                            };

                            let res = inject::inject_resolved(
                                matched.abbr_char_len,
                                &resolved.text,
                                resolved_html.as_deref(),
                                trigger_opt,
                                resolved.cursor_left_steps,
                                process_name.as_deref(),
                            );
                            // Geri alma kaydı kapı kapanmadan yazılır: bekleyen tuşlar yeniden
                            // gönderilince kanca bu kaydı doğru biçimde geçersiz kılar (§3.1).
                            if res.is_ok() && resolved.cursor_left_steps == 0 {
                                if let Ok(mut st) = worker_shared.state.lock() {
                                    st.last_expansion = Some(LastExpansion {
                                        hwnd,
                                        delete_len: resolved.text.chars().count()
                                            + usize::from(trigger_opt.is_some()),
                                        original_abbr: matched.abbr.clone(),
                                    });
                                }
                            }
                            drop(gate_guard);

                            if res.is_ok() {
                                let _ = worker_storage
                                    .record_usage(matched.phrase_id, process_name.as_deref());
                                if let Some(cb) = &on_expanded {
                                    cb(ExpansionNotice {
                                        phrase_id: matched.phrase_id,
                                        abbr: matched.abbr.clone(),
                                    });
                                }
                            }
                        }
                        WorkerJob::UndoExpansion {
                            delete_len,
                            original_abbr,
                        } => {
                            let gate_guard = worker_shared.gate.open(inject::replay_keys);
                            let _ = inject::inject_undo(delete_len, &original_abbr);
                            drop(gate_guard);
                        }
                        WorkerJob::SelectionCandidate(cand) => {
                            if let Some(cb) = &on_selection_candidate {
                                cb(cand);
                            }
                        }
                        WorkerJob::UserActivity(pos) => {
                            if let Some(cb) = &on_user_activity {
                                cb(pos);
                            }
                        }
                        WorkerJob::SuggestionsChanged {
                            items,
                            check_password,
                        } => {
                            #[cfg(windows)]
                            let safe_items =
                                if check_password && win_impl::is_focused_control_password() {
                                    if let Ok(mut st) = worker_shared.state.lock() {
                                        st.active_suggestions.clear();
                                        st.active_suggest_typed.clear();
                                    }
                                    Vec::new()
                                } else {
                                    items
                                };
                            #[cfg(not(windows))]
                            let safe_items = {
                                let _ = check_password;
                                items
                            };
                            if let Some(cb) = &on_suggestions_changed {
                                cb(safe_items);
                            }
                        }
                        WorkerJob::AcceptSuggestion { phrase_id } => {
                            let pending = if let Ok(mut st) = worker_shared.state.lock() {
                                let typed = std::mem::take(&mut st.active_suggest_typed);
                                let proc_name = st.active_suggest_process.take();
                                let hwnd = st.active_suggest_hwnd;
                                st.active_suggestions.clear();
                                st.buffer.clear();
                                let built =
                                    worker_shared.matcher.read().ok().and_then(|m| {
                                        m.build_match_for_id(phrase_id, &typed, '\0')
                                    });
                                built.map(|m| (m, hwnd, proc_name))
                            } else {
                                None
                            };
                            if let Some(cb) = &on_suggestions_changed {
                                cb(Vec::new());
                            }
                            if let Some((matched, hwnd, proc_name)) = pending {
                                let _ = worker_shared.tx.send(WorkerJob::Expand {
                                    matched,
                                    hwnd,
                                    process_name: proc_name,
                                });
                            }
                        }
                        WorkerJob::CycleSelectionCase => {
                            #[cfg(windows)]
                            {
                                let gate_guard = worker_shared.gate.open(inject::replay_keys);
                                if win_impl::is_focused_control_password() {
                                    continue;
                                }
                                let _ = inject::release_shift_keys();
                                thread::sleep(std::time::Duration::from_millis(20));
                                if let Some(selected) = selection::read_selected_text() {
                                    if !selected.trim().is_empty() {
                                        let cycled =
                                            crate::matcher::case::cycle_turkish_case(&selected);
                                        let (_, ctx) = win_impl::active_window_info();
                                        let shortcut = inject::paste_shortcut_for_process(
                                            ctx.process_name.as_deref(),
                                        );
                                        if inject::clipboard::paste_rich_preserving_clipboard(
                                            &cycled, None, shortcut,
                                        )
                                        .is_ok()
                                        {
                                            thread::sleep(std::time::Duration::from_millis(30));
                                            let left_n = inject::build_reselect_left_count(&cycled);
                                            let _ = inject::send_reselect_left(left_n);
                                        }
                                    }
                                }
                                drop(gate_guard);
                            }
                        }
                    }
                }
            })
            .ok();

        // Odak (parola kutusu) sorgu iş parçacığı (§7.4)
        #[cfg(windows)]
        {
            thread::Builder::new()
                .name("meetin-focus".to_string())
                .spawn(move || {
                    win_impl::run_focus_loop(focus_rx);
                })
                .ok();
        }
        #[cfg(not(windows))]
        drop(focus_rx);

        // Windows kanca iş parçacığı
        #[cfg(windows)]
        {
            thread::Builder::new()
                .name("meetin-hook".to_string())
                .spawn(move || {
                    win_impl::run_hook_loop();
                })
                .ok();
        }
    }
}

#[cfg(windows)]
mod win_impl {
    use super::focus::FocusState;
    use super::*;
    use windows::core::PWSTR;
    use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED,
    };
    use windows::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows::Win32::UI::Accessibility::{
        CUIAutomation, IUIAutomation, SetWinEventHook, UIA_IsPasswordPropertyId, UnhookWinEvent,
        HWINEVENTHOOK,
    };
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, GetKeyState, GetKeyboardLayout, ToUnicodeEx, VK_BACK, VK_CAPITAL,
        VK_CONTROL, VK_DELETE, VK_DOWN, VK_END, VK_ESCAPE, VK_F3, VK_HOME, VK_LEFT, VK_LSHIFT,
        VK_LWIN, VK_MENU, VK_NEXT, VK_PRIOR, VK_RETURN, VK_RIGHT, VK_RSHIFT, VK_RWIN, VK_SHIFT,
        VK_SPACE, VK_TAB, VK_UP,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, DispatchMessageW, GetForegroundWindow, GetMessageW, GetWindowTextW,
        GetWindowThreadProcessId, SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx,
        EVENT_OBJECT_FOCUS, EVENT_SYSTEM_FOREGROUND, KBDLLHOOKSTRUCT, LLKHF_EXTENDED, MSG,
        MSLLHOOKSTRUCT, WH_KEYBOARD_LL, WH_MOUSE_LL, WINEVENT_OUTOFCONTEXT,
        WINEVENT_SKIPOWNPROCESS, WM_KEYDOWN, WM_KEYUP, WM_LBUTTONDOWN, WM_LBUTTONUP,
        WM_MBUTTONDOWN, WM_RBUTTONDOWN, WM_SYSKEYDOWN, WM_SYSKEYUP,
    };

    pub fn run_hook_loop() {
        unsafe {
            let kb_hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), None, 0);
            let mouse_hook = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), None, 0);
            if kb_hook.is_err() {
                crate::logging::event(
                    crate::logging::Level::Error,
                    "hook.keyboard_install_failed",
                    &[],
                );
            }
            if mouse_hook.is_err() {
                crate::logging::event(
                    crate::logging::Level::Error,
                    "hook.mouse_install_failed",
                    &[],
                );
            }

            // Odak ve ön plan değişimleri (bu iş parçacığının mesaj döngüsünde teslim edilir)
            let flags = WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS;
            let focus_hook = SetWinEventHook(
                EVENT_OBJECT_FOCUS,
                EVENT_OBJECT_FOCUS,
                None,
                Some(focus_event_proc),
                0,
                0,
                flags,
            );
            let fg_hook = SetWinEventHook(
                EVENT_SYSTEM_FOREGROUND,
                EVENT_SYSTEM_FOREGROUND,
                None,
                Some(focus_event_proc),
                0,
                0,
                flags,
            );
            request_focus_check();

            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            if let Ok(h) = kb_hook {
                let _ = UnhookWindowsHookEx(h);
            }
            if let Ok(h) = mouse_hook {
                let _ = UnhookWindowsHookEx(h);
            }
            for h in [focus_hook, fg_hook] {
                if !h.is_invalid() {
                    let _ = UnhookWinEvent(h);
                }
            }
        }
    }

    /// Odak sorgusunu geçersiz kılar ve `meetin-focus` iş parçacığına iş yollar.
    fn request_focus_check() {
        if let Some(shared) = HOOK_SHARED.get() {
            shared.focus.invalidate();
            let _ = shared.focus_tx.send(());
        }
    }

    unsafe extern "system" fn focus_event_proc(
        _hook: HWINEVENTHOOK,
        _event: u32,
        _hwnd: HWND,
        _id_object: i32,
        _id_child: i32,
        _thread: u32,
        _time: u32,
    ) {
        request_focus_check();
    }

    /// Odaklanan denetimin parola kutusu olup olmadığını sorgulayan döngü.
    /// UIA nesnesi bir kez oluşturulur; birikmiş istekler tek sorguda birleştirilir.
    pub fn run_focus_loop(rx: mpsc::Receiver<()>) {
        let uia: Option<IUIAutomation> = unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER).ok()
        };
        while rx.recv().is_ok() {
            while rx.try_recv().is_ok() {}
            let Some(shared) = HOOK_SHARED.get() else {
                continue;
            };
            let generation = shared.focus.current_generation();
            let result = FocusState::from_query(uia.as_ref().and_then(query_is_password));
            if shared.focus.resolve(generation, result) && result == FocusState::Password {
                if let Ok(mut st) = shared.state.lock() {
                    st.buffer.clear();
                    st.last_expansion = None;
                    st.active_suggestions.clear();
                    st.active_suggest_typed.clear();
                }
                let _ = shared.tx.send(WorkerJob::SuggestionsChanged {
                    items: Vec::new(),
                    check_password: false,
                });
            }
        }
    }

    fn query_is_password(uia: &IUIAutomation) -> Option<bool> {
        unsafe {
            let elem = uia.GetFocusedElement().ok()?;
            let val = elem
                .GetCurrentPropertyValue(UIA_IsPasswordPropertyId)
                .ok()?;
            bool::try_from(&val).ok()
        }
    }

    /// Etkin pencerenin işlem adını ve başlığını döndürür.
    pub(super) fn active_window_info() -> (isize, ActiveContext) {
        unsafe {
            let hwnd = GetForegroundWindow();
            let raw = hwnd.0 as isize;
            if raw == 0 {
                return (0, ActiveContext::default());
            }

            let mut title_buf = [0u16; 256];
            let title_len = GetWindowTextW(hwnd, &mut title_buf);
            let window_title = if title_len > 0 {
                Some(String::from_utf16_lossy(&title_buf[..title_len as usize]))
            } else {
                None
            };

            let mut pid: u32 = 0;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            let process_name = if pid != 0 {
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
                        full.rsplit(['\\', '/']).next().map(|s| s.to_string())
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            };

            (
                raw,
                ActiveContext {
                    process_name,
                    window_title,
                },
            )
        }
    }

    /// Odaklanılan denetimin parola kutusu olup olmadığını UI Automation ile kontrol eder (§7.4).
    pub fn is_focused_control_password() -> bool {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            let Ok(uia): Result<IUIAutomation, _> =
                CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER)
            else {
                return false;
            };
            let Ok(elem) = uia.GetFocusedElement() else {
                return false;
            };
            if let Ok(val) = elem.GetCurrentPropertyValue(UIA_IsPasswordPropertyId) {
                // VARIANT boolean: VT_BOOL (11), boolVal != 0
                if let Ok(b) = bool::try_from(&val) {
                    return b;
                }
            }
            false
        }
    }

    #[inline]
    unsafe fn is_key_down(vk: u16) -> bool {
        (GetAsyncKeyState(vk as i32) as u16 & 0x8000) != 0
    }

    /// Klavye düzeninden bağımsız, ölü tuşları bozmadan (`uFlags = 0x4`) karakter çözer.
    unsafe fn vk_to_char(vk: u32, scan: u32, hwnd: HWND) -> Option<char> {
        match vk as u16 {
            v if v == VK_SPACE.0 => return Some(' '),
            v if v == VK_RETURN.0 => return Some('\n'),
            v if v == VK_TAB.0 => return Some('\t'),
            _ => {}
        }

        let mut kb_state = [0u8; 256];
        // Kanca iş parçacığının odaklanılmış penceresi olmadığı için değiştirici tuşları
        // doğrudan GetAsyncKeyState / GetKeyState üzerinden dolduruyoruz.
        if is_key_down(VK_SHIFT.0) || is_key_down(VK_LSHIFT.0) || is_key_down(VK_RSHIFT.0) {
            kb_state[VK_SHIFT.0 as usize] = 0x80;
        }
        if is_key_down(VK_CONTROL.0) {
            kb_state[VK_CONTROL.0 as usize] = 0x80;
        }
        if is_key_down(VK_MENU.0) {
            kb_state[VK_MENU.0 as usize] = 0x80;
        }
        if (GetKeyState(VK_CAPITAL.0 as i32) & 0x0001) != 0 {
            kb_state[VK_CAPITAL.0 as usize] = 0x01;
        }

        let thread_id = GetWindowThreadProcessId(hwnd, None);
        let hkl = GetKeyboardLayout(thread_id);
        let mut out = [0u16; 8];
        // 0x4 = Windows 10 1607+ bayrağı: klavye durumunu (ölü tuşları) DEĞİŞTİRMEZ
        let written = ToUnicodeEx(vk, scan, &kb_state, &mut out, 0x4, hkl);
        if written == 1 {
            char::from_u32(out[0] as u32)
        } else {
            None
        }
    }

    unsafe extern "system" fn mouse_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code >= 0 {
            let msg = wparam.0 as u32;
            let ms = &*(lparam.0 as *const MSLLHOOKSTRUCT);
            if ms.dwExtraInfo == INJECTED_EXTRA_INFO {
                return CallNextHookEx(None, code, wparam, lparam);
            }

            if let Some(shared) = HOOK_SHARED.get() {
                if msg == WM_LBUTTONDOWN || msg == WM_RBUTTONDOWN || msg == WM_MBUTTONDOWN {
                    if let Ok(mut st) = shared.state.try_lock() {
                        st.buffer.clear();
                        st.last_expansion = None;
                        if !st.active_suggestions.is_empty() {
                            st.active_suggestions.clear();
                            st.active_suggest_typed.clear();
                            let _ = shared.tx.send(WorkerJob::SuggestionsChanged {
                                items: Vec::new(),
                                check_password: false,
                            });
                        }
                        if msg == WM_LBUTTONDOWN {
                            st.mouse_down_pos = Some((ms.pt.x, ms.pt.y));
                        }
                    }
                    let _ = shared
                        .tx
                        .send(WorkerJob::UserActivity(Some((ms.pt.x, ms.pt.y))));
                } else if msg == WM_LBUTTONUP {
                    let mut is_selection_gesture = false;
                    let now = Instant::now();
                    if let Ok(mut st) = shared.state.try_lock() {
                        if let Some((dx0, dy0)) = st.mouse_down_pos.take() {
                            let drag_dist =
                                (((ms.pt.x - dx0).pow(2) + (ms.pt.y - dy0).pow(2)) as f64).sqrt();
                            if drag_dist > 8.0 {
                                is_selection_gesture = true;
                            }
                        }
                        if !is_selection_gesture {
                            if let Some((prev_t, px, py)) = st.last_click_up {
                                let dt = now.duration_since(prev_t).as_millis();
                                let dist =
                                    (((ms.pt.x - px).pow(2) + (ms.pt.y - py).pow(2)) as f64).sqrt();
                                if dt <= 450 && dist <= 6.0 {
                                    is_selection_gesture = true;
                                }
                            }
                        }
                        st.last_click_up = Some((now, ms.pt.x, ms.pt.y));
                    }

                    if is_selection_gesture && !shared.gate.is_active() {
                        let (_, ctx) = active_window_info();
                        let hidden = shared
                            .excluded
                            .try_read()
                            .map(|ex| is_bubble_hidden_process(ctx.process_name.as_deref(), &ex))
                            .unwrap_or(false);
                        if !hidden {
                            let _ =
                                shared
                                    .tx
                                    .send(WorkerJob::SelectionCandidate(SelectionCandidate {
                                        x: ms.pt.x,
                                        y: ms.pt.y,
                                        process_name: ctx.process_name,
                                    }));
                        }
                    }
                }
            }
        }
        CallNextHookEx(None, code, wparam, lparam)
    }

    unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code < 0 {
            return CallNextHookEx(None, code, wparam, lparam);
        }

        let msg = wparam.0 as u32;
        let Some(shared) = HOOK_SHARED.get() else {
            return CallNextHookEx(None, code, wparam, lparam);
        };

        let kb = &*(lparam.0 as *const KBDLLHOOKSTRUCT);

        // Kendi enjekte ettiğimiz metin tuşlarını doğrudan geçir
        if kb.dwExtraInfo == INJECTED_EXTRA_INFO {
            return CallNextHookEx(None, code, wparam, lparam);
        }

        // Genişletme sürüyorsa kullanıcının tuşlarını (basma ve bırakma) sıraya al;
        // worker bitince aynı sırayla yeniden gönderir (§1.3). Yeniden gönderilenler
        // REPLAYED_EXTRA_INFO taşır ve aşağıda normal yazı gibi işlenir.
        let queued = QueuedKey {
            vk: kb.vkCode as u16,
            scan: kb.scanCode as u16,
            up: msg == WM_KEYUP || msg == WM_SYSKEYUP,
            extended: kb.flags.0 & LLKHF_EXTENDED.0 != 0,
        };
        let held = if kb.dwExtraInfo == REPLAYED_EXTRA_INFO {
            shared.gate.admit_replayed(queued)
        } else {
            shared.gate.admit(queued, Instant::now())
        };
        if held {
            return LRESULT(1);
        }

        if msg != WM_KEYDOWN && msg != WM_SYSKEYDOWN {
            return CallNextHookEx(None, code, wparam, lparam);
        }

        if shared.paused.load(Ordering::Relaxed) {
            return CallNextHookEx(None, code, wparam, lparam);
        }

        // Kullanıcı yazmaya devam ediyorsa açık seçim balonunu gizle (§4.1)
        let _ = shared.tx.send(WorkerJob::UserActivity(None));

        // Parola kutusu ya da henüz sorgulanmamış odak: tuşa hiç dokunma (§7.4 [K]).
        // Arabelleğe alınmaz, tetik yutulmaz, öneri gösterilmez.
        if shared.focus.state().passes_through() {
            if let Ok(mut st) = shared.state.try_lock() {
                st.buffer.clear();
                st.last_expansion = None;
            }
            return CallNextHookEx(None, code, wparam, lparam);
        }

        let (hwnd_raw, ctx) = active_window_info();

        // Dışlanan programlarda tam geçiş (§7.4)
        if let Ok(excl) = shared.excluded.try_read() {
            if is_process_excluded(ctx.process_name.as_deref(), &excl) {
                if let Ok(mut st) = shared.state.try_lock() {
                    st.buffer.clear();
                    st.last_hwnd = hwnd_raw;
                }
                return CallNextHookEx(None, code, wparam, lparam);
            }
        }

        let vk = kb.vkCode as u16;

        // Ctrl, Alt (AltGr hariç) veya Win basılıysa kısayol kombinasyonudur -> arabelleği temizle
        let shift_down =
            is_key_down(VK_SHIFT.0) || is_key_down(VK_LSHIFT.0) || is_key_down(VK_RSHIFT.0);
        let ctrl_down = is_key_down(VK_CONTROL.0);
        let alt_down = is_key_down(VK_MENU.0);
        let win_down = is_key_down(VK_LWIN.0) || is_key_down(VK_RWIN.0);

        // Shift+F3: herhangi bir düzenlenebilir uygulamada seçili metnin büyük/küçük harfini döndür
        if vk == VK_F3.0
            && shift_down
            && !ctrl_down
            && !alt_down
            && !win_down
            && shared.shift_f3_enabled.load(Ordering::Relaxed)
        {
            if let Ok(mut st) = shared.state.try_lock() {
                st.buffer.clear();
                st.last_expansion = None;
            }
            shared.gate.begin(Instant::now());
            let _ = shared.tx.send(WorkerJob::CycleSelectionCase);
            return LRESULT(1);
        }

        // Türkçe klavyede AltGr = Ctrl+Alt kombinasyonuyla gelir; ikisi birlikteyken karakter üretebilir (@, €, ~ vb.)
        let is_altgr = ctrl_down && alt_down;
        if win_down || ((ctrl_down || alt_down) && !is_altgr) {
            if let Ok(mut st) = shared.state.try_lock() {
                st.buffer.clear();
                st.last_expansion = None;
            }
            return CallNextHookEx(None, code, wparam, lparam);
        }

        // Yön ve temizleme tuşları
        if matches!(
            vk,
            v if v == VK_LEFT.0
                || v == VK_RIGHT.0
                || v == VK_UP.0
                || v == VK_DOWN.0
                || v == VK_HOME.0
                || v == VK_END.0
                || v == VK_PRIOR.0
                || v == VK_NEXT.0
                || v == VK_ESCAPE.0
                || v == VK_DELETE.0
        ) {
            if let Ok(mut st) = shared.state.try_lock() {
                st.buffer.clear();
                st.last_expansion = None;
                if !st.active_suggestions.is_empty() {
                    st.active_suggestions.clear();
                    st.active_suggest_typed.clear();
                    let _ = shared.tx.send(WorkerJob::SuggestionsChanged {
                        items: Vec::new(),
                        check_password: false,
                    });
                }
            }
            return CallNextHookEx(None, code, wparam, lparam);
        }

        if vk == VK_BACK.0 {
            let mut undo_swallow = false;
            if let Ok(mut st) = shared.state.try_lock() {
                if st.last_hwnd != hwnd_raw {
                    st.buffer.clear();
                    st.last_expansion = None;
                    st.last_hwnd = hwnd_raw;
                    if !st.active_suggestions.is_empty() {
                        st.active_suggestions.clear();
                        st.active_suggest_typed.clear();
                        let _ = shared.tx.send(WorkerJob::SuggestionsChanged {
                            items: Vec::new(),
                            check_password: false,
                        });
                    }
                } else if let Some(last) = st.last_expansion.take() {
                    if last.hwnd == hwnd_raw {
                        shared.gate.begin(Instant::now());
                        let _ = shared.tx.send(WorkerJob::UndoExpansion {
                            delete_len: last.delete_len,
                            original_abbr: last.original_abbr,
                        });
                        undo_swallow = true;
                    }
                } else {
                    st.buffer.backspace();
                    if shared.suggest_enabled.load(Ordering::Relaxed) {
                        let snapshot = st.buffer.as_string();
                        let items = shared
                            .matcher
                            .try_read()
                            .ok()
                            .map(|m| m.suggest_prefix(&snapshot, &ctx, 3))
                            .unwrap_or_default();
                        let was_empty = st.active_suggestions.is_empty();
                        if items.is_empty() {
                            if !was_empty {
                                st.active_suggestions.clear();
                                st.active_suggest_typed.clear();
                                let _ = shared.tx.send(WorkerJob::SuggestionsChanged {
                                    items: Vec::new(),
                                    check_password: false,
                                });
                            }
                        } else {
                            let tlen = items[0].typed_len;
                            let chars: Vec<char> = snapshot.chars().collect();
                            let start = chars.len().saturating_sub(tlen);
                            st.active_suggest_typed = chars[start..].iter().collect();
                            st.active_suggest_hwnd = hwnd_raw;
                            st.active_suggest_process = ctx.process_name.clone();
                            st.active_suggestions = items.clone();
                            let _ = shared.tx.send(WorkerJob::SuggestionsChanged {
                                items,
                                check_password: true,
                            });
                        }
                    }
                }
            }
            if undo_swallow {
                return LRESULT(1);
            }
            return CallNextHookEx(None, code, wparam, lparam);
        }

        let hwnd = HWND(hwnd_raw as *mut _);
        let Some(ch) = vk_to_char(kb.vkCode, kb.scanCode, hwnd) else {
            return CallNextHookEx(None, code, wparam, lparam);
        };

        // Kontrol karakterlerini (Enter ve Tab hariç) yok say
        if ch.is_control() && ch != '\n' && ch != '\t' {
            return CallNextHookEx(None, code, wparam, lparam);
        }

        let mut swallow = false;
        if let Ok(mut st) = shared.state.try_lock() {
            st.last_expansion = None;
            if st.last_hwnd != hwnd_raw {
                st.buffer.clear();
                st.last_hwnd = hwnd_raw;
                if !st.active_suggestions.is_empty() {
                    st.active_suggestions.clear();
                    st.active_suggest_typed.clear();
                    let _ = shared.tx.send(WorkerJob::SuggestionsChanged {
                        items: Vec::new(),
                        check_password: false,
                    });
                }
            }

            let triggered = shared
                .trigger_config
                .try_read()
                .map(|tc| tc.matches(ch))
                .unwrap_or_else(|_| is_trigger_char(ch));

            if triggered {
                let snapshot = st.buffer.as_string();
                let matched = shared
                    .matcher
                    .try_read()
                    .ok()
                    .and_then(|m| m.try_match(&snapshot, ch, &ctx));

                if let Some(m) = matched {
                    st.buffer.clear();
                    if !st.active_suggestions.is_empty() {
                        st.active_suggestions.clear();
                        st.active_suggest_typed.clear();
                        let _ = shared.tx.send(WorkerJob::SuggestionsChanged {
                            items: Vec::new(),
                            check_password: false,
                        });
                    }
                    shared.gate.begin(Instant::now());
                    let _ = shared.tx.send(WorkerJob::Expand {
                        matched: m,
                        hwnd: hwnd_raw,
                        process_name: ctx.process_name,
                    });
                    swallow = true;
                } else if ch == '\t'
                    && !st.active_suggestions.is_empty()
                    && st.active_suggest_hwnd == hwnd_raw
                {
                    let top_id = st.active_suggestions[0].phrase_id;
                    let typed_slice = st.active_suggest_typed.clone();
                    let accepted = shared
                        .matcher
                        .try_read()
                        .ok()
                        .and_then(|m| m.build_match_for_id(top_id, &typed_slice, '\0'));
                    if let Some(m) = accepted {
                        st.buffer.clear();
                        st.active_suggestions.clear();
                        st.active_suggest_typed.clear();
                        let _ = shared.tx.send(WorkerJob::SuggestionsChanged {
                            items: Vec::new(),
                            check_password: false,
                        });
                        shared.gate.begin(Instant::now());
                        let _ = shared.tx.send(WorkerJob::Expand {
                            matched: m,
                            hwnd: hwnd_raw,
                            process_name: ctx.process_name,
                        });
                        swallow = true;
                    } else {
                        st.buffer.push(ch);
                    }
                } else {
                    if !st.active_suggestions.is_empty() {
                        st.active_suggestions.clear();
                        st.active_suggest_typed.clear();
                        let _ = shared.tx.send(WorkerJob::SuggestionsChanged {
                            items: Vec::new(),
                            check_password: false,
                        });
                    }
                    // Tetik karakterini (boşluk, noktalama vb.) arabellekte tut ki kullanıcı
                    // geri silip düzelttiğinde kısaltma bağlamı kaybolmasın (§3.1)
                    st.buffer.push(ch);
                }
            } else {
                st.buffer.push(ch);
                if shared.suggest_enabled.load(Ordering::Relaxed) {
                    let snapshot = st.buffer.as_string();
                    let items = shared
                        .matcher
                        .try_read()
                        .ok()
                        .map(|m| m.suggest_prefix(&snapshot, &ctx, 3))
                        .unwrap_or_default();
                    let was_empty = st.active_suggestions.is_empty();
                    if items.is_empty() {
                        if !was_empty {
                            st.active_suggestions.clear();
                            st.active_suggest_typed.clear();
                            let _ = shared.tx.send(WorkerJob::SuggestionsChanged {
                                items: Vec::new(),
                                check_password: false,
                            });
                        }
                    } else {
                        let tlen = items[0].typed_len;
                        let chars: Vec<char> = snapshot.chars().collect();
                        let start = chars.len().saturating_sub(tlen);
                        st.active_suggest_typed = chars[start..].iter().collect();
                        st.active_suggest_hwnd = hwnd_raw;
                        st.active_suggest_process = ctx.process_name.clone();
                        st.active_suggestions = items.clone();
                        let _ = shared.tx.send(WorkerJob::SuggestionsChanged {
                            items,
                            check_password: true,
                        });
                    }
                }
            }
        }

        if swallow {
            LRESULT(1)
        } else {
            CallNextHookEx(None, code, wparam, lparam)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn excluded_process_matching_is_case_insensitive() {
        let excl = vec!["keepassxc.exe".to_string(), "mstsc.exe".to_string()];
        assert!(is_process_excluded(Some("KeePassXC.EXE"), &excl));
        assert!(is_process_excluded(Some("mstsc.exe"), &excl));
        assert!(!is_process_excluded(Some("notepad.exe"), &excl));
        assert!(!is_process_excluded(None, &excl));
    }

    #[test]
    fn trigger_config_respects_toggles() {
        let mut tc = TriggerConfig::default();
        assert!(tc.matches(' '));
        assert!(tc.matches('\n'));
        assert!(tc.matches('\t'));
        assert!(tc.matches('.'));

        tc.tab = false;
        tc.punctuation = false;
        assert!(tc.matches(' '));
        assert!(tc.matches('\n'));
        assert!(!tc.matches('\t'));
        assert!(!tc.matches('.'));
        assert!(!tc.matches(','));
    }
}
