//! SQLite şema tanımları ve otomatik sürüm geçişleri (§7.1).

use rusqlite::Connection;

use super::DbError;

pub const LATEST_SCHEMA_VERSION: i32 = 2;

const MIGRATION_V1: &str = r#"
CREATE TABLE IF NOT EXISTS schema_migrations (
    version INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS folders (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    parent_id INTEGER REFERENCES folders(id) ON DELETE SET NULL,
    name TEXT NOT NULL,
    color TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS phrases (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    folder_id INTEGER REFERENCES folders(id) ON DELETE SET NULL,
    abbr TEXT NOT NULL,
    content TEXT NOT NULL,
    content_html TEXT,
    description TEXT NOT NULL DEFAULT '',
    case_mode TEXT NOT NULL DEFAULT 'keep_original',
    shortcut TEXT,
    enabled INTEGER NOT NULL DEFAULT 1,
    use_count INTEGER NOT NULL DEFAULT 0,
    last_used_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    deleted_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_phrases_abbr ON phrases(abbr);
CREATE INDEX IF NOT EXISTS idx_phrases_deleted ON phrases(deleted_at);
CREATE INDEX IF NOT EXISTS idx_phrases_folder ON phrases(folder_id);

CREATE TABLE IF NOT EXISTS phrase_versions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    phrase_id INTEGER NOT NULL REFERENCES phrases(id) ON DELETE CASCADE,
    abbr TEXT NOT NULL,
    content TEXT NOT NULL,
    content_html TEXT,
    description TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_phrase_versions_phrase ON phrase_versions(phrase_id);

CREATE TABLE IF NOT EXISTS tags (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS phrase_tags (
    phrase_id INTEGER NOT NULL REFERENCES phrases(id) ON DELETE CASCADE,
    tag_id INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (phrase_id, tag_id)
);

CREATE TABLE IF NOT EXISTS app_rules (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    phrase_id INTEGER NOT NULL REFERENCES phrases(id) ON DELETE CASCADE,
    kind TEXT NOT NULL CHECK(kind IN ('only', 'exclude')),
    process_name TEXT NOT NULL,
    title_contains TEXT
);

CREATE INDEX IF NOT EXISTS idx_app_rules_phrase ON app_rules(phrase_id);

-- Sadece ifade kimliği + zaman + program adı; yazılan metin ASLA kaydedilmez (§7.1).
CREATE TABLE IF NOT EXISTS usage_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    phrase_id INTEGER NOT NULL REFERENCES phrases(id) ON DELETE CASCADE,
    used_at TEXT NOT NULL DEFAULT (datetime('now')),
    process_name TEXT
);

CREATE INDEX IF NOT EXISTS idx_usage_events_phrase ON usage_events(phrase_id);
CREATE INDEX IF NOT EXISTS idx_usage_events_used_at ON usage_events(used_at);

CREATE TABLE IF NOT EXISTS clipboard_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    content TEXT NOT NULL,
    content_html TEXT,
    copied_at TEXT NOT NULL DEFAULT (datetime('now')),
    source_process TEXT
);

CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
"#;

const MIGRATION_V2: &str = r#"
CREATE TABLE IF NOT EXISTS phrase_folders (
    phrase_id INTEGER NOT NULL REFERENCES phrases(id) ON DELETE CASCADE,
    folder_id INTEGER NOT NULL REFERENCES folders(id) ON DELETE CASCADE,
    PRIMARY KEY (phrase_id, folder_id)
);

CREATE INDEX IF NOT EXISTS idx_phrase_folders_folder ON phrase_folders(folder_id);

INSERT OR IGNORE INTO phrase_folders (phrase_id, folder_id)
SELECT id, folder_id FROM phrases WHERE folder_id IS NOT NULL;
"#;

/// Veritabanını en güncel şema sürümüne geçirir.
pub fn run_migrations(conn: &mut Connection) -> Result<i32, DbError> {
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA foreign_keys = ON;
         PRAGMA synchronous = NORMAL;",
    )?;

    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        );",
    )?;

    let current: i32 = conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
        [],
        |row| row.get(0),
    )?;

    if current < 1 {
        let tx = conn.transaction()?;
        tx.execute_batch(MIGRATION_V1)?;
        tx.execute(
            "INSERT OR IGNORE INTO schema_migrations (version) VALUES (1)",
            [],
        )?;
        tx.commit()?;
    }

    if current < 2 {
        let tx = conn.transaction()?;
        tx.execute_batch(MIGRATION_V2)?;
        tx.execute(
            "INSERT OR IGNORE INTO schema_migrations (version) VALUES (2)",
            [],
        )?;
        tx.commit()?;
    }

    let final_version: i32 = conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
        [],
        |row| row.get(0),
    )?;

    Ok(final_version)
}
