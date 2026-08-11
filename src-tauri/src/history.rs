use base64::Engine;
use rusqlite::{params, Connection};

use crate::models::HistoryEntry;

pub struct History {
    conn: Connection,
}

impl History {
    pub fn new(dir: &std::path::Path) -> Result<Self, String> {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        let db_path = dir.join("history.db");
        let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                created_at INTEGER NOT NULL,
                kind TEXT NOT NULL,
                content TEXT NOT NULL,
                format TEXT NOT NULL DEFAULT '',
                preview BLOB
            );",
        )
        .map_err(|e| e.to_string())?;
        Ok(Self { conn })
    }

    pub fn insert(
        &self,
        created_at: i64,
        kind: &str,
        content: &str,
        format: &str,
        preview: Option<&[u8]>,
    ) -> Result<i64, String> {
        self.conn
            .execute(
                "INSERT INTO history (created_at, kind, content, format, preview) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![created_at, kind, content, format, preview],
            )
            .map_err(|e| e.to_string())?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn list(&self, limit: usize) -> Result<Vec<HistoryEntry>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, created_at, kind, content, format, preview FROM history ORDER BY id DESC LIMIT ?1",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![limit as i64], |row| {
                let preview: Option<Vec<u8>> = row.get(5)?;
                Ok(HistoryEntry {
                    id: row.get(0)?,
                    created_at: row.get(1)?,
                    kind: row.get(2)?,
                    content: row.get(3)?,
                    format: row.get(4)?,
                    preview: preview.map(|p| base64::engine::general_purpose::STANDARD.encode(p)),
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn clear(&self) -> Result<(), String> {
        self.conn
            .execute_batch("DELETE FROM history;")
            .map_err(|e| e.to_string())
    }

    pub fn delete(&self, id: i64) -> Result<(), String> {
        self.conn
            .execute("DELETE FROM history WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}
