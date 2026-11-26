//! SQLite depolama katmanı (§7.1, §7.2, §7.3).
//!
//! Tüm yazmalar işlem (transaction) içinde yapılır, silme işlemleri `deleted_at`
//! ile çöp kutusuna taşır.

pub mod backup;
pub mod schema;

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use chrono::{Local, NaiveDate};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::matcher::case::CaseMode;
use crate::matcher::{AppRule, AppRuleKind, PhraseEntry};

#[derive(Debug, Error)]
pub enum DbError {
    #[error("Veritabanı hatası: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("Dosya sistemi hatası: {0}")]
    Io(#[from] std::io::Error),
    #[error("Kilit alınamadı")]
    LockPoisoned,
    #[error("Kayıt bulunamadı: {0}")]
    NotFound(i64),
    #[error("Geçersiz veri: {0}")]
    InvalidInput(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhraseRecord {
    pub id: i64,
    pub folder_id: Option<i64>,
    #[serde(default)]
    pub folder_ids: Vec<i64>,
    pub abbr: String,
    pub content: String,
    pub content_html: Option<String>,
    pub description: String,
    pub case_mode: CaseMode,
    pub shortcut: Option<String>,
    pub enabled: bool,
    pub use_count: i64,
    pub last_used_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewPhrase {
    pub abbr: String,
    pub content: String,
    pub content_html: Option<String>,
    #[serde(default)]
    pub description: String,
    pub folder_id: Option<i64>,
    #[serde(default)]
    pub folder_ids: Vec<i64>,
    #[serde(default)]
    pub case_mode: CaseMode,
    pub shortcut: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePhrase {
    pub id: i64,
    pub abbr: String,
    pub content: String,
    pub content_html: Option<String>,
    #[serde(default)]
    pub description: String,
    pub folder_id: Option<i64>,
    #[serde(default)]
    pub folder_ids: Vec<i64>,
    #[serde(default)]
    pub case_mode: CaseMode,
    pub shortcut: Option<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub tags: Vec<String>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderRecord {
    pub id: i64,
    pub parent_id: Option<i64>,
    pub name: String,
    pub color: Option<String>,
    pub sort_order: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhraseVersionRecord {
    pub id: i64,
    pub phrase_id: i64,
    pub abbr: String,
    pub content: String,
    pub content_html: Option<String>,
    pub description: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardHistoryRecord {
    pub id: i64,
    pub content: String,
    pub content_html: Option<String>,
    pub copied_at: String,
    pub source_process: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopPhraseStat {
    pub id: i64,
    pub abbr: String,
    pub use_count: i64,
    pub last_used_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageStats {
    pub today_count: i64,
    pub week_count: i64,
    pub total_count: i64,
    pub saved_chars: i64,
    pub saved_minutes: f64,
    pub active_phrases: i64,
    pub top_phrases: Vec<TopPhraseStat>,
}

/// Veri dizinini belirler (§7.2):
/// EXE'nin yanında `local-data.flag` varsa `.\data\`, aksi hâlde `%APPDATA%\Meetin\`.
pub fn resolve_data_dir(exe_dir: &Path, appdata_dir: Option<&Path>) -> PathBuf {
    if exe_dir.join("local-data.flag").exists() {
        return exe_dir.join("data");
    }
    if let Some(appdata) = appdata_dir {
        return appdata.join("Meetin");
    }
    exe_dir.join("data")
}

/// Çalışma zamanında gerçek veri dizinini çözer.
pub fn runtime_data_dir() -> PathBuf {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."));
    let appdata = std::env::var_os("APPDATA").map(PathBuf::from);
    resolve_data_dir(&exe_dir, appdata.as_deref())
}

pub struct Storage {
    conn: Mutex<Connection>,
    db_path: PathBuf,
}

impl Storage {
    pub fn open(db_path: &Path) -> Result<Self, DbError> {
        if let Some(parent) = db_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        match Connection::open(db_path) {
            Ok(mut conn) => match schema::run_migrations(&mut conn) {
                Ok(_) => Ok(Self {
                    conn: Mutex::new(conn),
                    db_path: db_path.to_path_buf(),
                }),
                Err(err) if db_path.exists() => {
                    let ro = Connection::open_with_flags(
                        db_path,
                        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY
                            | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
                    )
                    .map_err(|_| err)?;
                    Ok(Self {
                        conn: Mutex::new(ro),
                        db_path: db_path.to_path_buf(),
                    })
                }
                Err(err) => Err(err),
            },
            Err(err) if db_path.exists() => {
                let ro = Connection::open_with_flags(
                    db_path,
                    rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY
                        | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
                )
                .map_err(|_| err)?;
                Ok(Self {
                    conn: Mutex::new(ro),
                    db_path: db_path.to_path_buf(),
                })
            }
            Err(err) => Err(err.into()),
        }
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    pub fn backups_dir(&self) -> PathBuf {
        self.db_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("backups")
    }

    pub fn ensure_daily_backup_today(&self) -> Result<Option<PathBuf>, DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::LockPoisoned)?;
        let today = Local::now().date_naive();
        backup::ensure_daily_backup(
            &conn,
            &self.backups_dir(),
            today,
            backup::DEFAULT_MAX_BACKUPS,
        )
    }

    pub fn create_backup_now(&self) -> Result<backup::BackupFileInfo, DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::LockPoisoned)?;
        backup::create_timestamped_backup(
            &conn,
            &self.backups_dir(),
            "",
            backup::DEFAULT_MAX_BACKUPS,
        )
    }

    pub fn list_backups(&self) -> Result<Vec<backup::BackupFileInfo>, DbError> {
        backup::list_backups(&self.backups_dir())
    }

    /// Yedek dosyasından geri yükler; öncesinde mevcut veritabanının emniyet yedeğini alır (§7.3, M6.5).
    pub fn restore_from_backup(&self, filename: &str) -> Result<backup::BackupFileInfo, DbError> {
        if filename.contains('/') || filename.contains('\\') || filename.contains("..") || !filename.ends_with(".db") {
            return Err(DbError::InvalidInput("Geçersiz yedek dosya adı".to_string()));
        }
        let backups_dir = self.backups_dir();
        let src_path = backups_dir.join(filename);
        if !src_path.is_file() {
            return Err(DbError::InvalidInput("Yedek dosyası bulunamadı".to_string()));
        }

        // Seçilen yedeğin geçerli bir Meetin veritabanı olduğunu doğrula
        {
            let probe = Connection::open(&src_path)?;
            let _: i64 = probe.query_row("SELECT COUNT(*) FROM phrases", [], |r| r.get(0))?;
        }

        let mut conn = self.conn.lock().map_err(|_| DbError::LockPoisoned)?;
        // Mevcut veritabanını bozmadan önce emniyet yedeği al
        let safety_copy = backup::create_timestamped_backup(
            &conn,
            &backups_dir,
            "pre-restore-",
            backup::DEFAULT_MAX_BACKUPS,
        )?;
        let _ = conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");

        // Windows dosya kilidini bırakmak için geçici bellek bağlantısına geç
        *conn = Connection::open_in_memory()?;

        for suffix in ["-wal", "-shm"] {
            let side = PathBuf::from(format!("{}{}", self.db_path.to_string_lossy(), suffix));
            if side.exists() {
                let _ = std::fs::remove_file(side);
            }
        }
        std::fs::copy(&src_path, &self.db_path)?;

        let mut reopened = Connection::open(&self.db_path)?;
        schema::run_migrations(&mut reopened)?;
        *conn = reopened;

        Ok(safety_copy)
    }

    pub fn create_backup_for_date(
        &self,
        backups_dir: &Path,
        date: NaiveDate,
        max_backups: usize,
    ) -> Result<PathBuf, DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::LockPoisoned)?;
        backup::create_backup(&conn, backups_dir, date, max_backups)
    }

    fn normalized_folder_ids(folder_id: Option<i64>, folder_ids: &[i64]) -> Vec<i64> {
        let mut out = Vec::new();
        for &fid in folder_ids {
            if !out.contains(&fid) {
                out.push(fid);
            }
        }
        if out.is_empty() {
            if let Some(fid) = folder_id {
                out.push(fid);
            }
        }
        out
    }

    fn sync_phrase_folders_tx(
        tx: &rusqlite::Transaction<'_>,
        phrase_id: i64,
        folder_ids: &[i64],
    ) -> Result<(), DbError> {
        tx.execute(
            "DELETE FROM phrase_folders WHERE phrase_id = ?1",
            params![phrase_id],
        )?;
        for &fid in folder_ids {
            tx.execute(
                "INSERT OR IGNORE INTO phrase_folders (phrase_id, folder_id) VALUES (?1, ?2)",
                params![phrase_id, fid],
            )?;
        }
        Ok(())
    }

    fn load_phrase_folders(
        conn: &Connection,
        phrase_id: i64,
        fallback_folder_id: Option<i64>,
    ) -> Result<Vec<i64>, DbError> {
        let mut stmt = conn.prepare(
            "SELECT folder_id FROM phrase_folders WHERE phrase_id = ?1 ORDER BY folder_id",
        )?;
        let rows = stmt.query_map(params![phrase_id], |r| r.get::<_, i64>(0))?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        if out.is_empty() {
            if let Some(fid) = fallback_folder_id {
                out.push(fid);
            }
        }
        Ok(out)
    }

    fn ensure_unique_abbr_tx(
        tx: &rusqlite::Transaction<'_>,
        abbr: &str,
        exclude_id: Option<i64>,
    ) -> Result<(), DbError> {
        let fold: bool = tx
            .query_row(
                "SELECT value FROM settings WHERE key = 'fold_dotted_i'",
                [],
                |r| r.get::<_, String>(0),
            )
            .optional()?
            .map(|v| v == "true")
            .unwrap_or(false);
        let target_norm = crate::matcher::case::normalize_abbr(abbr, fold);
        let mut stmt =
            tx.prepare("SELECT id, abbr FROM phrases WHERE deleted_at IS NULL")?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?;
        for row in rows {
            let (id, existing_abbr) = row?;
            if exclude_id == Some(id) {
                continue;
            }
            if crate::matcher::case::normalize_abbr(&existing_abbr, fold) == target_norm {
                return Err(DbError::InvalidInput(format!(
                    "Bu kısaltma zaten mevcut: {existing_abbr}"
                )));
            }
        }
        Ok(())
    }

    /// Yeni ifade ekler (transaction içinde).
    pub fn create_phrase(&self, input: &NewPhrase) -> Result<PhraseRecord, DbError> {
        let abbr = input.abbr.trim();
        if abbr.is_empty() {
            return Err(DbError::InvalidInput("Kısaltma boş olamaz".to_string()));
        }

        let folder_ids = Self::normalized_folder_ids(input.folder_id, &input.folder_ids);
        let primary_folder = folder_ids.first().copied();

        let mut conn = self.conn.lock().map_err(|_| DbError::LockPoisoned)?;
        let tx = conn.transaction()?;

        Self::ensure_unique_abbr_tx(&tx, abbr, None)?;

        tx.execute(
            "INSERT INTO phrases (folder_id, abbr, content, content_html, description, case_mode, shortcut)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                primary_folder,
                abbr,
                input.content,
                input.content_html,
                input.description,
                input.case_mode.as_str(),
                input.shortcut,
            ],
        )?;
        let id = tx.last_insert_rowid();

        Self::sync_phrase_folders_tx(&tx, id, &folder_ids)?;
        Self::sync_tags_tx(&tx, id, &input.tags)?;
        tx.commit()?;

        drop(conn);
        self.get_phrase(id)?.ok_or(DbError::NotFound(id))
    }

    /// Mevcut ifadeyi günceller ve önceki sürümü `phrase_versions` tablosuna kaydeder.
    pub fn update_phrase(&self, input: &UpdatePhrase) -> Result<PhraseRecord, DbError> {
        let abbr = input.abbr.trim();
        if abbr.is_empty() {
            return Err(DbError::InvalidInput("Kısaltma boş olamaz".to_string()));
        }

        let folder_ids = Self::normalized_folder_ids(input.folder_id, &input.folder_ids);
        let primary_folder = folder_ids.first().copied();

        let mut conn = self.conn.lock().map_err(|_| DbError::LockPoisoned)?;
        let tx = conn.transaction()?;

        // Önceki hâli sürüm geçmişine ekle
        let prev: Option<(String, String, Option<String>, String)> = tx
            .query_row(
                "SELECT abbr, content, content_html, description FROM phrases WHERE id = ?1",
                params![input.id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .optional()?;

        let Some((prev_abbr, prev_content, prev_html, prev_desc)) = prev else {
            return Err(DbError::NotFound(input.id));
        };

        Self::ensure_unique_abbr_tx(&tx, abbr, Some(input.id))?;

        if prev_abbr != abbr
            || prev_content != input.content
            || prev_html != input.content_html
            || prev_desc != input.description
        {
            tx.execute(
                "INSERT INTO phrase_versions (phrase_id, abbr, content, content_html, description)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![input.id, prev_abbr, prev_content, prev_html, prev_desc],
            )?;
        }

        tx.execute(
            "UPDATE phrases
             SET folder_id = ?1,
                 abbr = ?2,
                 content = ?3,
                 content_html = ?4,
                 description = ?5,
                 case_mode = ?6,
                 shortcut = ?7,
                 enabled = ?8,
                 updated_at = datetime('now')
             WHERE id = ?9",
            params![
                primary_folder,
                abbr,
                input.content,
                input.content_html,
                input.description,
                input.case_mode.as_str(),
                input.shortcut,
                if input.enabled { 1 } else { 0 },
                input.id,
            ],
        )?;

        Self::sync_phrase_folders_tx(&tx, input.id, &folder_ids)?;
        Self::sync_tags_tx(&tx, input.id, &input.tags)?;
        tx.commit()?;

        drop(conn);
        self.get_phrase(input.id)?.ok_or(DbError::NotFound(input.id))
    }

    /// İfadeyi sürükle-bırak ile klasöre taşır veya `copy = true` ise hedef klasöre de ekler.
    pub fn move_or_copy_phrase_folder(
        &self,
        phrase_id: i64,
        target_folder_id: Option<i64>,
        source_folder_id: Option<i64>,
        copy: bool,
    ) -> Result<PhraseRecord, DbError> {
        {
            let mut conn = self.conn.lock().map_err(|_| DbError::LockPoisoned)?;
            let tx = conn.transaction()?;

            let exists: Option<i64> = tx
                .query_row(
                    "SELECT id FROM phrases WHERE id = ?1",
                    params![phrase_id],
                    |r| r.get(0),
                )
                .optional()?;
            if exists.is_none() {
                return Err(DbError::NotFound(phrase_id));
            }

            match target_folder_id {
                None => {
                    if !copy {
                        tx.execute(
                            "DELETE FROM phrase_folders WHERE phrase_id = ?1",
                            params![phrase_id],
                        )?;
                        tx.execute(
                            "UPDATE phrases SET folder_id = NULL, updated_at = datetime('now') WHERE id = ?1",
                            params![phrase_id],
                        )?;
                    }
                }
                Some(tid) => {
                    if !copy {
                        if let Some(sid) = source_folder_id {
                            if sid != tid {
                                tx.execute(
                                    "DELETE FROM phrase_folders WHERE phrase_id = ?1 AND folder_id = ?2",
                                    params![phrase_id, sid],
                                )?;
                            }
                        } else {
                            tx.execute(
                                "DELETE FROM phrase_folders WHERE phrase_id = ?1",
                                params![phrase_id],
                            )?;
                        }
                    }
                    tx.execute(
                        "INSERT OR IGNORE INTO phrase_folders (phrase_id, folder_id) VALUES (?1, ?2)",
                        params![phrase_id, tid],
                    )?;
                    let first_folder: Option<i64> = tx
                        .query_row(
                            "SELECT folder_id FROM phrase_folders WHERE phrase_id = ?1 ORDER BY folder_id LIMIT 1",
                            params![phrase_id],
                            |r| r.get(0),
                        )
                        .optional()?;
                    tx.execute(
                        "UPDATE phrases SET folder_id = ?1, updated_at = datetime('now') WHERE id = ?2",
                        params![first_folder, phrase_id],
                    )?;
                }
            }
            tx.commit()?;
        }

        self.get_phrase(phrase_id)?.ok_or(DbError::NotFound(phrase_id))
    }

    /// İfadeyi çöp kutusuna taşır (`deleted_at` alanı dolar, §7.1).
    pub fn soft_delete_phrase(&self, id: i64) -> Result<(), DbError> {
        let mut conn = self.conn.lock().map_err(|_| DbError::LockPoisoned)?;
        let tx = conn.transaction()?;
        let affected = tx.execute(
            "UPDATE phrases SET deleted_at = datetime('now'), updated_at = datetime('now')
             WHERE id = ?1 AND deleted_at IS NULL",
            params![id],
        )?;
        if affected == 0 {
            return Err(DbError::NotFound(id));
        }
        tx.commit()?;
        Ok(())
    }

    /// Çöp kutusundaki ifadeyi geri yükler.
    pub fn restore_phrase(&self, id: i64) -> Result<(), DbError> {
        let mut conn = self.conn.lock().map_err(|_| DbError::LockPoisoned)?;
        let tx = conn.transaction()?;
        let abbr: Option<String> = tx
            .query_row("SELECT abbr FROM phrases WHERE id = ?1", params![id], |r| r.get(0))
            .optional()?;
        let Some(abbr) = abbr else {
            return Err(DbError::NotFound(id));
        };
        Self::ensure_unique_abbr_tx(&tx, &abbr, Some(id))?;
        let affected = tx.execute(
            "UPDATE phrases SET deleted_at = NULL, updated_at = datetime('now')
             WHERE id = ?1",
            params![id],
        )?;
        if affected == 0 {
            return Err(DbError::NotFound(id));
        }
        tx.commit()?;
        Ok(())
    }

    /// İfade genişletildiğinde kullanım sayacını ve `usage_events` kaydını günceller.
    pub fn record_usage(&self, phrase_id: i64, process_name: Option<&str>) -> Result<(), DbError> {
        let mut conn = self.conn.lock().map_err(|_| DbError::LockPoisoned)?;
        let tx = conn.transaction()?;
        tx.execute(
            "UPDATE phrases SET use_count = use_count + 1, last_used_at = datetime('now')
             WHERE id = ?1",
            params![phrase_id],
        )?;
        tx.execute(
            "INSERT INTO usage_events (phrase_id, process_name) VALUES (?1, ?2)",
            params![phrase_id, process_name],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// İsimle klasör bulur veya oluşturur (CSV içe aktarma için).
    pub fn get_or_create_folder(&self, name: &str) -> Result<i64, DbError> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(DbError::InvalidInput("Klasör adı boş olamaz".to_string()));
        }
        let mut conn = self.conn.lock().map_err(|_| DbError::LockPoisoned)?;
        let tx = conn.transaction()?;
        let existing: Option<i64> = tx
            .query_row(
                "SELECT id FROM folders WHERE name = ?1 LIMIT 1",
                params![trimmed],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(id) = existing {
            tx.commit()?;
            return Ok(id);
        }
        tx.execute("INSERT INTO folders (name) VALUES (?1)", params![trimmed])?;
        let id = tx.last_insert_rowid();
        tx.commit()?;
        Ok(id)
    }

    pub fn list_folders(&self) -> Result<Vec<FolderRecord>, DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::LockPoisoned)?;
        let mut stmt = conn.prepare(
            "SELECT id, parent_id, name, color, sort_order FROM folders ORDER BY sort_order, name",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(FolderRecord {
                id: r.get(0)?,
                parent_id: r.get(1)?,
                name: r.get(2)?,
                color: r.get(3)?,
                sort_order: r.get(4)?,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    pub fn get_phrase(&self, id: i64) -> Result<Option<PhraseRecord>, DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::LockPoisoned)?;
        let rec = conn
            .query_row(
                "SELECT id, folder_id, abbr, content, content_html, description, case_mode,
                        shortcut, enabled, use_count, last_used_at, created_at, updated_at, deleted_at
                 FROM phrases WHERE id = ?1",
                params![id],
                |r| {
                    Ok(PhraseRecord {
                        id: r.get(0)?,
                        folder_id: r.get(1)?,
                        folder_ids: Vec::new(),
                        abbr: r.get(2)?,
                        content: r.get(3)?,
                        content_html: r.get(4)?,
                        description: r.get(5)?,
                        case_mode: CaseMode::parse(&r.get::<_, String>(6)?),
                        shortcut: r.get(7)?,
                        enabled: r.get::<_, i64>(8)? != 0,
                        use_count: r.get(9)?,
                        last_used_at: r.get(10)?,
                        created_at: r.get(11)?,
                        updated_at: r.get(12)?,
                        deleted_at: r.get(13)?,
                        tags: Vec::new(),
                    })
                },
            )
            .optional()?;

        if let Some(mut phrase) = rec {
            phrase.folder_ids = Self::load_phrase_folders(&conn, phrase.id, phrase.folder_id)?;
            phrase.tags = Self::load_tags(&conn, phrase.id)?;
            Ok(Some(phrase))
        } else {
            Ok(None)
        }
    }

    pub fn list_phrases(&self, include_deleted: bool) -> Result<Vec<PhraseRecord>, DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::LockPoisoned)?;
        let sql = if include_deleted {
            "SELECT id, folder_id, abbr, content, content_html, description, case_mode,
                    shortcut, enabled, use_count, last_used_at, created_at, updated_at, deleted_at
             FROM phrases ORDER BY updated_at DESC, id DESC"
        } else {
            "SELECT id, folder_id, abbr, content, content_html, description, case_mode,
                    shortcut, enabled, use_count, last_used_at, created_at, updated_at, deleted_at
             FROM phrases WHERE deleted_at IS NULL ORDER BY updated_at DESC, id DESC"
        };

        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map([], |r| {
            Ok(PhraseRecord {
                id: r.get(0)?,
                folder_id: r.get(1)?,
                folder_ids: Vec::new(),
                abbr: r.get(2)?,
                content: r.get(3)?,
                content_html: r.get(4)?,
                description: r.get(5)?,
                case_mode: CaseMode::parse(&r.get::<_, String>(6)?),
                shortcut: r.get(7)?,
                enabled: r.get::<_, i64>(8)? != 0,
                use_count: r.get(9)?,
                last_used_at: r.get(10)?,
                created_at: r.get(11)?,
                updated_at: r.get(12)?,
                deleted_at: r.get(13)?,
                tags: Vec::new(),
            })
        })?;

        let mut out = Vec::new();
        for row in rows {
            let mut phrase = row?;
            phrase.folder_ids = Self::load_phrase_folders(&conn, phrase.id, phrase.folder_id)?;
            phrase.tags = Self::load_tags(&conn, phrase.id)?;
            out.push(phrase);
        }
        Ok(out)
    }

    /// Eşleme motorunu beslemek için etkin (silinmemiş) ifadeleri ve program kurallarını döndürür.
    pub fn load_matcher_entries(&self) -> Result<Vec<PhraseEntry>, DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::LockPoisoned)?;
        let mut stmt = conn.prepare(
            "SELECT id, abbr, content, content_html, case_mode, enabled
             FROM phrases
             WHERE deleted_at IS NULL AND enabled = 1",
        )?;

        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, Option<String>>(3)?,
                CaseMode::parse(&r.get::<_, String>(4)?),
                r.get::<_, i64>(5)? != 0,
            ))
        })?;

        let mut entries = Vec::new();
        for row in rows {
            let (id, abbr, content, content_html, case_mode, enabled) = row?;
            let app_rules = Self::load_app_rules(&conn, id)?;
            entries.push(PhraseEntry {
                id,
                abbr,
                content,
                content_html,
                case_mode,
                app_rules,
                enabled,
            });
        }
        Ok(entries)
    }

    pub fn get_phrase_app_rules(&self, phrase_id: i64) -> Result<Vec<AppRule>, DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::LockPoisoned)?;
        Self::load_app_rules(&conn, phrase_id)
    }

    pub fn set_phrase_app_rules(&self, phrase_id: i64, rules: &[AppRule]) -> Result<(), DbError> {
        let mut conn = self.conn.lock().map_err(|_| DbError::LockPoisoned)?;
        let tx = conn.transaction()?;
        tx.execute(
            "DELETE FROM app_rules WHERE phrase_id = ?1",
            params![phrase_id],
        )?;
        for rule in rules {
            let kind_str = match rule.kind {
                AppRuleKind::Only => "only",
                AppRuleKind::Exclude => "exclude",
            };
            let proc_name = rule.process_name.trim();
            if proc_name.is_empty() {
                continue;
            }
            tx.execute(
                "INSERT INTO app_rules (phrase_id, kind, process_name, title_contains)
                 VALUES (?1, ?2, ?3, ?4)",
                params![phrase_id, kind_str, proc_name, rule.title_contains],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<String>, DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::LockPoisoned)?;
        let val = conn
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                params![key],
                |r| r.get(0),
            )
            .optional()?;
        Ok(val)
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<(), DbError> {
        let mut conn = self.conn.lock().map_err(|_| DbError::LockPoisoned)?;
        let tx = conn.transaction()?;
        tx.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn create_folder(
        &self,
        name: &str,
        parent_id: Option<i64>,
        color: Option<&str>,
    ) -> Result<FolderRecord, DbError> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(DbError::InvalidInput("Klasör adı boş olamaz".to_string()));
        }
        let mut conn = self.conn.lock().map_err(|_| DbError::LockPoisoned)?;
        let tx = conn.transaction()?;
        let next_order: i64 = tx.query_row(
            "SELECT COALESCE(MAX(sort_order), 0) + 1 FROM folders",
            [],
            |r| r.get(0),
        )?;
        tx.execute(
            "INSERT INTO folders (parent_id, name, color, sort_order) VALUES (?1, ?2, ?3, ?4)",
            params![parent_id, trimmed, color, next_order],
        )?;
        let id = tx.last_insert_rowid();
        let record = tx.query_row(
            "SELECT id, parent_id, name, color, sort_order FROM folders WHERE id = ?1",
            params![id],
            |r| {
                Ok(FolderRecord {
                    id: r.get(0)?,
                    parent_id: r.get(1)?,
                    name: r.get(2)?,
                    color: r.get(3)?,
                    sort_order: r.get(4)?,
                })
            },
        )?;
        tx.commit()?;
        Ok(record)
    }

    pub fn delete_folder(&self, id: i64) -> Result<(), DbError> {
        let mut conn = self.conn.lock().map_err(|_| DbError::LockPoisoned)?;
        let tx = conn.transaction()?;
        tx.execute(
            "DELETE FROM phrase_folders WHERE folder_id = ?1",
            params![id],
        )?;
        tx.execute(
            "UPDATE phrases SET folder_id = (
                SELECT pf.folder_id FROM phrase_folders pf WHERE pf.phrase_id = phrases.id ORDER BY pf.folder_id LIMIT 1
             ), updated_at = datetime('now')
             WHERE folder_id = ?1",
            params![id],
        )?;
        tx.execute(
            "UPDATE folders SET parent_id = NULL WHERE parent_id = ?1",
            params![id],
        )?;
        let affected = tx.execute("DELETE FROM folders WHERE id = ?1", params![id])?;
        if affected == 0 {
            return Err(DbError::NotFound(id));
        }
        tx.commit()?;
        Ok(())
    }

    pub fn list_phrase_versions(
        &self,
        phrase_id: i64,
    ) -> Result<Vec<PhraseVersionRecord>, DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::LockPoisoned)?;
        let mut stmt = conn.prepare(
            "SELECT id, phrase_id, abbr, content, content_html, description, created_at
             FROM phrase_versions
             WHERE phrase_id = ?1
             ORDER BY id DESC
             LIMIT 50",
        )?;
        let rows = stmt.query_map(params![phrase_id], |r| {
            Ok(PhraseVersionRecord {
                id: r.get(0)?,
                phrase_id: r.get(1)?,
                abbr: r.get(2)?,
                content: r.get(3)?,
                content_html: r.get(4)?,
                description: r.get(5)?,
                created_at: r.get(6)?,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    pub fn restore_phrase_version(
        &self,
        phrase_id: i64,
        version_id: i64,
    ) -> Result<PhraseRecord, DbError> {
        {
            let mut conn = self.conn.lock().map_err(|_| DbError::LockPoisoned)?;
            let tx = conn.transaction()?;
            let target = tx
                .query_row(
                    "SELECT abbr, content, content_html, description
                     FROM phrase_versions
                     WHERE id = ?1 AND phrase_id = ?2",
                    params![version_id, phrase_id],
                    |r| {
                        Ok((
                            r.get::<_, String>(0)?,
                            r.get::<_, String>(1)?,
                            r.get::<_, Option<String>>(2)?,
                            r.get::<_, String>(3)?,
                        ))
                    },
                )
                .optional()?
                .ok_or(DbError::NotFound(version_id))?;

            let current = tx
                .query_row(
                    "SELECT abbr, content, content_html, description FROM phrases WHERE id = ?1",
                    params![phrase_id],
                    |r| {
                        Ok((
                            r.get::<_, String>(0)?,
                            r.get::<_, String>(1)?,
                            r.get::<_, Option<String>>(2)?,
                            r.get::<_, String>(3)?,
                        ))
                    },
                )
                .optional()?
                .ok_or(DbError::NotFound(phrase_id))?;

            tx.execute(
                "INSERT INTO phrase_versions (phrase_id, abbr, content, content_html, description)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![phrase_id, current.0, current.1, current.2, current.3],
            )?;

            tx.execute(
                "UPDATE phrases
                 SET abbr = ?1, content = ?2, content_html = ?3, description = ?4,
                     updated_at = datetime('now')
                 WHERE id = ?5",
                params![target.0, target.1, target.2, target.3, phrase_id],
            )?;
            tx.commit()?;
        }
        self.get_phrase(phrase_id)?
            .ok_or(DbError::NotFound(phrase_id))
    }

    pub fn hard_delete_phrase(&self, id: i64) -> Result<(), DbError> {
        let mut conn = self.conn.lock().map_err(|_| DbError::LockPoisoned)?;
        let tx = conn.transaction()?;
        let affected = tx.execute("DELETE FROM phrases WHERE id = ?1", params![id])?;
        if affected == 0 {
            return Err(DbError::NotFound(id));
        }
        tx.commit()?;
        Ok(())
    }

    pub fn empty_trash(&self) -> Result<usize, DbError> {
        let mut conn = self.conn.lock().map_err(|_| DbError::LockPoisoned)?;
        let tx = conn.transaction()?;
        let affected = tx.execute("DELETE FROM phrases WHERE deleted_at IS NOT NULL", [])?;
        tx.commit()?;
        Ok(affected)
    }

    pub fn purge_expired_trash(&self, retention_days: i64) -> Result<usize, DbError> {
        let days = retention_days.max(1);
        let modifier = format!("-{} days", days);
        let mut conn = self.conn.lock().map_err(|_| DbError::LockPoisoned)?;
        let tx = conn.transaction()?;
        let affected = tx.execute(
            "DELETE FROM phrases
             WHERE deleted_at IS NOT NULL
               AND deleted_at <= datetime('now', ?1)",
            params![modifier],
        )?;
        tx.commit()?;
        Ok(affected)
    }

    pub fn push_clipboard_history(
        &self,
        content: &str,
        content_html: Option<&str>,
        source_process: Option<&str>,
        max_items: usize,
    ) -> Result<bool, DbError> {
        if content.trim().is_empty() {
            return Ok(false);
        }
        let limit = max_items.max(1) as i64;
        let mut conn = self.conn.lock().map_err(|_| DbError::LockPoisoned)?;
        let tx = conn.transaction()?;
        let last_content: Option<String> = tx
            .query_row(
                "SELECT content FROM clipboard_history ORDER BY id DESC LIMIT 1",
                [],
                |r| r.get(0),
            )
            .optional()?;
        if last_content.as_deref() == Some(content) {
            return Ok(false);
        }
        tx.execute(
            "INSERT INTO clipboard_history (content, content_html, source_process)
             VALUES (?1, ?2, ?3)",
            params![content, content_html, source_process],
        )?;
        tx.execute(
            "DELETE FROM clipboard_history
             WHERE id NOT IN (
                 SELECT id FROM clipboard_history ORDER BY id DESC LIMIT ?1
             )",
            params![limit],
        )?;
        tx.commit()?;
        Ok(true)
    }

    pub fn list_clipboard_history(
        &self,
        limit: usize,
    ) -> Result<Vec<ClipboardHistoryRecord>, DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::LockPoisoned)?;
        let lim = limit.clamp(1, 500) as i64;
        let mut stmt = conn.prepare(
            "SELECT id, content, content_html, copied_at, source_process
             FROM clipboard_history
             ORDER BY id DESC
             LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![lim], |r| {
            Ok(ClipboardHistoryRecord {
                id: r.get(0)?,
                content: r.get(1)?,
                content_html: r.get(2)?,
                copied_at: r.get(3)?,
                source_process: r.get(4)?,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    pub fn clear_clipboard_history(&self) -> Result<usize, DbError> {
        let mut conn = self.conn.lock().map_err(|_| DbError::LockPoisoned)?;
        let tx = conn.transaction()?;
        let affected = tx.execute("DELETE FROM clipboard_history", [])?;
        tx.commit()?;
        Ok(affected)
    }

    pub fn trim_clipboard_history(&self, limit: usize) -> Result<(), DbError> {
        let lim = limit.clamp(1, 500) as i64;
        let mut conn = self.conn.lock().map_err(|_| DbError::LockPoisoned)?;
        let tx = conn.transaction()?;
        tx.execute(
            "DELETE FROM clipboard_history
             WHERE id NOT IN (SELECT id FROM clipboard_history ORDER BY id DESC LIMIT ?1)",
            params![lim],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn get_usage_stats(&self) -> Result<UsageStats, DbError> {
        let conn = self.conn.lock().map_err(|_| DbError::LockPoisoned)?;
        let today_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM usage_events WHERE date(used_at) = date('now')",
            [],
            |r| r.get(0),
        )?;
        let week_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM usage_events WHERE used_at >= datetime('now', '-7 days')",
            [],
            |r| r.get(0),
        )?;
        let total_count: i64 =
            conn.query_row("SELECT COUNT(*) FROM usage_events", [], |r| r.get(0))?;
        let saved_chars: i64 = conn.query_row(
            "SELECT COALESCE(SUM(MAX(0, LENGTH(p.content) - LENGTH(p.abbr))), 0)
             FROM usage_events u
             INNER JOIN phrases p ON p.id = u.phrase_id",
            [],
            |r| r.get(0),
        )?;
        let active_phrases: i64 = conn.query_row(
            "SELECT COUNT(*) FROM phrases WHERE deleted_at IS NULL AND enabled = 1",
            [],
            |r| r.get(0),
        )?;
        let mut stmt = conn.prepare(
            "SELECT id, abbr, use_count, last_used_at
             FROM phrases
             WHERE deleted_at IS NULL AND use_count > 0
             ORDER BY use_count DESC, last_used_at DESC
             LIMIT 20",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(TopPhraseStat {
                id: r.get(0)?,
                abbr: r.get(1)?,
                use_count: r.get(2)?,
                last_used_at: r.get(3)?,
            })
        })?;
        let mut top_phrases = Vec::new();
        for row in rows {
            top_phrases.push(row?);
        }
        let saved_minutes = (saved_chars as f64) / 200.0;
        Ok(UsageStats {
            today_count,
            week_count,
            total_count,
            saved_chars,
            saved_minutes,
            active_phrases,
            top_phrases,
        })
    }

    fn load_tags(conn: &Connection, phrase_id: i64) -> Result<Vec<String>, DbError> {
        let mut stmt = conn.prepare(
            "SELECT t.name FROM tags t
             INNER JOIN phrase_tags pt ON pt.tag_id = t.id
             WHERE pt.phrase_id = ?1
             ORDER BY t.name",
        )?;
        let rows = stmt.query_map(params![phrase_id], |r| r.get(0))?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    fn load_app_rules(conn: &Connection, phrase_id: i64) -> Result<Vec<AppRule>, DbError> {
        let mut stmt = conn.prepare(
            "SELECT kind, process_name, title_contains FROM app_rules WHERE phrase_id = ?1",
        )?;
        let rows = stmt.query_map(params![phrase_id], |r| {
            let kind_str: String = r.get(0)?;
            let kind = if kind_str == "exclude" {
                AppRuleKind::Exclude
            } else {
                AppRuleKind::Only
            };
            Ok(AppRule {
                kind,
                process_name: r.get(1)?,
                title_contains: r.get(2)?,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    fn sync_tags_tx(
        tx: &rusqlite::Transaction<'_>,
        phrase_id: i64,
        tags: &[String],
    ) -> Result<(), DbError> {
        tx.execute(
            "DELETE FROM phrase_tags WHERE phrase_id = ?1",
            params![phrase_id],
        )?;
        for raw in tags {
            let tag = raw.trim();
            if tag.is_empty() {
                continue;
            }
            tx.execute("INSERT OR IGNORE INTO tags (name) VALUES (?1)", params![tag])?;
            let tag_id: i64 =
                tx.query_row("SELECT id FROM tags WHERE name = ?1", params![tag], |r| {
                    r.get(0)
                })?;
            tx.execute(
                "INSERT OR IGNORE INTO phrase_tags (phrase_id, tag_id) VALUES (?1, ?2)",
                params![phrase_id, tag_id],
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn migrations_are_idempotent_and_crud_works() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("meetin.db");

        // İlk açılış -> v1 şeması
        let storage = Storage::open(&db_path).unwrap();
        // İkinci açılış -> değişiklik olmadan geçer
        drop(storage);
        let storage = Storage::open(&db_path).unwrap();

        let created = storage
            .create_phrase(&NewPhrase {
                abbr: "CİPY".to_string(),
                content: "Claude ile Proje Yapmak".to_string(),
                content_html: None,
                description: "Örnek kısaltma".to_string(),
                folder_id: None,
                folder_ids: vec![],
                case_mode: CaseMode::MatchTyped,
                shortcut: None,
                tags: vec!["genel".to_string(), "yapay-zeka".to_string()],
            })
            .unwrap();

        assert_eq!(created.abbr, "CİPY");
        assert_eq!(created.tags, vec!["genel", "yapay-zeka"]);

        // Güncelleme
        let updated = storage
            .update_phrase(&UpdatePhrase {
                id: created.id,
                abbr: "CİPY".to_string(),
                content: "Claude ile Proje Yapmak v2".to_string(),
                content_html: None,
                description: "Güncellendi".to_string(),
                folder_id: None,
                folder_ids: vec![],
                case_mode: CaseMode::KeepOriginal,
                shortcut: None,
                enabled: true,
                tags: vec!["genel".to_string()],
            })
            .unwrap();
        assert_eq!(updated.content, "Claude ile Proje Yapmak v2");
        assert_eq!(updated.case_mode, CaseMode::KeepOriginal);
        assert_eq!(updated.tags, vec!["genel"]);

        // Soft delete -> listeden düşer, include_deleted ile görünür
        storage.soft_delete_phrase(created.id).unwrap();
        assert_eq!(storage.list_phrases(false).unwrap().len(), 0);
        assert_eq!(storage.list_phrases(true).unwrap().len(), 1);

        // Restore
        storage.restore_phrase(created.id).unwrap();
        assert_eq!(storage.list_phrases(false).unwrap().len(), 1);
    }

    #[test]
    fn portable_flag_selects_local_data_dir() {
        let dir = TempDir::new().unwrap();
        let appdata = TempDir::new().unwrap();

        // local-data.flag yokken %APPDATA%\Meetin
        let installed = resolve_data_dir(dir.path(), Some(appdata.path()));
        assert_eq!(installed, appdata.path().join("Meetin"));

        // local-data.flag varken .\data
        std::fs::write(dir.path().join("local-data.flag"), "").unwrap();
        let portable = resolve_data_dir(dir.path(), Some(appdata.path()));
        assert_eq!(portable, dir.path().join("data"));
    }

    #[test]
    fn duplicate_abbr_is_rejected() {
        let dir = TempDir::new().unwrap();
        let storage = Storage::open(&dir.path().join("meetin.db")).unwrap();

        storage
            .create_phrase(&NewPhrase {
                abbr: "selam".to_string(),
                content: "Merhaba dünya".to_string(),
                content_html: None,
                description: String::new(),
                folder_id: None,
                folder_ids: vec![],
                case_mode: CaseMode::MatchTyped,
                shortcut: None,
                tags: vec![],
            })
            .unwrap();

        let dup = storage.create_phrase(&NewPhrase {
            abbr: "SELAM".to_string(),
            content: "Farklı içerik".to_string(),
            content_html: None,
            description: String::new(),
            folder_id: None,
            folder_ids: vec![],
            case_mode: CaseMode::MatchTyped,
            shortcut: None,
            tags: vec![],
        });
        assert!(dup.is_err());

        // Türkçe i/İ ayrımı: cipy ile CİPY çakışır
        storage
            .create_phrase(&NewPhrase {
                abbr: "cipy".to_string(),
                content: "Deneme".to_string(),
                content_html: None,
                description: String::new(),
                folder_id: None,
                folder_ids: vec![],
                case_mode: CaseMode::MatchTyped,
                shortcut: None,
                tags: vec![],
            })
            .unwrap();
        let dup_tr = storage.create_phrase(&NewPhrase {
            abbr: "CİPY".to_string(),
            content: "Deneme 2".to_string(),
            content_html: None,
            description: String::new(),
            folder_id: None,
            folder_ids: vec![],
            case_mode: CaseMode::MatchTyped,
            shortcut: None,
            tags: vec![],
        });
        assert!(dup_tr.is_err());
    }

    #[test]
    fn phrase_can_belong_to_multiple_folders_via_copy_and_move() {
        let dir = TempDir::new().unwrap();
        let storage = Storage::open(&dir.path().join("meetin.db")).unwrap();
        let f1 = storage.create_folder("Birinci", None, None).unwrap();
        let f2 = storage.create_folder("İkinci", None, None).unwrap();

        let p = storage
            .create_phrase(&NewPhrase {
                abbr: "kl".to_string(),
                content: "Klasör testi".to_string(),
                content_html: None,
                description: String::new(),
                folder_id: Some(f1.id),
                folder_ids: vec![f1.id],
                case_mode: CaseMode::MatchTyped,
                shortcut: None,
                tags: vec![],
            })
            .unwrap();
        assert_eq!(p.folder_ids, vec![f1.id]);

        // Ctrl+sürükle (copy = true) ile f2'ye de eklenir
        let copied = storage
            .move_or_copy_phrase_folder(p.id, Some(f2.id), Some(f1.id), true)
            .unwrap();
        assert_eq!(copied.folder_ids, vec![f1.id, f2.id]);

        // Normal sürükle (copy = false) f1 -> f2 f1'i kaldırır, yalnızca f2 kalır
        let moved = storage
            .move_or_copy_phrase_folder(p.id, Some(f2.id), Some(f1.id), false)
            .unwrap();
        assert_eq!(moved.folder_ids, vec![f2.id]);
    }

    #[test]
    fn folder_create_and_delete_detaches_phrases() {
        let dir = TempDir::new().unwrap();
        let storage = Storage::open(&dir.path().join("meetin.db")).unwrap();
        let folder = storage.create_folder("İmzalar", None, None).unwrap();
        assert_eq!(storage.list_folders().unwrap().len(), 1);

        let phrase = storage
            .create_phrase(&NewPhrase {
                abbr: "imz".to_string(),
                content: "Saygılarımla".to_string(),
                content_html: None,
                description: String::new(),
                folder_id: Some(folder.id),
                folder_ids: vec![folder.id],
                case_mode: CaseMode::MatchTyped,
                shortcut: None,
                tags: vec![],
            })
            .unwrap();
        assert_eq!(phrase.folder_id, Some(folder.id));

        storage.delete_folder(folder.id).unwrap();
        assert_eq!(storage.list_folders().unwrap().len(), 0);
        let detached = storage.get_phrase(phrase.id).unwrap().unwrap();
        assert_eq!(detached.folder_id, None);
    }

    #[test]
    fn phrase_version_history_and_restore_works() {
        let dir = TempDir::new().unwrap();
        let storage = Storage::open(&dir.path().join("meetin.db")).unwrap();
        let created = storage
            .create_phrase(&NewPhrase {
                abbr: "selam".to_string(),
                content: "İlk metin".to_string(),
                content_html: None,
                description: "v1".to_string(),
                folder_id: None,
                folder_ids: vec![],
                case_mode: CaseMode::MatchTyped,
                shortcut: None,
                tags: vec![],
            })
            .unwrap();

        storage
            .update_phrase(&UpdatePhrase {
                id: created.id,
                abbr: "selam".to_string(),
                content: "İkinci metin".to_string(),
                content_html: None,
                description: "v2".to_string(),
                folder_id: None,
                folder_ids: vec![],
                case_mode: CaseMode::MatchTyped,
                shortcut: None,
                enabled: true,
                tags: vec![],
            })
            .unwrap();

        let versions = storage.list_phrase_versions(created.id).unwrap();
        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0].content, "İlk metin");

        let restored = storage
            .restore_phrase_version(created.id, versions[0].id)
            .unwrap();
        assert_eq!(restored.content, "İlk metin");
        assert_eq!(storage.list_phrase_versions(created.id).unwrap().len(), 2);
    }

    #[test]
    fn trash_hard_delete_empty_and_purge_works() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("meetin.db");
        let storage = Storage::open(&db_path).unwrap();
        let p1 = storage
            .create_phrase(&NewPhrase {
                abbr: "a1".to_string(),
                content: "bir".to_string(),
                content_html: None,
                description: String::new(),
                folder_id: None,
                folder_ids: vec![],
                case_mode: CaseMode::MatchTyped,
                shortcut: None,
                tags: vec![],
            })
            .unwrap();
        let p2 = storage
            .create_phrase(&NewPhrase {
                abbr: "a2".to_string(),
                content: "iki".to_string(),
                content_html: None,
                description: String::new(),
                folder_id: None,
                folder_ids: vec![],
                case_mode: CaseMode::MatchTyped,
                shortcut: None,
                tags: vec![],
            })
            .unwrap();

        storage.soft_delete_phrase(p1.id).unwrap();
        storage.soft_delete_phrase(p2.id).unwrap();

        // p1'in silinme tarihini 35 gün geriye çek
        {
            let conn = storage.conn.lock().unwrap();
            conn.execute(
                "UPDATE phrases SET deleted_at = datetime('now', '-35 days') WHERE id = ?1",
                params![p1.id],
            )
            .unwrap();
        }
        assert_eq!(storage.purge_expired_trash(30).unwrap(), 1);
        assert_eq!(storage.list_phrases(true).unwrap().len(), 1);

        assert_eq!(storage.empty_trash().unwrap(), 1);
        assert_eq!(storage.list_phrases(true).unwrap().len(), 0);
    }

    #[test]
    fn clipboard_history_dedup_and_trim_works() {
        let dir = TempDir::new().unwrap();
        let storage = Storage::open(&dir.path().join("meetin.db")).unwrap();

        assert!(storage
            .push_clipboard_history("birinci", None, Some("notepad.exe"), 3)
            .unwrap());
        // Ardışık aynı içerik tekrar eklenmez
        assert!(!storage
            .push_clipboard_history("birinci", None, Some("notepad.exe"), 3)
            .unwrap());
        storage
            .push_clipboard_history("ikinci", None, None, 3)
            .unwrap();
        storage
            .push_clipboard_history("üçüncü", None, None, 3)
            .unwrap();
        storage
            .push_clipboard_history("dördüncü", None, None, 3)
            .unwrap();

        let items = storage.list_clipboard_history(10).unwrap();
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].content, "dördüncü");
        assert_eq!(items[2].content, "ikinci");

        assert_eq!(storage.clear_clipboard_history().unwrap(), 3);
        assert!(storage.list_clipboard_history(10).unwrap().is_empty());
    }

    #[test]
    fn usage_stats_computes_counts_and_top_phrases() {
        let dir = TempDir::new().unwrap();
        let storage = Storage::open(&dir.path().join("meetin.db")).unwrap();
        let p = storage
            .create_phrase(&NewPhrase {
                abbr: "mr".to_string(),
                content: "Merhaba dünya".to_string(),
                content_html: None,
                description: String::new(),
                folder_id: None,
                folder_ids: vec![],
                case_mode: CaseMode::MatchTyped,
                shortcut: None,
                tags: vec![],
            })
            .unwrap();
        storage.record_usage(p.id, Some("Code.exe")).unwrap();
        storage.record_usage(p.id, Some("Code.exe")).unwrap();

        let stats = storage.get_usage_stats().unwrap();
        assert_eq!(stats.today_count, 2);
        assert_eq!(stats.week_count, 2);
        assert_eq!(stats.total_count, 2);
        assert_eq!(stats.active_phrases, 1);
        assert!(stats.saved_chars > 0);
        assert_eq!(stats.top_phrases.len(), 1);
        assert_eq!(stats.top_phrases[0].abbr, "mr");
        assert_eq!(stats.top_phrases[0].use_count, 2);
    }

    #[test]
    fn opens_readonly_database_gracefully() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("meetin.db");
        {
            let storage = Storage::open(&db_path).unwrap();
            storage
                .create_phrase(&NewPhrase {
                    abbr: "ro".to_string(),
                    content: "Salt okunur ifade".to_string(),
                    content_html: None,
                    description: String::new(),
                    folder_id: None,
                    folder_ids: vec![],
                    case_mode: CaseMode::MatchTyped,
                    shortcut: None,
                    tags: vec![],
                })
                .unwrap();
            let conn = storage.conn.lock().unwrap();
            let _ = conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE); PRAGMA journal_mode = DELETE;");
        }

        let mut perms = std::fs::metadata(&db_path).unwrap().permissions();
        perms.set_readonly(true);
        std::fs::set_permissions(&db_path, perms).unwrap();

        let ro_storage = Storage::open(&db_path).unwrap();
        let list = ro_storage.list_phrases(false).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].abbr, "ro");
        drop(ro_storage);

        let mut reset_perms = std::fs::metadata(&db_path).unwrap().permissions();
        #[allow(clippy::permissions_set_readonly_false)]
        reset_perms.set_readonly(false);
        let _ = std::fs::set_permissions(&db_path, reset_perms);
    }
}
