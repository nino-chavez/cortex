use crate::storage::Database;
use chrono::Utc;
use log::{error, info};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Clone, Serialize)]
pub struct ClipboardEntry {
    pub id: i64,
    pub timestamp: String,
    pub content_type: String,
    pub text_content: String,
}

/// Start the clipboard watcher on a background thread.
pub fn start_clipboard_watcher(db: Arc<Database>, stop_flag: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        info!("Clipboard watcher started");
        let mut last_content = String::new();

        loop {
            if stop_flag.load(Ordering::Relaxed) {
                break;
            }

            if let Some(text) = get_clipboard_text() {
                if !text.is_empty() && text != last_content {
                    let content_type = if text.starts_with("http://") || text.starts_with("https://") {
                        "url"
                    } else {
                        "text"
                    };

                    let timestamp = Utc::now().to_rfc3339();
                    if let Err(e) = db.insert_clipboard_entry(&timestamp, content_type, &text) {
                        error!("Failed to save clipboard entry: {}", e);
                    }

                    last_content = text;
                }
            }

            std::thread::sleep(Duration::from_secs(1));
        }

        info!("Clipboard watcher stopped");
    });
}

/// Read current clipboard text content via NSPasteboard (Objective-C bridge).
fn get_clipboard_text() -> Option<String> {
    use objc2_app_kit::NSPasteboard;
    use objc2_foundation::NSString;

    let pasteboard = unsafe { NSPasteboard::generalPasteboard() };
    let string_type = unsafe { NSString::from_str("public.utf8-plain-text") };

    pasteboard
        .stringForType(&string_type)
        .map(|s| s.to_string())
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
    fn clipboard_insert_and_query() {
        let (db, dir) = temp_db();

        db.insert_clipboard_entry("2026-03-16T10:00:00Z", "text", "hello world").unwrap();
        db.insert_clipboard_entry("2026-03-16T10:00:05Z", "url", "https://example.com").unwrap();

        let entries = db.get_clipboard_entries(10).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].content_type, "url"); // newest first
        assert_eq!(entries[1].text_content, "hello world");

        std::fs::remove_dir_all(&dir).ok();
    }
}
