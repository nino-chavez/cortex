mod accessibility;
mod capture;
mod permissions;
mod storage;
mod tray;

use capture::{CaptureState, SharedCaptureState};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use storage::Database;

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

                tray::setup_tray(app.handle(), state, db)?;

                Ok(())
            }
        })
        .manage(capture_state)
        .manage(db)
        .invoke_handler(tauri::generate_handler![
            start_capture,
            pause_capture,
            get_capture_status,
            get_recent_captures,
            check_permissions,
            set_capture_interval,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
