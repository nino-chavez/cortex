use crate::embedding::EmbeddingEngine;
use crate::storage::Database;
use log::{error, info};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

const BATCH_SIZE: i64 = 10;
const POLL_INTERVAL: Duration = Duration::from_secs(5);

/// Background worker that generates embeddings for OCR'd captures.
pub fn start_embedding_worker(
    db: Arc<Database>,
    engine: Arc<EmbeddingEngine>,
    stop_flag: Arc<AtomicBool>,
) {
    std::thread::spawn(move || {
        info!("Embedding worker started");

        loop {
            if stop_flag.load(Ordering::Relaxed) {
                break;
            }

            let pending = match db.get_pending_embeddings(BATCH_SIZE) {
                Ok(p) => p,
                Err(e) => {
                    error!("Embedding worker: failed to query pending: {}", e);
                    std::thread::sleep(POLL_INTERVAL);
                    continue;
                }
            };

            if pending.is_empty() {
                std::thread::sleep(POLL_INTERVAL);
                continue;
            }

            for (capture_id, ocr_text) in &pending {
                if stop_flag.load(Ordering::Relaxed) {
                    break;
                }

                if ocr_text.trim().is_empty() {
                    // Mark as completed with no embedding
                    db.set_embedding_status(*capture_id, "completed").ok();
                    continue;
                }

                match engine.embed_text(ocr_text) {
                    Some(embedding) => {
                        if let Err(e) = db.insert_capture_embedding(*capture_id, &embedding) {
                            error!("Failed to insert embedding for capture {}: {}", capture_id, e);
                        } else {
                            info!("Embedded capture {}", capture_id);
                        }
                    }
                    None => {
                        error!("Failed to embed text for capture {}", capture_id);
                        db.set_embedding_status(*capture_id, "failed").ok();
                    }
                }
            }
        }

        info!("Embedding worker stopped");
    });
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
    fn pending_embeddings_returns_completed_ocr_only() {
        let (db, dir) = temp_db();

        // Insert a capture with completed OCR
        let id1 = db.insert_capture("2026-03-16T10:00:00Z", "App", "com.app", "Title", 1, "/a.webp", "h1").unwrap();
        db.set_ocr_status(id1, "completed").ok();
        db.insert_fts(id1, "some OCR text here").ok();

        // Insert a capture still pending OCR
        let _id2 = db.insert_capture("2026-03-16T10:00:05Z", "App", "com.app", "Title", 1, "/b.webp", "h2").unwrap();

        let pending = db.get_pending_embeddings(10).unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].0, id1);

        std::fs::remove_dir_all(&dir).ok();
    }
}
