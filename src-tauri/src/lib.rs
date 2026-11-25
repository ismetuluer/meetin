//! Meetin çekirdeği. Modül görevleri için bkz. `docs/MEETIN-SARTNAME.md` §10.

pub mod db;
pub mod hook;
pub mod inject;
pub mod io;
pub mod matcher;
pub mod selection;
pub mod vars;
// Klasör adı §10 gereği `windows/`; `windows` crate ile karışmasın diye modül adı `pencere`.
#[path = "windows/mod.rs"]
pub mod pencere;

/// Tauri uygulamasını başlatır.
pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("Tauri uygulaması başlatılamadı");
}

#[cfg(test)]
mod tests {
    #[test]
    fn surum_paket_ile_ayni() {
        let conf: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        assert_eq!(conf["version"], env!("CARGO_PKG_VERSION"));
    }
}
