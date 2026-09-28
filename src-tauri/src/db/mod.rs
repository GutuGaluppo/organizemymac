//! Local SQLite database: scan history, operation log, ignore list, settings and local-only metrics.
//! Nothing here ever leaves the Mac.

use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::error::AppResult;

const MIGRATIONS: &[&str] = &[
    // v1
    "CREATE TABLE scans (
        id TEXT PRIMARY KEY,
        module TEXT NOT NULL,
        root TEXT NOT NULL,
        started_at INTEGER NOT NULL,
        finished_at INTEGER,
        files INTEGER NOT NULL DEFAULT 0,
        directories INTEGER NOT NULL DEFAULT 0,
        bytes_scanned INTEGER NOT NULL DEFAULT 0,
        bytes_allocated INTEGER NOT NULL DEFAULT 0,
        reclaimable_bytes INTEGER NOT NULL DEFAULT 0,
        warnings INTEGER NOT NULL DEFAULT 0,
        duration_ms INTEGER NOT NULL DEFAULT 0,
        cancelled INTEGER NOT NULL DEFAULT 0
     );
     CREATE INDEX scans_started ON scans(started_at DESC);
     CREATE TABLE operations (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        ts INTEGER NOT NULL,
        action TEXT NOT NULL,
        path TEXT NOT NULL,
        size INTEGER NOT NULL DEFAULT 0,
        ok INTEGER NOT NULL,
        detail TEXT
     );
     CREATE INDEX operations_ts ON operations(ts DESC);
     CREATE TABLE ignore_list (
        path TEXT PRIMARY KEY,
        added_at INTEGER NOT NULL,
        reason TEXT
     );
     CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
     CREATE TABLE metrics (key TEXT PRIMARY KEY, value INTEGER NOT NULL);",
];

pub struct Db {
    conn: Connection,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanRecord {
    pub id: String,
    pub module: String,
    pub root: String,
    pub started_at: i64,
    pub finished_at: Option<i64>,
    pub files: u64,
    pub directories: u64,
    pub bytes_scanned: u64,
    pub bytes_allocated: u64,
    pub reclaimable_bytes: u64,
    pub warnings: u64,
    pub duration_ms: u64,
    pub cancelled: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationRecord {
    pub id: i64,
    pub ts: i64,
    pub action: String,
    pub path: String,
    pub size: u64,
    pub ok: bool,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IgnoreEntry {
    pub path: String,
    pub added_at: i64,
    pub reason: Option<String>,
}

pub fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

impl Db {
    pub fn open(path: &Path) -> AppResult<Self> {
        let conn = Connection::open(path)?;
        Self::init(conn)
    }

    pub fn open_in_memory() -> AppResult<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(mut conn: Connection) -> AppResult<Self> {
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        let version: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
        for (i, sql) in MIGRATIONS.iter().enumerate().skip(version as usize) {
            let tx = conn.transaction()?;
            tx.execute_batch(sql)?;
            tx.pragma_update(None, "user_version", (i + 1) as i64)?;
            tx.commit()?;
        }
        Ok(Db { conn })
    }

    pub fn insert_scan(&self, r: &ScanRecord) -> AppResult<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO scans (id, module, root, started_at, finished_at, files, directories,
                bytes_scanned, bytes_allocated, reclaimable_bytes, warnings, duration_ms, cancelled)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                r.id, r.module, r.root, r.started_at, r.finished_at, r.files as i64, r.directories as i64,
                r.bytes_scanned as i64, r.bytes_allocated as i64, r.reclaimable_bytes as i64,
                r.warnings as i64, r.duration_ms as i64, r.cancelled
            ],
        )?;
        self.bump_metric("scans_total", 1)?;
        self.bump_metric("files_scanned_total", r.files as i64)?;
        self.bump_metric("bytes_scanned_total", r.bytes_scanned as i64)?;
        Ok(())
    }

    pub fn recent_scans(&self, module: Option<&str>, limit: u32) -> AppResult<Vec<ScanRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, module, root, started_at, finished_at, files, directories, bytes_scanned,
                    bytes_allocated, reclaimable_bytes, warnings, duration_ms, cancelled
             FROM scans WHERE (?1 IS NULL OR module = ?1) ORDER BY started_at DESC LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![module, limit], |r| {
            Ok(ScanRecord {
                id: r.get(0)?,
                module: r.get(1)?,
                root: r.get(2)?,
                started_at: r.get(3)?,
                finished_at: r.get(4)?,
                files: r.get::<_, i64>(5)? as u64,
                directories: r.get::<_, i64>(6)? as u64,
                bytes_scanned: r.get::<_, i64>(7)? as u64,
                bytes_allocated: r.get::<_, i64>(8)? as u64,
                reclaimable_bytes: r.get::<_, i64>(9)? as u64,
                warnings: r.get::<_, i64>(10)? as u64,
                duration_ms: r.get::<_, i64>(11)? as u64,
                cancelled: r.get(12)?,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn log_operation(&self, action: &str, path: &str, size: u64, ok: bool, detail: Option<&str>) -> AppResult<()> {
        self.conn.execute(
            "INSERT INTO operations (ts, action, path, size, ok, detail) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![now_ms(), action, path, size as i64, ok, detail],
        )?;
        if ok {
            self.bump_metric("bytes_reclaimed_total", size as i64)?;
            self.bump_metric("items_removed_total", 1)?;
        } else {
            self.bump_metric("operation_errors_total", 1)?;
        }
        Ok(())
    }

    pub fn operations(&self, limit: u32) -> AppResult<Vec<OperationRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, ts, action, path, size, ok, detail FROM operations ORDER BY id DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit], |r| {
            Ok(OperationRecord {
                id: r.get(0)?,
                ts: r.get(1)?,
                action: r.get(2)?,
                path: r.get(3)?,
                size: r.get::<_, i64>(4)? as u64,
                ok: r.get(5)?,
                detail: r.get(6)?,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn ignore_list(&self) -> AppResult<Vec<IgnoreEntry>> {
        let mut stmt = self.conn.prepare("SELECT path, added_at, reason FROM ignore_list ORDER BY path")?;
        let rows = stmt.query_map([], |r| Ok(IgnoreEntry { path: r.get(0)?, added_at: r.get(1)?, reason: r.get(2)? }))?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn add_ignore(&self, path: &str, reason: Option<&str>) -> AppResult<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO ignore_list (path, added_at, reason) VALUES (?1, ?2, ?3)",
            params![path, now_ms(), reason],
        )?;
        Ok(())
    }

    pub fn remove_ignore(&self, path: &str) -> AppResult<()> {
        self.conn.execute("DELETE FROM ignore_list WHERE path = ?1", params![path])?;
        Ok(())
    }

    pub fn setting(&self, key: &str) -> AppResult<Option<String>> {
        Ok(self.conn.query_row("SELECT value FROM settings WHERE key = ?1", params![key], |r| r.get(0)).optional()?)
    }

    pub fn set_setting(&self, key: &str, value: &str) -> AppResult<()> {
        self.conn.execute("INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)", params![key, value])?;
        Ok(())
    }

    pub fn bump_metric(&self, key: &str, by: i64) -> AppResult<()> {
        self.conn.execute(
            "INSERT INTO metrics (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = value + ?2",
            params![key, by],
        )?;
        Ok(())
    }

    pub fn metrics(&self) -> AppResult<Vec<(String, i64)>> {
        let mut stmt = self.conn.prepare("SELECT key, value FROM metrics ORDER BY key")?;
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
        Ok(rows.collect::<Result<_, _>>()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_and_roundtrip() {
        let db = Db::open_in_memory().unwrap();
        db.insert_scan(&ScanRecord {
            id: "a".into(), module: "scanner".into(), root: "/tmp".into(), started_at: 1, finished_at: Some(2),
            files: 10, directories: 2, bytes_scanned: 100, bytes_allocated: 120, reclaimable_bytes: 0,
            warnings: 0, duration_ms: 5, cancelled: false,
        }).unwrap();
        assert_eq!(db.recent_scans(Some("scanner"), 10).unwrap().len(), 1);
        assert!(db.recent_scans(Some("other"), 10).unwrap().is_empty());
        db.log_operation("trash", "/x", 50, true, None).unwrap();
        db.log_operation("trash", "/y", 10, false, Some("denied")).unwrap();
        assert_eq!(db.operations(10).unwrap().len(), 2);
        let metrics: std::collections::HashMap<_, _> = db.metrics().unwrap().into_iter().collect();
        assert_eq!(metrics["bytes_reclaimed_total"], 50);
        assert_eq!(metrics["files_scanned_total"], 10);
        db.add_ignore("/x", Some("keep")).unwrap();
        assert_eq!(db.ignore_list().unwrap()[0].path, "/x");
        db.remove_ignore("/x").unwrap();
        assert!(db.ignore_list().unwrap().is_empty());
        db.set_setting("k", "v").unwrap();
        assert_eq!(db.setting("k").unwrap().as_deref(), Some("v"));
    }
}
