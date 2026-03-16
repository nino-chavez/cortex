use serde::Serialize;

#[derive(Debug, Clone, Serialize, Default)]
pub struct FocusedAppInfo {
    pub app_name: String,
    pub bundle_id: String,
    pub window_title: String,
}

pub fn get_focused_app() -> FocusedAppInfo {
    let mut info = FocusedAppInfo::default();

    // Get app name and bundle ID via NSWorkspace (no permission needed)
    {
        use objc2_app_kit::NSWorkspace;

        let workspace = NSWorkspace::sharedWorkspace();
        if let Some(app) = workspace.frontmostApplication() {
            if let Some(name) = app.localizedName() {
                info.app_name = name.to_string();
            }
            if let Some(bundle) = app.bundleIdentifier() {
                info.bundle_id = bundle.to_string();
            }
        }
    }

    // Get window title via active-win-pos-rs (requires Accessibility permission)
    match active_win_pos_rs::get_active_window() {
        Ok(win) => {
            info.window_title = win.title;
        }
        Err(_) => {
            info.window_title = "Unknown".to_string();
        }
    }

    info
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focused_app_info_serializes() {
        let info = FocusedAppInfo {
            app_name: "Cursor".to_string(),
            bundle_id: "com.todesktop.230313mzl4w4u92".to_string(),
            window_title: "lib.rs — cortex".to_string(),
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("Cursor"));
        assert!(json.contains("bundle_id"));
        assert!(json.contains("window_title"));
    }
}
