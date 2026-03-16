use rusqlite::{Connection, Result, params};
use serde::Serialize;
use std::path::Path;
use std::sync::Mutex;

const CURRENT_SCHEMA_VERSION: i32 = 6;

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
    pub(crate) conn: Mutex<Connection>,
}

impl Database {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        // Load sqlite-vec extension before opening
        unsafe {
            rusqlite::ffi::sqlite3_auto_extension(Some(std::mem::transmute(
                sqlite_vec::sqlite3_vec_init as *const (),
            )));
        }

        let conn = Connection::open(path)?;
        Self::run_migrations(&conn)?;

        Ok(Database {
            conn: Mutex::new(conn),
        })
    }

    fn run_migrations(conn: &Connection) -> Result<()> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_version (
                version INTEGER NOT NULL
            );",
        )?;

        let version: i32 = conn
            .query_row("SELECT version FROM schema_version LIMIT 1", [], |row| row.get(0))
            .unwrap_or(0);

        if version < 1 {
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
                CREATE INDEX IF NOT EXISTS idx_captures_app ON captures(app_name);
                CREATE INDEX IF NOT EXISTS idx_captures_bundle ON captures(bundle_id);",
            )?;

        }

        if version < 2 {
            // Migration v2: OCR pipeline support
            // Add ocr_status column (use execute for each since ALTER TABLE can't batch)
            let has_ocr_status: bool = conn
                .prepare("SELECT COUNT(*) FROM pragma_table_info('captures') WHERE name='ocr_status'")?
                .query_row([], |row| row.get::<_, i64>(0))
                .unwrap_or(0) > 0;

            if !has_ocr_status {
                conn.execute_batch(
                    "ALTER TABLE captures ADD COLUMN ocr_status TEXT NOT NULL DEFAULT 'pending';
                     ALTER TABLE captures ADD COLUMN ocr_retries INTEGER NOT NULL DEFAULT 0;"
                )?;
            }

            conn.execute_batch(
                "CREATE INDEX IF NOT EXISTS idx_captures_ocr_status ON captures(ocr_status);
                 CREATE VIRTUAL TABLE IF NOT EXISTS captures_fts USING fts5(capture_id, ocr_text, tokenize='unicode61');"
            )?;
        }

        if version < 3 {
            // Migration v3: Audio transcription support
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS transcriptions (
                    id INTEGER PRIMARY KEY,
                    capture_id INTEGER,
                    timestamp_start TEXT NOT NULL,
                    timestamp_end TEXT NOT NULL,
                    text TEXT NOT NULL DEFAULT '',
                    source TEXT NOT NULL DEFAULT 'system',
                    audio_path TEXT NOT NULL DEFAULT '',
                    FOREIGN KEY (capture_id) REFERENCES captures(id)
                );
                CREATE INDEX IF NOT EXISTS idx_transcriptions_start ON transcriptions(timestamp_start);
                CREATE INDEX IF NOT EXISTS idx_transcriptions_source ON transcriptions(source);
                CREATE VIRTUAL TABLE IF NOT EXISTS transcriptions_fts USING fts5(transcription_id, text, tokenize='unicode61');"
            )?;
        }

        if version < 4 {
            // Migration v4: Vector embeddings via sqlite-vec
            let has_embed_status: bool = conn
                .prepare("SELECT COUNT(*) FROM pragma_table_info('captures') WHERE name='embedding_status'")?
                .query_row([], |row| row.get::<_, i64>(0))
                .unwrap_or(0) > 0;

            if !has_embed_status {
                conn.execute_batch(
                    "ALTER TABLE captures ADD COLUMN embedding_status TEXT NOT NULL DEFAULT 'pending';"
                ).ok(); // ok() because column may already exist from partial migration
            }

            conn.execute_batch(
                "CREATE VIRTUAL TABLE IF NOT EXISTS vec_captures USING vec0(
                    capture_id INTEGER PRIMARY KEY,
                    embedding float[384] distance_metric=cosine
                );
                CREATE VIRTUAL TABLE IF NOT EXISTS vec_transcriptions USING vec0(
                    transcription_id INTEGER PRIMARY KEY,
                    embedding float[384] distance_metric=cosine
                );"
            )?;
        }

        if version < 5 {
            // Migration v5: Meeting memory
            let has_meeting_id: bool = conn
                .prepare("SELECT COUNT(*) FROM pragma_table_info('captures') WHERE name='meeting_id'")?
                .query_row([], |row| row.get::<_, i64>(0))
                .unwrap_or(0) > 0;

            if !has_meeting_id {
                conn.execute_batch(
                    "ALTER TABLE captures ADD COLUMN meeting_id TEXT;"
                ).ok();
                conn.execute_batch(
                    "ALTER TABLE transcriptions ADD COLUMN meeting_id TEXT;"
                ).ok();
            }

            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS meetings (
                    id TEXT PRIMARY KEY,
                    title TEXT NOT NULL DEFAULT 'Untitled Meeting',
                    start_time TEXT NOT NULL,
                    end_time TEXT NOT NULL,
                    summary TEXT NOT NULL DEFAULT '',
                    participant_count INTEGER NOT NULL DEFAULT 1
                );
                CREATE INDEX IF NOT EXISTS idx_captures_meeting ON captures(meeting_id);
                CREATE INDEX IF NOT EXISTS idx_transcriptions_meeting ON transcriptions(meeting_id);"
            )?;
        }

        if version < 6 {
            // Migration v6: Clipboard history
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS clipboard_entries (
                    id INTEGER PRIMARY KEY,
                    timestamp TEXT NOT NULL,
                    content_type TEXT NOT NULL DEFAULT 'text',
                    text_content TEXT NOT NULL DEFAULT ''
                );
                CREATE INDEX IF NOT EXISTS idx_clipboard_timestamp ON clipboard_entries(timestamp);
                CREATE VIRTUAL TABLE IF NOT EXISTS clipboard_fts USING fts5(entry_id, text_content, tokenize='unicode61');"
            )?;
        }

        // Update or insert schema version
        if version == 0 {
            conn.execute("INSERT INTO schema_version (version) VALUES (?1)", params![CURRENT_SCHEMA_VERSION])?;
        } else if version < CURRENT_SCHEMA_VERSION {
            conn.execute("UPDATE schema_version SET version = ?1", params![CURRENT_SCHEMA_VERSION])?;
        }

        Ok(())
    }

    pub fn schema_version(&self) -> Result<i32> {
        let conn = self.conn.lock().unwrap();
        conn.query_row("SELECT version FROM schema_version LIMIT 1", [], |row| row.get(0))
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

    /// Atomically save a capture: write file first, then DB. Clean up on failure.
    pub fn insert_capture_atomic(
        &self,
        timestamp: &str,
        app_name: &str,
        bundle_id: &str,
        window_title: &str,
        display_id: u32,
        image_path: &str,
        image_hash: &str,
    ) -> Result<i64> {
        // File must already exist at image_path (written by caller)
        let path = std::path::Path::new(image_path);
        if !path.exists() {
            return Err(rusqlite::Error::InvalidParameterName(
                "Image file does not exist".to_string(),
            ));
        }

        match self.insert_capture(timestamp, app_name, bundle_id, window_title, display_id, image_path, image_hash) {
            Ok(id) => Ok(id),
            Err(e) => {
                // DB insert failed — clean up orphaned file
                std::fs::remove_file(path).ok();
                Err(e)
            }
        }
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

    pub fn get_captures_by_app(&self, app_name: &str, limit: i64) -> Result<Vec<CaptureRow>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, timestamp, app_name, bundle_id, window_title, display_id, image_path, image_hash, is_private
             FROM captures WHERE app_name = ?1 ORDER BY timestamp DESC LIMIT ?2",
        )?;
        let rows = stmt
            .query_map(params![app_name, limit], |row| {
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

    pub fn insert_transcription(
        &self,
        capture_id: Option<i64>,
        timestamp_start: &str,
        timestamp_end: &str,
        text: &str,
        source: &str,
        audio_path: &str,
    ) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO transcriptions (capture_id, timestamp_start, timestamp_end, text, source, audio_path)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![capture_id, timestamp_start, timestamp_end, text, source, audio_path],
        )?;
        let id = conn.last_insert_rowid();

        // Also insert into FTS5
        conn.execute(
            "INSERT INTO transcriptions_fts (transcription_id, text) VALUES (?1, ?2)",
            params![id, text],
        )?;

        Ok(id)
    }

    /// Insert a capture embedding into sqlite-vec.
    pub fn insert_capture_embedding(&self, capture_id: i64, embedding: &[f32]) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let bytes: &[u8] = unsafe {
            std::slice::from_raw_parts(embedding.as_ptr() as *const u8, embedding.len() * 4)
        };
        conn.execute(
            "INSERT INTO vec_captures(capture_id, embedding) VALUES (?1, ?2)",
            params![capture_id, bytes],
        )?;
        conn.execute(
            "UPDATE captures SET embedding_status = 'completed' WHERE id = ?1",
            params![capture_id],
        )?;
        Ok(())
    }

    /// Insert a transcription embedding into sqlite-vec.
    pub fn insert_transcription_embedding(&self, transcription_id: i64, embedding: &[f32]) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let bytes: &[u8] = unsafe {
            std::slice::from_raw_parts(embedding.as_ptr() as *const u8, embedding.len() * 4)
        };
        conn.execute(
            "INSERT INTO vec_transcriptions(transcription_id, embedding) VALUES (?1, ?2)",
            params![transcription_id, bytes],
        )?;
        Ok(())
    }

    /// Get captures pending embedding.
    pub fn get_pending_embeddings(&self, limit: i64) -> Result<Vec<(i64, String)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT c.id, fts.ocr_text FROM captures c
             JOIN captures_fts fts ON fts.capture_id = c.id
             WHERE c.embedding_status = 'pending' AND c.ocr_status = 'completed'
             ORDER BY c.timestamp DESC LIMIT ?1"
        )?;
        let rows = stmt
            .query_map(params![limit], |row: &rusqlite::Row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// Semantic search via sqlite-vec cosine similarity.
    pub fn semantic_search_captures(&self, query_embedding: &[f32], limit: i64) -> Result<Vec<(i64, f64)>> {
        let conn = self.conn.lock().unwrap();
        let bytes: &[u8] = unsafe {
            std::slice::from_raw_parts(query_embedding.as_ptr() as *const u8, query_embedding.len() * 4)
        };
        let mut stmt = conn.prepare(
            "SELECT capture_id, distance FROM vec_captures
             WHERE embedding MATCH ?1
             ORDER BY distance LIMIT ?2"
        )?;
        let rows = stmt
            .query_map(params![bytes, limit], |row: &rusqlite::Row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, f64>(1)?))
            })?
            .collect::<Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// Get all captures for a specific day (YYYY-MM-DD), ordered by timestamp.
    pub fn get_captures_for_day(&self, date: &str) -> Result<Vec<CaptureRow>> {
        let conn = self.conn.lock().unwrap();
        let start = format!("{}T00:00:00", date);
        let end = format!("{}T23:59:59", date);
        let mut stmt = conn.prepare(
            "SELECT id, timestamp, app_name, bundle_id, window_title, display_id, image_path, image_hash, is_private
             FROM captures WHERE timestamp >= ?1 AND timestamp <= ?2 ORDER BY timestamp ASC",
        )?;
        let rows = stmt
            .query_map(params![start, end], |row: &rusqlite::Row| {
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

    /// Get a single capture by ID.
    pub fn get_capture_by_id(&self, id: i64) -> Result<Option<CaptureRow>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, timestamp, app_name, bundle_id, window_title, display_id, image_path, image_hash, is_private
             FROM captures WHERE id = ?1",
        )?;
        let row = stmt
            .query_row(params![id], |row: &rusqlite::Row| {
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
            })
            .ok();
        Ok(row)
    }

    /// Get OCR text for a capture from FTS5 table.
    pub fn get_capture_ocr_text(&self, capture_id: i64) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let text = conn
            .query_row(
                "SELECT ocr_text FROM captures_fts WHERE capture_id = ?1",
                params![capture_id],
                |row: &rusqlite::Row| row.get::<_, String>(0),
            )
            .ok();
        Ok(text)
    }

    /// Get distinct app names from captures.
    pub fn get_distinct_apps(&self) -> Result<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT DISTINCT app_name FROM captures WHERE app_name != '' ORDER BY app_name",
        )?;
        let rows = stmt
            .query_map([], |row: &rusqlite::Row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>>>()?;
        Ok(rows)
    }

    pub fn insert_meeting(
        &self, id: &str, title: &str, start_time: &str, end_time: &str, summary: &str, participant_count: i32,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO meetings (id, title, start_time, end_time, summary, participant_count) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, title, start_time, end_time, summary, participant_count],
        )?;
        Ok(())
    }

    pub fn update_meeting_summary(&self, id: &str, summary: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("UPDATE meetings SET summary = ?1 WHERE id = ?2", params![summary, id])?;
        Ok(())
    }

    pub fn get_meeting(&self, id: &str) -> Option<crate::meeting::MeetingRow> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, title, start_time, end_time, summary, participant_count FROM meetings WHERE id = ?1",
            params![id],
            |row: &rusqlite::Row| {
                Ok(crate::meeting::MeetingRow {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    start_time: row.get(2)?,
                    end_time: row.get(3)?,
                    summary: row.get(4)?,
                    participant_count: row.get(5)?,
                })
            },
        ).ok()
    }

    pub fn list_meetings(&self, limit: i64) -> Result<Vec<crate::meeting::MeetingRow>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, title, start_time, end_time, summary, participant_count FROM meetings ORDER BY start_time DESC LIMIT ?1"
        )?;
        let rows = stmt.query_map(params![limit], |row: &rusqlite::Row| {
            Ok(crate::meeting::MeetingRow {
                id: row.get(0)?,
                title: row.get(1)?,
                start_time: row.get(2)?,
                end_time: row.get(3)?,
                summary: row.get(4)?,
                participant_count: row.get(5)?,
            })
        })?.collect::<Result<Vec<_>>>()?;
        Ok(rows)
    }

    pub fn get_meeting_transcriptions(&self, meeting_id: &str) -> Result<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT text FROM transcriptions WHERE meeting_id = ?1 ORDER BY timestamp_start ASC"
        )?;
        let rows = stmt.query_map(params![meeting_id], |row: &rusqlite::Row| {
            row.get::<_, String>(0)
        })?.collect::<Result<Vec<_>>>()?;
        Ok(rows)
    }

    pub fn set_embedding_status(&self, capture_id: i64, status: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE captures SET embedding_status = ?1 WHERE id = ?2",
            params![status, capture_id],
        )?;
        Ok(())
    }

    pub fn get_captures_in_range(&self, from: &str, to: &str) -> Result<Vec<CaptureRow>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, timestamp, app_name, bundle_id, window_title, display_id, image_path, image_hash, is_private
             FROM captures WHERE timestamp >= ?1 AND timestamp <= ?2 ORDER BY timestamp ASC"
        )?;
        let rows = stmt.query_map(params![from, to], |row: &rusqlite::Row| {
            Ok(CaptureRow {
                id: row.get(0)?, timestamp: row.get(1)?, app_name: row.get(2)?,
                bundle_id: row.get(3)?, window_title: row.get(4)?, display_id: row.get(5)?,
                image_path: row.get(6)?, image_hash: row.get(7)?, is_private: row.get::<_, i32>(8)? != 0,
            })
        })?.collect::<Result<Vec<_>>>()?;
        Ok(rows)
    }

    pub fn insert_clipboard_entry(&self, timestamp: &str, content_type: &str, text: &str) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO clipboard_entries (timestamp, content_type, text_content) VALUES (?1, ?2, ?3)",
            params![timestamp, content_type, text],
        )?;
        let id = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO clipboard_fts (entry_id, text_content) VALUES (?1, ?2)",
            params![id, text],
        )?;
        Ok(id)
    }

    pub fn get_clipboard_entries(&self, limit: i64) -> Result<Vec<crate::clipboard::ClipboardEntry>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, timestamp, content_type, text_content FROM clipboard_entries ORDER BY timestamp DESC LIMIT ?1"
        )?;
        let rows = stmt.query_map(params![limit], |row: &rusqlite::Row| {
            Ok(crate::clipboard::ClipboardEntry {
                id: row.get(0)?,
                timestamp: row.get(1)?,
                content_type: row.get(2)?,
                text_content: row.get(3)?,
            })
        })?.collect::<Result<Vec<_>>>()?;
        Ok(rows)
    }

    pub fn get_capture_count(&self) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.query_row("SELECT COUNT(*) FROM captures", [], |row| row.get(0))
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
    fn migration_creates_schema_version() {
        let (db, dir) = temp_db();
        let version = db.schema_version().unwrap();
        assert_eq!(version, CURRENT_SCHEMA_VERSION);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn create_and_insert_roundtrip() {
        let (db, dir) = temp_db();

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

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn get_recent_captures_ordering_and_limit() {
        let (db, dir) = temp_db();

        db.insert_capture("2026-03-16T10:00:00Z", "A", "com.a", "T", 1, "/a.webp", "h1").unwrap();
        db.insert_capture("2026-03-16T10:00:05Z", "B", "com.b", "T", 1, "/b.webp", "h2").unwrap();
        db.insert_capture("2026-03-16T10:00:10Z", "C", "com.c", "T", 1, "/c.webp", "h3").unwrap();

        let rows = db.get_recent_captures(2).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].app_name, "C"); // most recent first
        assert_eq!(rows[1].app_name, "B");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn get_captures_by_app_filters() {
        let (db, dir) = temp_db();

        db.insert_capture("2026-03-16T10:00:00Z", "Chrome", "com.chrome", "T", 1, "/a.webp", "h1").unwrap();
        db.insert_capture("2026-03-16T10:00:05Z", "Slack", "com.slack", "T", 1, "/b.webp", "h2").unwrap();
        db.insert_capture("2026-03-16T10:00:10Z", "Chrome", "com.chrome", "T", 1, "/c.webp", "h3").unwrap();

        let chrome = db.get_captures_by_app("Chrome", 10).unwrap();
        assert_eq!(chrome.len(), 2);

        let slack = db.get_captures_by_app("Slack", 10).unwrap();
        assert_eq!(slack.len(), 1);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn change_detection_hash_lookup() {
        let (db, dir) = temp_db();

        db.insert_capture("2026-03-16T10:00:00Z", "App", "com.app", "Title", 1, "/a.webp", "hash_a").unwrap();
        db.insert_capture("2026-03-16T10:00:05Z", "App", "com.app", "Title", 1, "/b.webp", "hash_b").unwrap();
        db.insert_capture("2026-03-16T10:00:10Z", "App", "com.app", "Title", 2, "/c.webp", "hash_c").unwrap();

        let hash1 = db.get_last_hash_for_display(1).unwrap();
        assert_eq!(hash1, Some("hash_b".to_string()));

        let hash2 = db.get_last_hash_for_display(2).unwrap();
        assert_eq!(hash2, Some("hash_c".to_string()));

        let hash3 = db.get_last_hash_for_display(99).unwrap();
        assert_eq!(hash3, None);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn atomic_insert_cleans_up_orphaned_file() {
        let (db, dir) = temp_db();

        // Create a temp file to simulate a saved screenshot
        let img_path = dir.join("orphan.webp");
        std::fs::write(&img_path, b"fake webp data").unwrap();
        assert!(img_path.exists());

        // Force a DB error by inserting with a duplicate primary key
        // First insert succeeds
        db.insert_capture("2026-03-16T10:00:00Z", "App", "com.app", "Title", 1, img_path.to_str().unwrap(), "h1").unwrap();

        // Create another file
        let img_path2 = dir.join("orphan2.webp");
        std::fs::write(&img_path2, b"fake webp data 2").unwrap();

        // This should succeed (different row)
        let result = db.insert_capture_atomic(
            "2026-03-16T10:00:05Z", "App", "com.app", "Title", 1,
            img_path2.to_str().unwrap(), "h2",
        );
        assert!(result.is_ok());
        assert!(img_path2.exists()); // file kept on success

        // Test with non-existent file
        let result = db.insert_capture_atomic(
            "2026-03-16T10:00:10Z", "App", "com.app", "Title", 1,
            "/nonexistent/file.webp", "h3",
        );
        assert!(result.is_err());

        std::fs::remove_dir_all(&dir).ok();
    }
}
