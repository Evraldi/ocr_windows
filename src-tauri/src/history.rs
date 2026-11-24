use base64::Engine;
use rusqlite::{params, Connection};

use crate::models::HistoryEntry;

/// Batas jumlah entri yang disimpan. Preview PNG bisa besar, jadi history
/// tidak boleh tumbuh tanpa batas di disk pengguna.
pub const MAX_ENTRIES: i64 = 500;

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
        self.conn
            .execute(
                "DELETE FROM history WHERE id NOT IN (SELECT id FROM history ORDER BY id DESC LIMIT ?1)",
                params![MAX_ENTRIES],
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

    fn temp_history() -> (History, std::path::PathBuf) {
        let suffix = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "ocr-scan-history-test-{}-{suffix}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        (History::new(&dir).expect("history opens"), dir)
    }

    #[test]
    fn insert_and_list_roundtrip() {
        let (history, dir) = temp_history();
        let id = history
            .insert(100, "code", "ABC-123", "CODE_128", Some(&[1, 2, 3]))
            .expect("insert");
        let rows = history.list(10).expect("list");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, id);
        assert_eq!(rows[0].content, "ABC-123");
        assert!(rows[0].preview.is_some());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn insert_prunes_old_entries() {
        let (history, dir) = temp_history();
        for i in 0..MAX_ENTRIES + 25 {
            history
                .insert(i, "code", &format!("CODE-{i}"), "QR_CODE", None)
                .expect("insert");
        }
        let rows = history.list(1000).expect("list");
        assert_eq!(rows.len() as i64, MAX_ENTRIES);
        assert_eq!(rows[0].content, format!("CODE-{}", MAX_ENTRIES + 24));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn delete_and_clear() {
        let (history, dir) = temp_history();
        let id = history
            .insert(1, "text", "hello", "text", None)
            .expect("insert");
        history.delete(id).expect("delete");
        assert!(history.list(10).expect("list").is_empty());
        history
            .insert(2, "text", "again", "text", None)
            .expect("insert");
        history.clear().expect("clear");
        assert!(history.list(10).expect("list").is_empty());
        let _ = std::fs::remove_dir_all(dir);
    }
}
