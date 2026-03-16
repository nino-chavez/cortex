use rusqlite::{Connection, Result, params};
use serde::Serialize;
use std::path::Path;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize)]
pub struct CaptureRow {
    pub id: i64,
    pub timestamp: String,
    pub app_name: String,
    pub bundle_id: String,
    pub window_title: String,
    pub display_id: u32,
    pub image_path: String,
    pub image_hash: String,
    pub is_private: bool,
}

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        let conn = Connection::open(path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS captures (
                id INTEGER PRIMARY KEY,
                timestamp TEXT NOT NULL,
                app_name TEXT NOT NULL DEFAULT '',
                bundle_id TEXT NOT NULL DEFAULT '',
                window_title TEXT NOT NULL DEFAULT '',
                display_id INTEGER NOT NULL DEFAULT 0,
                image_path TEXT NOT NULL,
                image_hash TEXT NOT NULL DEFAULT '',
                is_private INTEGER NOT NULL DEFAULT 0
            );
            CREATE INDEX IF NOT EXISTS idx_captures_timestamp ON captures(timestamp);
            CREATE INDEX IF NOT EXISTS idx_captures_app ON captures(app_name);",
        )?;

        Ok(Database {
            conn: Mutex::new(conn),
        })
    }

    pub fn insert_capture(
        &self,
        timestamp: &str,
        app_name: &str,
        bundle_id: &str,
        window_title: &str,
        display_id: u32,
        image_path: &str,
        image_hash: &str,
    ) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO captures (timestamp, app_name, bundle_id, window_title, display_id, image_path, image_hash, is_private)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0)",
            params![timestamp, app_name, bundle_id, window_title, display_id, image_path, image_hash],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn get_last_hash_for_display(&self, display_id: u32) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT image_hash FROM captures WHERE display_id = ?1 ORDER BY timestamp DESC LIMIT 1",
        )?;
        let hash = stmt
            .query_row(params![display_id], |row| row.get::<_, String>(0))
            .ok();
        Ok(hash)
    }

    pub fn get_recent_captures(&self, limit: i64) -> Result<Vec<CaptureRow>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, timestamp, app_name, bundle_id, window_title, display_id, image_path, image_hash, is_private
             FROM captures ORDER BY timestamp DESC LIMIT ?1",
        )?;
        let rows = stmt
            .query_map(params![limit], |row| {
                Ok(CaptureRow {
                    id: row.get(0)?,
                    timestamp: row.get(1)?,
                    app_name: row.get(2)?,
                    bundle_id: row.get(3)?,
                    window_title: row.get(4)?,
                    display_id: row.get(5)?,
                    image_path: row.get(6)?,
                    image_hash: row.get(7)?,
                    is_private: row.get::<_, i32>(8)? != 0,
                })
            })?
            .collect::<Result<Vec<_>>>()?;
        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_db() -> (Database, PathBuf) {
        let dir = std::env::temp_dir().join(format!(
            "cortex_test_{}_{}", std::process::id(), std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.db");
        let db = Database::open(&path).unwrap();
        (db, dir)
    }

    #[test]
    fn create_and_insert_roundtrip() {
        let (db, path) = temp_db();

        let id = db
            .insert_capture(
                "2026-03-16T10:00:00Z",
                "Cursor",
                "com.todesktop.230313mzl4w4u92",
                "lib.rs — cortex",
                1,
                "/tmp/test.webp",
                "abc123",
            )
            .unwrap();
        assert!(id > 0);

        let rows = db.get_recent_captures(10).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].app_name, "Cursor");
        assert_eq!(rows[0].bundle_id, "com.todesktop.230313mzl4w4u92");
        assert_eq!(rows[0].image_hash, "abc123");

        std::fs::remove_dir_all(&path).ok();
    }

    #[test]
    fn change_detection_hash_lookup() {
        let (db, path) = temp_db();

        db.insert_capture("2026-03-16T10:00:00Z", "App", "com.app", "Title", 1, "/a.webp", "hash_a").unwrap();
        db.insert_capture("2026-03-16T10:00:05Z", "App", "com.app", "Title", 1, "/b.webp", "hash_b").unwrap();
        db.insert_capture("2026-03-16T10:00:10Z", "App", "com.app", "Title", 2, "/c.webp", "hash_c").unwrap();

        let hash1 = db.get_last_hash_for_display(1).unwrap();
        assert_eq!(hash1, Some("hash_b".to_string()));

        let hash2 = db.get_last_hash_for_display(2).unwrap();
        assert_eq!(hash2, Some("hash_c".to_string()));

        let hash3 = db.get_last_hash_for_display(99).unwrap();
        assert_eq!(hash3, None);

        std::fs::remove_dir_all(&path).ok();
    }
}
