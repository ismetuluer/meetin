//! Kayıt günlüğü ve çökme kaydı (R1.4).
//!
//! Dosya: `<veri klasörü>\logs\meetin.log`, 1 MB'ı aşınca döner (`meetin.1.log` … `meetin.4.log`).
//!
//! **Gizlilik (§7.4 [K]):** Günlüğe tuş, tuş arabelleği, ifade metni, kısaltma, pano içeriği ya da
//! pencere başlığı yazılmaz. Bunu güvenceye almak için [`event`] yalnızca sabit olay kodu
//! (`&'static str`) ve [`Field`] değerleri (sayı, mantıksal değer, sabit kod) alır. Serbest metin
//! yalnızca [`error_text`] ile, sistem/kütüphane hata iletileri için yazılır; kırpılır ve tek
//! satıra indirilir. Kullanıcı yolu (kullanıcı adı içerir) yazılmaz.

use std::fmt::Display;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

pub const FILE_NAME: &str = "meetin.log";
pub const MAX_FILE_BYTES: u64 = 1024 * 1024;
pub const KEEP_ROTATED: usize = 4;
pub const MAX_TEXT_CHARS: usize = 300;
const EXIT_CODE: &str = "app.exit";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Info,
    Warn,
    Error,
    Crash,
}

impl Level {
    fn as_str(self) -> &'static str {
        match self {
            Level::Info => "INFO",
            Level::Warn => "WARN",
            Level::Error => "ERROR",
            Level::Crash => "CRASH",
        }
    }
}

/// Bir olaya eklenebilecek değerler. Kullanıcı içeriği taşıyabilecek `String` bilerek yoktur;
/// `Sys` yalnız sistemden gelen değerler (sürüm numarası gibi) içindir.
#[derive(Debug, Clone)]
pub enum Field {
    Num(i64),
    Bool(bool),
    Code(&'static str),
    Sys(String),
}

impl std::fmt::Display for Field {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Field::Num(n) => write!(f, "{n}"),
            Field::Bool(b) => write!(f, "{b}"),
            Field::Code(c) => write!(f, "{c}"),
            Field::Sys(s) => write!(f, "\"{}\"", sanitize(s)),
        }
    }
}

struct Logger {
    dir: PathBuf,
    file: Mutex<Option<File>>,
}

static LOGGER: OnceLock<Logger> = OnceLock::new();

/// Metni tek satıra indirir, tırnakları kaçırır ve [`MAX_TEXT_CHARS`] ile kırpar.
pub fn sanitize(text: &str) -> String {
    // Kullanıcı klasörü (kullanıcı adını içerir) günlüğe yazılmaz.
    let text = match std::env::var("USERPROFILE") {
        Ok(home) if !home.is_empty() => text.replace(&home, "%USERPROFILE%"),
        _ => text.to_string(),
    };
    let text = text.as_str();
    let mut out: String = text
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .take(MAX_TEXT_CHARS)
        .collect::<String>()
        .replace('"', "'");
    if text.chars().count() > MAX_TEXT_CHARS {
        out.push('…');
    }
    out
}

/// Tek bir günlük satırı üretir (saf; test edilir).
pub fn format_line(
    timestamp: &str,
    level: Level,
    code: &str,
    fields: &[(&'static str, Field)],
) -> String {
    let mut line = format!("{timestamp} {:<5} {code}", level.as_str());
    for (k, v) in fields {
        line.push_str(&format!(" {k}={v}"));
    }
    line.push('\n');
    line
}

/// Önceki oturumun düzgün kapanıp kapanmadığını son satırdan anlar (saf; test edilir).
pub fn previous_session_was_clean(tail: &str) -> bool {
    match tail.lines().rev().find(|l| !l.trim().is_empty()) {
        None => true,
        Some(last) => last.contains(EXIT_CODE),
    }
}

/// `meetin.log` → `meetin.1.log` → … → `meetin.N.log` kaydırır; en eskisi silinir.
pub fn rotate(dir: &Path) {
    let name = |i: usize| {
        if i == 0 {
            dir.join(FILE_NAME)
        } else {
            dir.join(format!("meetin.{i}.log"))
        }
    };
    let _ = fs::remove_file(name(KEEP_ROTATED));
    for i in (0..KEEP_ROTATED).rev() {
        let from = name(i);
        if from.exists() {
            let _ = fs::rename(&from, name(i + 1));
        }
    }
}

fn read_tail(path: &Path, bytes: u64) -> String {
    let Ok(mut f) = File::open(path) else {
        return String::new();
    };
    let len = f.metadata().map(|m| m.len()).unwrap_or(0);
    let _ = f.seek(SeekFrom::Start(len.saturating_sub(bytes)));
    let mut buf = Vec::new();
    let _ = f.read_to_end(&mut buf);
    String::from_utf8_lossy(&buf).into_owned()
}

fn open_append(path: &Path) -> Option<File> {
    OpenOptions::new().create(true).append(true).open(path).ok()
}

/// Günlüğü başlatır, panik ve yerel çökme yakalayıcılarını kurar. Bir kez çağrılır.
pub fn init(data_dir: &Path) {
    let dir = data_dir.join("logs");
    if fs::create_dir_all(&dir).is_err() {
        return;
    }
    let path = dir.join(FILE_NAME);
    let clean = previous_session_was_clean(&read_tail(&path, 512));
    if fs::metadata(&path).map(|m| m.len()).unwrap_or(0) > MAX_FILE_BYTES {
        rotate(&dir);
    }
    let file = open_append(&path);
    if LOGGER
        .set(Logger {
            dir,
            file: Mutex::new(file),
        })
        .is_err()
    {
        return;
    }
    if !clean {
        event(Level::Warn, "app.previous_session_unclean", &[]);
    }
    install_panic_hook();
    #[cfg(windows)]
    native::install();
}

pub fn log_dir() -> Option<PathBuf> {
    LOGGER.get().map(|l| l.dir.clone())
}

/// Günlüğe bir olay yazar. Günlük kurulmamışsa sessizce hiçbir şey yapmaz.
pub fn event(level: Level, code: &'static str, fields: &[(&'static str, Field)]) {
    write_event(level, code, fields, true);
}

/// Çökme yollarında kullanılır: günlük kilidi o an tutuluyorsa beklemez (kilitlenme olmaz).
fn crash_event(code: &'static str, fields: &[(&'static str, Field)]) {
    write_event(Level::Crash, code, fields, false);
}

fn write_event(level: Level, code: &'static str, fields: &[(&'static str, Field)], wait: bool) {
    let Some(logger) = LOGGER.get() else { return };
    let ts = chrono::Local::now()
        .format("%d.%m.%Y %H:%M:%S%.3f")
        .to_string();
    let line = format_line(&ts, level, code, fields);
    let mut guard = if wait {
        logger.file.lock().unwrap_or_else(|e| e.into_inner())
    } else {
        match logger.file.try_lock() {
            Ok(g) => g,
            Err(std::sync::TryLockError::Poisoned(e)) => e.into_inner(),
            Err(std::sync::TryLockError::WouldBlock) => return,
        }
    };
    let path = logger.dir.join(FILE_NAME);
    let size = guard
        .as_ref()
        .and_then(|f| f.metadata().ok())
        .map(|m| m.len())
        .unwrap_or(0);
    if size + line.len() as u64 > MAX_FILE_BYTES {
        *guard = None;
        rotate(&logger.dir);
    }
    if guard.is_none() {
        *guard = open_append(&path);
    }
    if let Some(f) = guard.as_mut() {
        let _ = f.write_all(line.as_bytes());
        let _ = f.flush();
    }
}

/// Sistem ya da kütüphane hata iletisini yazar (kırpılmış, tek satır).
/// Kullanıcı içeriği taşıyabilecek hatalar için kullanılmaz.
pub fn error_text(level: Level, code: &'static str, err: &dyn Display) {
    event(level, code, &[("err", Field::Sys(err.to_string()))]);
}

/// Açılış bilgisi: sürüm, Windows yapı numarası, kurulum türü, WebView2 sürümü.
pub fn log_startup(portable: bool) {
    let mut fields = vec![
        ("version", Field::Code(env!("CARGO_PKG_VERSION"))),
        ("portable", Field::Bool(portable)),
    ];
    #[cfg(windows)]
    if let Some(build) = native::windows_build() {
        fields.push(("windows_build", Field::Sys(build)));
    }
    if let Ok(wv) = tauri::webview_version() {
        fields.push(("webview2", Field::Sys(wv)));
    }
    event(Level::Info, "app.start", &fields);
}

/// Düzgün kapanış işareti (bir sonraki açılışta çökme tespiti için).
pub fn log_exit() {
    event(Level::Info, EXIT_CODE, &[]);
}

fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let location = info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_default();
        let message = info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_default();
        let thread = std::thread::current().name().unwrap_or("?").to_string();
        let backtrace = std::backtrace::Backtrace::force_capture().to_string();
        let frames: Vec<&str> = backtrace
            .lines()
            .map(str::trim)
            .filter(|l| l.contains("meetin"))
            .take(12)
            .collect();
        crash_event(
            "panic",
            &[
                ("thread", Field::Sys(thread)),
                ("at", Field::Sys(location)),
                ("msg", Field::Sys(message)),
                ("frames", Field::Sys(frames.join(" | "))),
            ],
        );
        previous(info);
    }));
}

#[cfg(windows)]
mod native {
    use super::{crash_event, Field};
    use windows::core::w;
    use windows::Win32::System::Diagnostics::Debug::{
        SetUnhandledExceptionFilter, EXCEPTION_POINTERS,
    };
    use windows::Win32::System::Registry::{RegGetValueW, HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ};

    /// Yakalanmamış yerel istisna (erişim ihlali vb.): kod ve adres yazılır, ardından
    /// Windows Hata Bildirimi her zamanki gibi devam eder.
    unsafe extern "system" fn on_native_crash(info: *const EXCEPTION_POINTERS) -> i32 {
        let (code, addr) = info
            .as_ref()
            .and_then(|p| p.ExceptionRecord.as_ref())
            .map(|r| {
                (
                    r.ExceptionCode.0 as u32 as i64,
                    r.ExceptionAddress as usize as i64,
                )
            })
            .unwrap_or((0, 0));
        crash_event(
            "native_exception",
            &[("code", Field::Num(code)), ("addr", Field::Num(addr))],
        );
        0 // EXCEPTION_CONTINUE_SEARCH
    }

    pub fn install() {
        unsafe {
            let _ = SetUnhandledExceptionFilter(Some(on_native_crash));
        }
    }

    pub fn windows_build() -> Option<String> {
        let mut buf = [0u16; 32];
        let mut size = (buf.len() * 2) as u32;
        let ok = unsafe {
            RegGetValueW(
                HKEY_LOCAL_MACHINE,
                w!("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion"),
                w!("CurrentBuild"),
                RRF_RT_REG_SZ,
                None,
                Some(buf.as_mut_ptr().cast()),
                Some(&mut size),
            )
        };
        if ok.is_err() {
            return None;
        }
        let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        Some(String::from_utf16_lossy(&buf[..len]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn satir_bicimi() {
        let line = format_line(
            "27.11.2026 10:00:00.000",
            Level::Error,
            "db.backup",
            &[
                ("n", Field::Num(3)),
                ("ok", Field::Bool(false)),
                ("kind", Field::Code("io")),
            ],
        );
        assert_eq!(
            line,
            "27.11.2026 10:00:00.000 ERROR db.backup n=3 ok=false kind=io\n"
        );
    }

    #[test]
    fn serbest_metin_tek_satir_ve_kirpilir() {
        let long = "a\nb\"c".to_string() + &"x".repeat(500);
        let s = sanitize(&long);
        assert!(!s.contains('\n'));
        assert!(!s.contains('"'));
        assert!(s.ends_with('…'));
        assert_eq!(s.chars().count(), MAX_TEXT_CHARS + 1);
    }

    #[test]
    fn duzgun_kapanis_tespiti() {
        assert!(previous_session_was_clean(""));
        assert!(previous_session_was_clean(
            "x INFO app.start\nx INFO app.exit\n"
        ));
        assert!(!previous_session_was_clean("x INFO app.start\n"));
        assert!(!previous_session_was_clean(
            "x INFO app.exit\nx INFO app.start\n\n"
        ));
    }

    #[test]
    fn donme_en_eskiyi_siler() {
        let dir = tempfile::tempdir().unwrap();
        for i in 0..=KEEP_ROTATED {
            let name = if i == 0 {
                FILE_NAME.to_string()
            } else {
                format!("meetin.{i}.log")
            };
            fs::write(dir.path().join(name), format!("{i}")).unwrap();
        }
        rotate(dir.path());
        assert!(!dir.path().join(FILE_NAME).exists());
        assert_eq!(
            fs::read_to_string(dir.path().join("meetin.1.log")).unwrap(),
            "0"
        );
        assert_eq!(
            fs::read_to_string(dir.path().join(format!("meetin.{KEEP_ROTATED}.log"))).unwrap(),
            format!("{}", KEEP_ROTATED - 1)
        );
    }

    /// Gizlilik kuralı: çekirdekte günlüğe yalnız bu modül yazar; `log`/`println` gibi serbest
    /// metin kanalları kullanılmaz.
    #[test]
    fn serbest_gunluk_kanali_yok() {
        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut stack = vec![src];
        let banned = [
            "log::info",
            "log::warn",
            "log::error",
            "log::debug",
            "println!",
            "eprintln!",
        ];
        while let Some(d) = stack.pop() {
            for e in fs::read_dir(&d).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.extension().is_some_and(|x| x == "rs") && !p.ends_with("logging.rs") {
                    let text = fs::read_to_string(&p).unwrap();
                    for b in banned {
                        // Yalnız tam ad: önünde harf/rakam/_ olmamalı (ör. `dialog::error` sayılmaz).
                        let hit = text.match_indices(b).any(|(i, _)| {
                            !text[..i]
                                .chars()
                                .next_back()
                                .is_some_and(|c| c.is_alphanumeric() || c == '_')
                        });
                        assert!(!hit, "{} içinde yasak günlük kanalı: {b}", p.display());
                    }
                }
            }
        }
    }
}
