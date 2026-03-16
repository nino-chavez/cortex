mod accessibility;
mod audio;
mod capture;
mod chat;
mod clipboard;
mod embedding;
mod meeting;
mod ocr;
mod ocr_worker;
mod permissions;
mod search;
mod storage;
mod summary;
mod tray;

use capture::{CaptureState, SharedCaptureState};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use storage::Database;
use tauri::Manager;

fn cortex_data_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".cortex")
}

#[tauri::command]
fn start_capture(state: tauri::State<SharedCaptureState>, db: tauri::State<Arc<Database>>) {
    capture::request_start(&state);
    capture::start_capture_loop(state.inner().clone(), db.inner().clone());
}

#[tauri::command]
fn pause_capture(state: tauri::State<SharedCaptureState>) {
    capture::request_stop(&state);
}

#[tauri::command]
fn get_capture_status(state: tauri::State<SharedCaptureState>) -> String {
    let state_lock = state.lock().unwrap();
    state_lock.status.to_string()
}

#[tauri::command]
fn get_recent_captures(db: tauri::State<Arc<Database>>) -> Vec<storage::CaptureRow> {
    db.get_recent_captures(20).unwrap_or_default()
}

#[tauri::command]
fn search_captures(
    db: tauri::State<Arc<Database>>,
    query: String,
    app_filter: Option<String>,
    time_from: Option<String>,
    time_to: Option<String>,
) -> Vec<search::SearchResult> {
    db.search_captures(
        &query,
        app_filter.as_deref(),
        time_from.as_deref(),
        time_to.as_deref(),
    )
    .unwrap_or_default()
}

#[tauri::command]
fn get_ocr_status(db: tauri::State<Arc<Database>>) -> search::OcrStatusCounts {
    db.get_ocr_status_counts().unwrap_or(search::OcrStatusCounts {
        pending: 0,
        completed: 0,
        failed: 0,
    })
}

#[tauri::command]
fn get_captures_for_day(db: tauri::State<Arc<Database>>, date: String) -> Vec<storage::CaptureRow> {
    db.get_captures_for_day(&date).unwrap_or_default()
}

#[tauri::command]
fn get_capture_by_id(db: tauri::State<Arc<Database>>, id: i64) -> Option<storage::CaptureRow> {
    db.get_capture_by_id(id).unwrap_or(None)
}

#[tauri::command]
fn get_capture_ocr_text(db: tauri::State<Arc<Database>>, capture_id: i64) -> Option<String> {
    db.get_capture_ocr_text(capture_id).unwrap_or(None)
}

#[tauri::command]
fn get_distinct_apps(db: tauri::State<Arc<Database>>) -> Vec<String> {
    db.get_distinct_apps().unwrap_or_default()
}

#[tauri::command]
fn chat_message(
    db: tauri::State<Arc<Database>>,
    engine: tauri::State<Arc<embedding::EmbeddingEngine>>,
    message: String,
) -> Result<chat::ChatResponse, String> {
    chat::chat_message(&message, &db, &engine)
}

#[tauri::command]
fn start_meeting(
    meeting_state: tauri::State<meeting::SharedMeetingState>,
    capture_state: tauri::State<SharedCaptureState>,
) -> String {
    meeting::start_meeting(&meeting_state, &capture_state)
}

#[tauri::command]
fn end_meeting(
    meeting_state: tauri::State<meeting::SharedMeetingState>,
    capture_state: tauri::State<SharedCaptureState>,
    db: tauri::State<Arc<Database>>,
) -> Result<meeting::MeetingRow, String> {
    meeting::end_meeting(&meeting_state, &capture_state, &db)
}

#[tauri::command]
fn list_meetings(db: tauri::State<Arc<Database>>, limit: i64) -> Vec<meeting::MeetingRow> {
    db.list_meetings(limit).unwrap_or_default()
}

#[tauri::command]
fn check_ollama_status() -> chat::OllamaStatus {
    chat::check_ollama()
}

#[tauri::command]
fn summarize_period(db: tauri::State<Arc<Database>>, from: String, to: String) -> Result<summary::SummaryResponse, String> {
    summary::summarize_period(&db, &from, &to)
}

#[tauri::command]
fn summarize_app(db: tauri::State<Arc<Database>>, app_name: String, date: String) -> Result<summary::SummaryResponse, String> {
    summary::summarize_app(&db, &app_name, &date)
}

#[tauri::command]
fn summarize_topic(
    db: tauri::State<Arc<Database>>,
    engine: tauri::State<Arc<embedding::EmbeddingEngine>>,
    topic: String,
) -> Result<summary::SummaryResponse, String> {
    summary::summarize_topic(&db, &engine, &topic)
}

#[tauri::command]
fn get_clipboard_entries(db: tauri::State<Arc<Database>>, limit: i64) -> Vec<clipboard::ClipboardEntry> {
    db.get_clipboard_entries(limit).unwrap_or_default()
}

#[tauri::command]
fn check_permissions() -> permissions::PermissionStatus {
    permissions::check_all()
}

#[tauri::command]
fn set_capture_interval(state: tauri::State<SharedCaptureState>, seconds: u64) -> bool {
    if !(1..=60).contains(&seconds) {
        return false;
    }
    let mut state_lock = state.lock().unwrap();
    state_lock.interval_secs = seconds;
    true
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let db_path = cortex_data_dir().join("cortex.db");
    let db = Arc::new(Database::open(&db_path).expect("Failed to open database"));
    let capture_state: SharedCaptureState = Arc::new(Mutex::new(CaptureState::new()));
    let meeting_state: meeting::SharedMeetingState = Arc::new(Mutex::new(meeting::MeetingState::new()));
    let embed_engine = Arc::new(
        embedding::EmbeddingEngine::new().expect("Failed to load embedding model"),
    );

    tauri::Builder::default()
        .setup({
            let state = capture_state.clone();
            let db = db.clone();
            move |app| {
                if cfg!(debug_assertions) {
                    app.handle().plugin(
                        tauri_plugin_log::Builder::default()
                            .level(log::LevelFilter::Info)
                            .build(),
                    )?;
                }

                // Check permissions on startup
                let perm_status = permissions::check_all();
                if !perm_status.screen_recording {
                    log::warn!("Screen Recording permission not granted");
                    permissions::request_screen_recording();
                }
                if !perm_status.accessibility {
                    log::warn!("Accessibility permission not granted — window titles will be 'Unknown'");
                }

                // Register global shortcut: Cmd+Shift+Space toggles search window
                app.handle().plugin(
                    tauri_plugin_global_shortcut::Builder::new()
                        .with_handler(|app, _shortcut, event| {
                            if event.state() == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                                if let Some(window) = app.get_webview_window("search") {
                                    let visible = window.is_visible().unwrap_or(false);
                                    if visible {
                                        window.hide().ok();
                                    } else {
                                        window.show().ok();
                                        window.set_focus().ok();
                                    }
                                }
                            }
                        })
                        .build(),
                )?;

                use tauri_plugin_global_shortcut::GlobalShortcutExt;
                app.handle().global_shortcut().register("CommandOrControl+Shift+Space")?;

                // Start clipboard watcher
                let clipboard_stop = Arc::new(AtomicBool::new(false));
                clipboard::start_clipboard_watcher(db.clone(), clipboard_stop);

                // Start OCR background worker
                let ocr_stop = Arc::new(AtomicBool::new(false));
                ocr_worker::start_ocr_worker(db.clone(), ocr_stop);

                tray::setup_tray(app.handle(), state, db)?;

                Ok(())
            }
        })
        .manage(capture_state)
        .manage(db)
        .manage(embed_engine)
        .manage(meeting_state)
        .invoke_handler(tauri::generate_handler![
            start_capture,
            pause_capture,
            get_capture_status,
            get_recent_captures,
            search_captures,
            get_ocr_status,
            get_captures_for_day,
            get_capture_by_id,
            get_capture_ocr_text,
            get_distinct_apps,
            chat_message,
            check_ollama_status,
            start_meeting,
            end_meeting,
            list_meetings,
            summarize_period,
            summarize_app,
            summarize_topic,
            get_clipboard_entries,
            check_permissions,
            set_capture_interval,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
