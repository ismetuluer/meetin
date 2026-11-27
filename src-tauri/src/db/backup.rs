//! Otomatik SQLite yedekleme (`VACUUM INTO`) ve rotasyon (§7.3, M1.4).

use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Local, NaiveDate};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use super::DbError;

pub const DEFAULT_MAX_BACKUPS: usize = 30;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupFileInfo {
    pub filename: String,
    pub path: String,
    pub size_bytes: u64,
    pub modified_at: String,
}

/// Belirli bir tarih için yedek dosya adını döndürür: `meetin-YYYY-MM-DD.db`.
pub fn backup_filename(date: NaiveDate) -> String {
    format!("meetin-{}.db", date.format("%Y-%m-%d"))
}

/// `VACUUM INTO` kullanarak tutarlı bir SQLite yedeği alır.
/// Aynı günün yedeği zaten varsa geçici dosyaya alıp atomik olarak yeniler.
pub fn create_backup(
    conn: &Connection,
    backups_dir: &Path,
    date: NaiveDate,
    max_backups: usize,
) -> Result<PathBuf, DbError> {
    fs::create_dir_all(backups_dir)?;

    let final_path = backups_dir.join(backup_filename(date));
    let tmp_path = backups_dir.join(format!(".{}.tmp", backup_filename(date)));

    if tmp_path.exists() {
        let _ = fs::remove_file(&tmp_path);
    }

    // SQLite VACUUM INTO tek tırnak kaçışı
    let escaped = tmp_path.to_string_lossy().replace('\'', "''");
    conn.execute_batch(&format!("VACUUM INTO '{}';", escaped))?;

    if final_path.exists() {
        fs::remove_file(&final_path)?;
    }
    fs::rename(&tmp_path, &final_path)?;

    prune_old_backups(backups_dir, max_backups)?;
    Ok(final_path)
}

/// Günün yedeği yoksa alır (ilk açılış için §7.3).
pub fn ensure_daily_backup(
    conn: &Connection,
    backups_dir: &Path,
    date: NaiveDate,
    max_backups: usize,
) -> Result<Option<PathBuf>, DbError> {
    let target = backups_dir.join(backup_filename(date));
    if target.exists() {
        return Ok(None);
    }
    let created = create_backup(conn, backups_dir, date, max_backups)?;
    Ok(Some(created))
}

/// `backups_dir` içindeki `meetin-YYYY-MM-DD.db` dosyalarını tarihe göre sıralar
/// ve `max_backups` adedinden eski olanları siler.
pub fn prune_old_backups(backups_dir: &Path, max_backups: usize) -> Result<usize, DbError> {
    if !backups_dir.exists() || max_backups == 0 {
        return Ok(0);
    }

    let mut entries: Vec<(String, PathBuf)> = Vec::new();
    for entry in fs::read_dir(backups_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.starts_with("meetin-") && name.ends_with(".db") {
                entries.push((name.to_string(), path));
            }
        }
    }

    // YYYY-MM-DD biçimi sözlük sırasıyla tarih sırasına eşittir
    entries.sort_by(|a, b| a.0.cmp(&b.0));

    let mut deleted = 0;
    if entries.len() > max_backups {
        let to_remove = entries.len() - max_backups;
        for (_, path) in entries.into_iter().take(to_remove) {
            fs::remove_file(path)?;
            deleted += 1;
        }
    }
    Ok(deleted)
}

/// Anlık veya geri yükleme öncesi zaman damgalı SQLite yedeği (`VACUUM INTO`) alır (§7.3, M6.5).
pub fn create_timestamped_backup(
    conn: &Connection,
    backups_dir: &Path,
    prefix: &str,
    max_backups: usize,
) -> Result<BackupFileInfo, DbError> {
    fs::create_dir_all(backups_dir)?;
    let now = Local::now();
    let filename = format!("meetin-{}{}.db", prefix, now.format("%Y-%m-%d-%H%M%S"));
    let final_path = backups_dir.join(&filename);
    let tmp_path = backups_dir.join(format!(".{}.tmp", filename));

    if tmp_path.exists() {
        let _ = fs::remove_file(&tmp_path);
    }

    let escaped = tmp_path.to_string_lossy().replace('\'', "''");
    conn.execute_batch(&format!("VACUUM INTO '{}';", escaped))?;

    if final_path.exists() {
        fs::remove_file(&final_path)?;
    }
    fs::rename(&tmp_path, &final_path)?;
    prune_old_backups(backups_dir, max_backups)?;

    let meta = fs::metadata(&final_path)?;
    let modified_at = meta
        .modified()
        .ok()
        .map(|t| {
            let dt: DateTime<Local> = t.into();
            dt.format("%Y-%m-%d %H:%M:%S").to_string()
        })
        .unwrap_or_else(|| now.format("%Y-%m-%d %H:%M:%S").to_string());

    Ok(BackupFileInfo {
        filename,
        path: final_path.to_string_lossy().into_owned(),
        size_bytes: meta.len(),
        modified_at,
    })
}

/// Yedek klasöründeki tüm `meetin-*.db` dosyalarını en yeniden eskiye listeler.
pub fn list_backups(backups_dir: &Path) -> Result<Vec<BackupFileInfo>, DbError> {
    if !backups_dir.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in fs::read_dir(backups_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !(name.starts_with("meetin-") && name.ends_with(".db")) {
            continue;
        }
        let meta = fs::metadata(&path)?;
        let modified_at = meta
            .modified()
            .ok()
            .map(|t| {
                let dt: DateTime<Local> = t.into();
                dt.format("%Y-%m-%d %H:%M:%S").to_string()
            })
            .unwrap_or_default();
        out.push(BackupFileInfo {
            filename: name.to_string(),
            path: path.to_string_lossy().into_owned(),
            size_bytes: meta.len(),
            modified_at,
        });
    }
    out.sort_by(|a, b| {
        b.modified_at
            .cmp(&a.modified_at)
            .then_with(|| b.filename.cmp(&a.filename))
    });
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{NewPhrase, Storage};
    use crate::matcher::case::CaseMode;
    use tempfile::TempDir;

    #[test]
    fn backup_creates_valid_copy_and_rotates_after_30() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("meetin.db");
        let backups_dir = dir.path().join("backups");

        let storage = Storage::open(&db_path).unwrap();
        storage
            .create_phrase(&NewPhrase {
                abbr: "MİPY".to_string(),
                content: "Metin ile Pratik Yazım".to_string(),
                content_html: None,
                description: "Test".to_string(),
                folder_id: None,
                folder_ids: vec![],
                case_mode: CaseMode::MatchTyped,
                shortcut: None,
                tags: vec![],
            })
            .unwrap();

        let base = NaiveDate::from_ymd_opt(2026, 10, 1).unwrap();
        // 31 farklı gün için yedek al
        for offset in 0..31 {
            let day = base + chrono::Days::new(offset);
            storage
                .create_backup_for_date(&backups_dir, day, DEFAULT_MAX_BACKUPS)
                .unwrap();
        }

        // İlk gün silinmiş olmalı, son 30 gün kalmalı
        assert!(!backups_dir.join("meetin-2026-10-01.db").exists());
        let latest = backups_dir.join("meetin-2026-10-31.db");
        assert!(latest.exists());

        let count = fs::read_dir(&backups_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().ends_with(".db"))
            .count();
        assert_eq!(count, 30);

        // Yedek dosyası açılabilir ve içerik birebir olmalı
        let restored = Storage::open(&latest).unwrap();
        let phrases = restored.list_phrases(false).unwrap();
        assert_eq!(phrases.len(), 1);
        assert_eq!(phrases[0].abbr, "MİPY");
        assert_eq!(phrases[0].content, "Metin ile Pratik Yazım");
    }

    #[test]
    fn restore_from_backup_preserves_safety_copy_and_restores_data() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("meetin.db");
        let storage = Storage::open(&db_path).unwrap();

        storage
            .create_phrase(&NewPhrase {
                abbr: "ilk".to_string(),
                content: "İlk ifade".to_string(),
                content_html: None,
                description: String::new(),
                folder_id: None,
                folder_ids: vec![],
                case_mode: CaseMode::MatchTyped,
                shortcut: None,
                tags: vec![],
            })
            .unwrap();

        let snap = storage.create_backup_now().unwrap();
        assert!(storage
            .list_backups()
            .unwrap()
            .iter()
            .any(|b| b.filename == snap.filename));

        storage
            .create_phrase(&NewPhrase {
                abbr: "sonra".to_string(),
                content: "Sonraki ifade".to_string(),
                content_html: None,
                description: String::new(),
                folder_id: None,
                folder_ids: vec![],
                case_mode: CaseMode::MatchTyped,
                shortcut: None,
                tags: vec![],
            })
            .unwrap();
        assert_eq!(storage.list_phrases(false).unwrap().len(), 2);

        let _restored_info = storage.restore_from_backup(&snap.filename).unwrap();
        let restored_phrases = storage.list_phrases(false).unwrap();
        assert_eq!(restored_phrases.len(), 1);
        assert_eq!(restored_phrases[0].abbr, "ilk");
        assert!(storage
            .list_backups()
            .unwrap()
            .iter()
            .any(|b| b.filename.contains("pre-restore-")));
    }
}
