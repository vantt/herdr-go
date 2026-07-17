//! SQLite store — WAL journal, busy-timeout in the connection, embedded
//! migrations applied all-or-nothing in a single transaction (a half-migrated
//! record is never an observable state — airemote migrations rule).

use std::path::Path;
use std::sync::Mutex;

use async_trait::async_trait;
use rusqlite::Connection;

use super::{Notification, Result, Store, StoreError};

fn err<E: std::fmt::Display>(e: E) -> StoreError {
    StoreError::Backend(e.to_string())
}

/// Every migration, embedded in the binary — not operator-managed files.
const MIGRATIONS: &[&str] = &[
    // 0001: kv for the poll offset + the notification outbox.
    r#"
    CREATE TABLE IF NOT EXISTS kv (
        key   TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );
    CREATE TABLE IF NOT EXISTS notifications (
        id           INTEGER PRIMARY KEY AUTOINCREMENT,
        pane_id      TEXT NOT NULL,
        kind         TEXT NOT NULL,
        body         TEXT NOT NULL,
        created_at   INTEGER NOT NULL DEFAULT (strftime('%s','now')),
        delivered_at INTEGER
    );
    "#,
];

pub struct SqliteStore {
    conn: Mutex<Connection>,
}

impl SqliteStore {
    /// Open (creating if needed) a store at `path`, applying migrations.
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path).map_err(err)?;
        Self::init(conn)
    }

    /// Open an in-memory database (tests).
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory().map_err(err)?;
        Self::init(conn)
    }

    fn init(conn: Connection) -> Result<Self> {
        conn.busy_timeout(std::time::Duration::from_secs(5))
            .map_err(err)?;
        conn.pragma_update(None, "journal_mode", "WAL")
            .map_err(err)?;
        conn.pragma_update(None, "synchronous", "NORMAL")
            .map_err(err)?;
        // Apply all pending migrations in ONE transaction — all-or-nothing.
        let applied: i64 = conn
            .query_row(
                "SELECT COALESCE((SELECT value FROM kv WHERE key='schema_version'), '0')",
                [],
                |r| r.get::<_, String>(0),
            )
            .unwrap_or_else(|_| "0".to_string())
            .parse()
            .unwrap_or(0);
        // The kv table may not exist yet for a fresh DB — guard the read above
        // with a table check by attempting migration from `applied`.
        let start = applied.max(0) as usize;
        if start < MIGRATIONS.len() {
            let tx = conn.unchecked_transaction().map_err(err)?;
            for sql in &MIGRATIONS[start..] {
                tx.execute_batch(sql).map_err(err)?;
            }
            tx.execute(
                "INSERT INTO kv(key,value) VALUES('schema_version', ?1)
                 ON CONFLICT(key) DO UPDATE SET value=excluded.value",
                [MIGRATIONS.len().to_string()],
            )
            .map_err(err)?;
            tx.commit().map_err(err)?;
        }
        Ok(SqliteStore {
            conn: Mutex::new(conn),
        })
    }
}

#[async_trait]
impl Store for SqliteStore {
    async fn poll_offset(&self) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        let v: rusqlite::Result<String> =
            conn.query_row("SELECT value FROM kv WHERE key='poll_offset'", [], |r| {
                r.get(0)
            });
        match v {
            Ok(s) => Ok(s.parse().unwrap_or(0)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(0),
            Err(e) => Err(err(e)),
        }
    }

    async fn set_poll_offset(&self, offset: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO kv(key,value) VALUES('poll_offset', ?1)
             ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            [offset.to_string()],
        )
        .map_err(err)?;
        Ok(())
    }

    async fn enqueue_notification(&self, pane_id: &str, kind: &str, body: &str) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO notifications(pane_id,kind,body) VALUES(?1,?2,?3)",
            rusqlite::params![pane_id, kind, body],
        )
        .map_err(err)?;
        Ok(conn.last_insert_rowid())
    }

    async fn undelivered(&self) -> Result<Vec<Notification>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT id,pane_id,kind,body FROM notifications WHERE delivered_at IS NULL ORDER BY id")
            .map_err(err)?;
        let rows = stmt
            .query_map([], |r| {
                Ok(Notification {
                    id: r.get(0)?,
                    pane_id: r.get(1)?,
                    kind: r.get(2)?,
                    body: r.get(3)?,
                })
            })
            .map_err(err)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(err)?);
        }
        Ok(out)
    }

    async fn mark_delivered(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE notifications SET delivered_at=strftime('%s','now') WHERE id=?1",
            [id],
        )
        .map_err(err)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn offset_round_trips() {
        let s = SqliteStore::open_in_memory().unwrap();
        assert_eq!(s.poll_offset().await.unwrap(), 0);
        s.set_poll_offset(42).await.unwrap();
        assert_eq!(s.poll_offset().await.unwrap(), 42);
    }

    #[tokio::test]
    async fn offset_survives_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.sqlite");
        {
            let s = SqliteStore::open(&path).unwrap();
            s.set_poll_offset(99).await.unwrap();
        }
        let s2 = SqliteStore::open(&path).unwrap();
        assert_eq!(s2.poll_offset().await.unwrap(), 99);
    }

    #[tokio::test]
    async fn notification_delivery_is_at_least_once() {
        let s = SqliteStore::open_in_memory().unwrap();
        let id = s
            .enqueue_notification("pane-1", "blocked", "agent needs you")
            .await
            .unwrap();
        let pending = s.undelivered().await.unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, id);
        s.mark_delivered(id).await.unwrap();
        assert!(s.undelivered().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn migration_is_idempotent_on_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("m.sqlite");
        let _ = SqliteStore::open(&path).unwrap();
        // Reopening applies no migration again and stays usable.
        let s2 = SqliteStore::open(&path).unwrap();
        assert_eq!(s2.poll_offset().await.unwrap(), 0);
    }
}
