use crate::capture::{self, CaptureStatus, SharedCaptureState};
use crate::storage::Database;
use log::info;
use std::sync::Arc;
use tauri::{
    menu::{Menu, MenuItem, MenuId},
    tray::TrayIconBuilder,
    AppHandle, Manager,
};

const TOGGLE_ID: &str = "toggle";
const STATUS_ID: &str = "status";

pub fn setup_tray(
    app: &AppHandle,
    state: SharedCaptureState,
    db: Arc<Database>,
) -> Result<(), Box<dyn std::error::Error>> {
    let toggle = MenuItem::with_id(app, TOGGLE_ID, "Start Capture", true, None::<&str>)?;
    let status = MenuItem::with_id(app, STATUS_ID, "Status: Paused", false, None::<&str>)?;
    let open = MenuItem::with_id(app, "open", "Open Cortex", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[&toggle, &status, &open, &quit])?;

    let _tray = TrayIconBuilder::new()
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(move |app, event| {
            match event.id.as_ref() {
                "toggle" => {
                    let is_recording = {
                        let state_lock = state.lock().unwrap();
                        state_lock.status == CaptureStatus::Recording
                    };

                    if is_recording {
                        capture::request_stop(&state);
                        update_menu_item(app, TOGGLE_ID, "Start Capture");
                        update_menu_item(app, STATUS_ID, "Status: Paused");
                        info!("Capture paused");
                    } else {
                        capture::request_start(&state);
                        capture::start_capture_loop(state.clone(), db.clone());
                        update_menu_item(app, TOGGLE_ID, "Pause Capture");
                        update_menu_item(app, STATUS_ID, "Status: Recording");
                        info!("Capture started");
                    }
                }
                "open" => {
                    if let Some(window) = app.get_webview_window("main") {
                        window.show().ok();
                        window.set_focus().ok();
                    }
                }
                "quit" => {
                    capture::request_stop(&state);
                    app.exit(0);
                }
                _ => {}
            }
        })
        .icon(app.default_window_icon().unwrap().clone())
        .build(app)?;

    Ok(())
}

fn update_menu_item(app: &AppHandle, id: &str, text: &str) {
    let menu_id = MenuId::new(id);
    if let Some(item) = app.menu().and_then(|m| m.get(&menu_id)) {
        if let Some(menu_item) = item.as_menuitem() {
            menu_item.set_text(text).ok();
        }
    }
}
