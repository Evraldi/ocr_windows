use base64::Engine;
use image::ImageFormat;
use rusqlite::{params, Connection, OptionalExtension};
use std::io::Cursor;

use crate::models::HistoryEntry;

/// Batas jumlah entri yang disimpan. Preview PNG bisa besar, jadi history
/// tidak boleh tumbuh tanpa batas di disk pengguna.
pub const MAX_ENTRIES: i64 = 500;

/// Kecilkan PNG legacy ke thumbnail supaya payload `get_history` tetap ringan.
/// Kalau blob-nya bukan PNG yang bisa dibaca, thumbnail dibuang — lightbox
/// tetap memakai `preview_full` yang asli.
fn thumbnail_png(full: &[u8]) -> Option<Vec<u8>> {
    let img = image::load_from_memory(full).ok()?;
    let scaled = img.thumbnail(
        crate::HISTORY_PREVIEW_MAX_DIM,
        crate::HISTORY_PREVIEW_MAX_DIM,
    );
    let mut buf = Vec::new();
    scaled
        .write_to(&mut Cursor::new(&mut buf), ImageFormat::Png)
        .ok()?;
    Some(buf)
}

/// Row dari DB lama punya `preview` berisi PNG 1600px dan belum punya
/// `preview_full`. Pindahkan full preview ke kolom baru, lalu ganti `preview`
/// dengan thumbnail. Aman dijalankan ulang: hanya row dengan `preview_full`
/// NULL yang diproses.
fn migrate_legacy_previews(conn: &mut Connection) -> Result<(), String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, preview FROM history WHERE preview_full IS NULL AND preview IS NOT NULL",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, Vec<u8>>(1)?))
        })
        .map_err(|e| e.to_string())?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    drop(stmt);
    if rows.is_empty() {
        return Ok(());
    }

    let tx = conn.transaction().map_err(|e| e.to_string())?;
    for (id, full) in rows {
        let thumbnail = thumbnail_png(&full);
        tx.execute(
            "UPDATE history SET preview_full = ?1, preview = ?2 WHERE id = ?3",
            params![full, thumbnail, id],
        )
        .map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())
}

pub struct History {
    conn: Connection,
}

impl History {
    pub fn new(dir: &std::path::Path) -> Result<Self, String> {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        let db_path = dir.join("history.db");
        let mut conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
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

        // DB lama hanya punya `preview` (full-size). Tambah kolom thumbnail
        // supaya `get_history` ringan tanpa mengorbankan lightbox.
        let has_preview_full = conn
            .prepare("SELECT preview_full FROM history LIMIT 0")
            .is_ok();
        if !has_preview_full {
            conn.execute_batch("ALTER TABLE history ADD COLUMN preview_full BLOB;")
                .map_err(|e| e.to_string())?;
        }
        migrate_legacy_previews(&mut conn)?;

        Ok(Self { conn })
    }

    pub fn insert(
        &self,
        created_at: i64,
        kind: &str,
        content: &str,
        format: &str,
        preview: Option<&[u8]>,
        preview_full: Option<&[u8]>,
    ) -> Result<i64, String> {
        self.conn
            .execute(
                "INSERT INTO history (created_at, kind, content, format, preview, preview_full) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![created_at, kind, content, format, preview, preview_full],
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

    /// Ambil preview full-size untuk lightbox. Tidak ikut payload `get_history`.
    pub fn get_preview(&self, id: i64) -> Result<Option<String>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT preview_full FROM history WHERE id = ?1")
            .map_err(|e| e.to_string())?;
        let preview: Option<Option<Vec<u8>>> = stmt
            .query_row(params![id], |row| row.get(0))
            .optional()
            .map_err(|e| e.to_string())?;
        Ok(preview
            .flatten()
            .map(|p| base64::engine::general_purpose::STANDARD.encode(p)))
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
            .insert(
                100,
                "code",
                "ABC-123",
                "CODE_128",
                Some(&[1, 2, 3]),
                Some(&[4, 5, 6]),
            )
            .expect("insert");
        let rows = history.list(10).expect("list");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, id);
        assert_eq!(rows[0].content, "ABC-123");
        assert!(rows[0].preview.is_some());
        assert!(history.get_preview(id).expect("full preview").is_some());
        assert!(history.get_preview(999).expect("missing preview").is_none());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn insert_prunes_old_entries() {
        let (history, dir) = temp_history();
        for i in 0..MAX_ENTRIES + 25 {
            history
                .insert(i, "code", &format!("CODE-{i}"), "QR_CODE", None, None)
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
            .insert(1, "text", "hello", "text", None, None)
            .expect("insert");
        history.delete(id).expect("delete");
        assert!(history.list(10).expect("list").is_empty());
        history
            .insert(2, "text", "again", "text", None, None)
            .expect("insert");
        history.clear().expect("clear");
        assert!(history.list(10).expect("list").is_empty());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn migrates_legacy_database_without_preview_full() {
        use base64::Engine;
        use image::{DynamicImage, ImageFormat};
        use std::io::Cursor;

        let suffix = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "ocr-scan-legacy-history-test-{}-{suffix}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create dir");
        let conn = Connection::open(dir.join("history.db")).expect("open legacy db");
        conn.execute_batch(
            "CREATE TABLE history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                created_at INTEGER NOT NULL,
                kind TEXT NOT NULL,
                content TEXT NOT NULL,
                format TEXT NOT NULL DEFAULT '',
                preview BLOB
            );",
        )
        .expect("create legacy schema");

        // Row legacy: `preview` dulu paling besar 1600px, belum ada
        // `preview_full` sama sekali.
        let full = {
            let img = DynamicImage::new_rgb8(640, 320);
            let mut buf = Vec::new();
            img.write_to(&mut Cursor::new(&mut buf), ImageFormat::Png)
                .expect("png");
            buf
        };
        conn.execute(
            "INSERT INTO history (created_at, kind, content, format, preview)
             VALUES (1, 'text', 'legacy', 'text', ?1)",
            params![full],
        )
        .expect("insert legacy row");
        drop(conn);

        let history = History::new(&dir).expect("migrate");
        let rows = history.list(10).expect("list");
        assert_eq!(rows.len(), 1);
        let legacy_id = rows[0].id;

        // Lightbox harus tetap dapat gambar full-size dari row lama.
        let full_b64 = base64::engine::general_purpose::STANDARD.encode(&full);
        assert_eq!(
            history.get_preview(legacy_id).expect("full preview"),
            Some(full_b64)
        );

        // Payload `get_history` harus berisi thumbnail, bukan PNG 640px.
        let thumb = base64::engine::general_purpose::STANDARD
            .decode(rows[0].preview.as_deref().expect("thumbnail"))
            .expect("decode thumbnail");
        let thumb_img = image::load_from_memory(&thumb).expect("thumbnail PNG");
        assert!(
            thumb_img.width().max(thumb_img.height()) <= crate::HISTORY_PREVIEW_MAX_DIM,
            "legacy thumbnail still full-size: {}x{}",
            thumb_img.width(),
            thumb_img.height()
        );
        assert!(thumb.len() < full.len());

        // Row baru tetap normal setelah migrasi.
        let id = history
            .insert(2, "text", "new", "text", Some(&[1]), Some(&[2]))
            .expect("insert after migration");
        assert!(history.get_preview(id).expect("full preview").is_some());
        let _ = std::fs::remove_dir_all(dir);
    }
}
