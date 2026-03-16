use crate::storage::Database;
use rusqlite::{params, Result};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct SearchResult {
    pub capture_id: i64,
    pub timestamp: String,
    pub app_name: String,
    pub snippet: String,
    pub image_path: String,
}

impl Database {
    /// Full-text search across OCR'd captures. Returns results with snippets.
    pub fn search_captures(
        &self,
        query: &str,
        app_filter: Option<&str>,
        time_from: Option<&str>,
        time_to: Option<&str>,
    ) -> Result<Vec<SearchResult>> {
        let conn = self.conn.lock().unwrap();

        let mut sql = String::from(
            "SELECT c.id, c.timestamp, c.app_name,
                    snippet(captures_fts, 1, '<b>', '</b>', '...', 32) as snippet,
                    c.image_path
             FROM captures_fts fts
             JOIN captures c ON c.id = fts.capture_id
             WHERE captures_fts MATCH ?1"
        );

        let mut param_count = 1;
        if app_filter.is_some() {
            param_count += 1;
            sql.push_str(&format!(" AND c.app_name = ?{}", param_count));
        }
        if time_from.is_some() {
            param_count += 1;
            sql.push_str(&format!(" AND c.timestamp >= ?{}", param_count));
        }
        if time_to.is_some() {
            param_count += 1;
            sql.push_str(&format!(" AND c.timestamp <= ?{}", param_count));
        }

        sql.push_str(" ORDER BY c.timestamp DESC LIMIT 50");

        let mut stmt = conn.prepare(&sql)?;

        // Build dynamic params
        let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(query.to_string())];
        if let Some(app) = app_filter {
            params_vec.push(Box::new(app.to_string()));
        }
        if let Some(from) = time_from {
            params_vec.push(Box::new(from.to_string()));
        }
        if let Some(to) = time_to {
            params_vec.push(Box::new(to.to_string()));
        }

        let param_refs: Vec<&dyn rusqlite::types::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();

        let rows = stmt
            .query_map(param_refs.as_slice(), |row: &rusqlite::Row| {
                Ok(SearchResult {
                    capture_id: row.get(0)?,
                    timestamp: row.get(1)?,
                    app_name: row.get(2)?,
                    snippet: row.get(3)?,
                    image_path: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>>>()?;

        Ok(rows)
    }

    /// Insert OCR text into the FTS5 index.
    pub fn insert_fts(&self, capture_id: i64, ocr_text: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO captures_fts (capture_id, ocr_text) VALUES (?1, ?2)",
            params![capture_id, ocr_text],
        )?;
        Ok(())
    }

    /// Update OCR status for a capture.
    pub fn set_ocr_status(&self, capture_id: i64, status: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE captures SET ocr_status = ?1 WHERE id = ?2",
            params![status, capture_id],
        )?;
        Ok(())
    }

    /// Get captures pending OCR, newest first.
    pub fn get_pending_ocr(&self, limit: i64) -> Result<Vec<(i64, String)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, image_path FROM captures WHERE ocr_status = 'pending' ORDER BY timestamp DESC LIMIT ?1",
        )?;
        let rows = stmt
            .query_map(params![limit], |row: &rusqlite::Row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// Get OCR status counts.
    pub fn get_ocr_status_counts(&self) -> Result<OcrStatusCounts> {
        let conn = self.conn.lock().unwrap();
        let pending: i64 = conn.query_row(
            "SELECT COUNT(*) FROM captures WHERE ocr_status = 'pending'", [], |r: &rusqlite::Row| r.get(0)
        )?;
        let completed: i64 = conn.query_row(
            "SELECT COUNT(*) FROM captures WHERE ocr_status = 'completed'", [], |r: &rusqlite::Row| r.get(0)
        )?;
        let failed: i64 = conn.query_row(
            "SELECT COUNT(*) FROM captures WHERE ocr_status = 'failed'", [], |r: &rusqlite::Row| r.get(0)
        )?;
        Ok(OcrStatusCounts { pending, completed, failed })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct OcrStatusCounts {
    pub pending: i64,
    pub completed: i64,
    pub failed: i64,
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
    fn fts5_insert_and_search_roundtrip() {
        let (db, dir) = temp_db();

        let id = db.insert_capture(
            "2026-03-16T10:00:00Z", "Cursor", "com.cursor", "main.rs", 1, "/a.webp", "h1"
        ).unwrap();

        db.insert_fts(id, "error E0308 mismatched types expected i32 found String").unwrap();
        db.set_ocr_status(id, "completed").unwrap();

        let results = db.search_captures("E0308", None, None, None).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].capture_id, id);
        assert!(results[0].snippet.contains("E0308"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn search_returns_empty_for_no_match() {
        let (db, dir) = temp_db();

        let id = db.insert_capture(
            "2026-03-16T10:00:00Z", "Chrome", "com.chrome", "Google", 1, "/a.webp", "h1"
        ).unwrap();

        db.insert_fts(id, "hello world this is some text on screen").unwrap();

        let results = db.search_captures("nonexistent_xyz", None, None, None).unwrap();
        assert!(results.is_empty());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn search_with_app_filter() {
        let (db, dir) = temp_db();

        let id1 = db.insert_capture(
            "2026-03-16T10:00:00Z", "Cursor", "com.cursor", "main.rs", 1, "/a.webp", "h1"
        ).unwrap();
        let id2 = db.insert_capture(
            "2026-03-16T10:00:05Z", "Chrome", "com.chrome", "Google", 1, "/b.webp", "h2"
        ).unwrap();

        db.insert_fts(id1, "error code E0308 in Rust compiler").unwrap();
        db.insert_fts(id2, "error code 404 not found").unwrap();

        let all = db.search_captures("error", None, None, None).unwrap();
        assert_eq!(all.len(), 2);

        let cursor_only = db.search_captures("error", Some("Cursor"), None, None).unwrap();
        assert_eq!(cursor_only.len(), 1);
        assert_eq!(cursor_only[0].app_name, "Cursor");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn migration_adds_ocr_status_column() {
        let (db, dir) = temp_db();

        // Insert a capture — should have default ocr_status = 'pending'
        let id = db.insert_capture(
            "2026-03-16T10:00:00Z", "App", "com.app", "Title", 1, "/a.webp", "h1"
        ).unwrap();

        let pending = db.get_pending_ocr(10).unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].0, id);

        let counts = db.get_ocr_status_counts().unwrap();
        assert_eq!(counts.pending, 1);
        assert_eq!(counts.completed, 0);

        std::fs::remove_dir_all(&dir).ok();
    }
}
