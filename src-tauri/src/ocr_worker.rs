use crate::ocr;
use crate::storage::Database;
use log::{error, info, warn};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

const BATCH_SIZE: i64 = 10;
const POLL_INTERVAL: Duration = Duration::from_secs(3);
const MAX_RETRIES: i32 = 3;

/// Start the background OCR worker on a dedicated thread.
pub fn start_ocr_worker(db: Arc<Database>, stop_flag: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        info!("OCR worker started");

        loop {
            if stop_flag.load(Ordering::Relaxed) {
                info!("OCR worker stopping");
                break;
            }

            let pending = match db.get_pending_ocr(BATCH_SIZE) {
                Ok(p) => p,
                Err(e) => {
                    error!("OCR worker: failed to query pending captures: {}", e);
                    std::thread::sleep(POLL_INTERVAL);
                    continue;
                }
            };

            if pending.is_empty() {
                std::thread::sleep(POLL_INTERVAL);
                continue;
            }

            for (capture_id, image_path) in &pending {
                if stop_flag.load(Ordering::Relaxed) {
                    break;
                }

                process_capture(&db, *capture_id, image_path);
            }
        }

        info!("OCR worker stopped");
    });
}

fn process_capture(db: &Database, capture_id: i64, image_path: &str) {
    // Set status to processing
    if let Err(e) = db.set_ocr_status(capture_id, "processing") {
        error!("OCR worker: failed to set processing status for {}: {}", capture_id, e);
        return;
    }

    // Run OCR
    match ocr::recognize_text_from_file(image_path) {
        Some(text) if !text.trim().is_empty() => {
            // Insert into FTS5
            if let Err(e) = db.insert_fts(capture_id, &text) {
                error!("OCR worker: failed to insert FTS5 for {}: {}", capture_id, e);
                handle_failure(db, capture_id);
                return;
            }

            // Mark completed
            if let Err(e) = db.set_ocr_status(capture_id, "completed") {
                error!("OCR worker: failed to set completed status for {}: {}", capture_id, e);
            } else {
                info!("OCR worker: processed capture {}", capture_id);
            }
        }
        _ => {
            warn!("OCR worker: no text extracted from capture {}", capture_id);
            handle_failure(db, capture_id);
        }
    }
}

fn handle_failure(db: &Database, capture_id: i64) {
    let retries = db.get_ocr_retries(capture_id).unwrap_or(0);
    let new_retries = retries + 1;

    if new_retries >= MAX_RETRIES {
        db.set_ocr_status(capture_id, "failed").ok();
        warn!("OCR worker: capture {} failed after {} retries", capture_id, MAX_RETRIES);
    } else {
        db.increment_ocr_retries(capture_id).ok();
        db.set_ocr_status(capture_id, "pending").ok();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_db() -> (Arc<Database>, PathBuf) {
        let dir = std::env::temp_dir().join(format!(
            "cortex_test_{}_{}", std::process::id(), std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.db");
        let db = Arc::new(Database::open(&path).unwrap());
        (db, dir)
    }

    #[test]
    fn worker_skips_completed_and_failed() {
        let (db, dir) = temp_db();

        db.insert_capture("2026-03-16T10:00:00Z", "A", "com.a", "T", 1, "/a.webp", "h1").unwrap();
        db.insert_capture("2026-03-16T10:00:05Z", "B", "com.b", "T", 1, "/b.webp", "h2").unwrap();
        db.insert_capture("2026-03-16T10:00:10Z", "C", "com.c", "T", 1, "/c.webp", "h3").unwrap();

        // Mark first as completed, second as failed
        db.set_ocr_status(1, "completed").unwrap();
        db.set_ocr_status(2, "failed").unwrap();

        // Only the third should be pending
        let pending = db.get_pending_ocr(10).unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].0, 3);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn retry_counter_tracks_failures() {
        let (db, dir) = temp_db();

        let id = db.insert_capture("2026-03-16T10:00:00Z", "A", "com.a", "T", 1, "/a.webp", "h1").unwrap();

        assert_eq!(db.get_ocr_retries(id).unwrap(), 0);

        db.increment_ocr_retries(id).unwrap();
        assert_eq!(db.get_ocr_retries(id).unwrap(), 1);

        db.increment_ocr_retries(id).unwrap();
        db.increment_ocr_retries(id).unwrap();
        assert_eq!(db.get_ocr_retries(id).unwrap(), 3);

        std::fs::remove_dir_all(&dir).ok();
    }
}
