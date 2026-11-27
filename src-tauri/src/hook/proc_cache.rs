//! Ön plandaki işlemin adı için tek girişli önbellek (R1.6).
//!
//! Klavye kancası her tuşta etkin programın adını ister (dışlanan programlar, program
//! kuralları). Adı bulmak süreç açmayı gerektirdiği için pahalıdır; ön plandaki işlem
//! değişmedikçe aynı ad kullanılır. Kilit tutuluyorsa beklenmez, önbellek atlanır.

use std::sync::Mutex;

pub struct ProcessNameCache {
    entry: Mutex<Option<(u32, Option<String>)>>,
}

impl ProcessNameCache {
    pub const fn new() -> Self {
        Self {
            entry: Mutex::new(None),
        }
    }

    /// PID önbellekteyse adını döndürür (ad bulunamamışsa `Some(None)`).
    pub fn lookup(&self, pid: u32) -> Option<Option<String>> {
        if pid == 0 {
            return None;
        }
        let guard = self.entry.try_lock().ok()?;
        match guard.as_ref() {
            Some((p, name)) if *p == pid => Some(name.clone()),
            _ => None,
        }
    }

    pub fn store(&self, pid: u32, name: Option<String>) {
        if pid == 0 {
            return;
        }
        if let Ok(mut guard) = self.entry.try_lock() {
            *guard = Some((pid, name));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ayni_pid_onbellekten_gelir() {
        let c = ProcessNameCache::new();
        assert_eq!(c.lookup(42), None);
        c.store(42, Some("WINWORD.EXE".into()));
        assert_eq!(c.lookup(42), Some(Some("WINWORD.EXE".into())));
    }

    #[test]
    fn pid_degisince_yeniden_sorgulanir() {
        let c = ProcessNameCache::new();
        c.store(42, Some("WINWORD.EXE".into()));
        assert_eq!(c.lookup(7), None);
        c.store(7, Some("chrome.exe".into()));
        assert_eq!(c.lookup(42), None);
        assert_eq!(c.lookup(7), Some(Some("chrome.exe".into())));
    }

    #[test]
    fn adi_bulunamayan_islem_de_onbelleklenir() {
        // Yönetici yetkili pencerede ad alınamaz; her tuşta yeniden denenmez.
        let c = ProcessNameCache::new();
        c.store(9, None);
        assert_eq!(c.lookup(9), Some(None));
    }

    #[test]
    fn pid_sifir_onbelleklenmez() {
        let c = ProcessNameCache::new();
        c.store(0, Some("x".into()));
        assert_eq!(c.lookup(0), None);
    }
}
