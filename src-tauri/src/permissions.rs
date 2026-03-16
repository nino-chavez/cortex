use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct PermissionStatus {
    pub screen_recording: bool,
    pub accessibility: bool,
}

/// Check if Screen Recording permission is granted.
pub fn check_screen_recording() -> bool {
    // CGPreflightScreenCaptureAccess returns true if permission is granted
    unsafe {
        extern "C" {
            fn CGPreflightScreenCaptureAccess() -> bool;
        }
        CGPreflightScreenCaptureAccess()
    }
}

/// Request Screen Recording permission (shows system dialog if not yet decided).
pub fn request_screen_recording() -> bool {
    unsafe {
        extern "C" {
            fn CGRequestScreenCaptureAccess() -> bool;
        }
        CGRequestScreenCaptureAccess()
    }
}

/// Check if Accessibility permission is granted.
pub fn check_accessibility() -> bool {
    unsafe {
        extern "C" {
            fn AXIsProcessTrusted() -> bool;
        }
        AXIsProcessTrusted()
    }
}

/// Check both permissions at once.
pub fn check_all() -> PermissionStatus {
    PermissionStatus {
        screen_recording: check_screen_recording(),
        accessibility: check_accessibility(),
    }
}

/// Request both permissions. Returns current status after requesting.
pub fn request_all() -> PermissionStatus {
    request_screen_recording();
    // Accessibility doesn't have a "request" API — the system prompts automatically
    // when AXUIElement APIs are first accessed. We can trigger it by checking.
    check_accessibility();
    check_all()
}

/// Open System Settings to the Screen Recording privacy pane.
pub fn open_screen_recording_settings() {
    std::process::Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture")
        .spawn()
        .ok();
}

/// Open System Settings to the Accessibility privacy pane.
pub fn open_accessibility_settings() {
    std::process::Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
        .spawn()
        .ok();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permission_status_serializes() {
        let status = PermissionStatus {
            screen_recording: true,
            accessibility: false,
        };
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("\"screen_recording\":true"));
        assert!(json.contains("\"accessibility\":false"));
    }

    #[test]
    fn check_all_returns_valid_struct() {
        // This test just verifies the function doesn't panic.
        // Actual permission state depends on the test runner's environment.
        let status = check_all();
        // Both fields should be booleans (type system guarantees this,
        // but verify serialization works)
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("screen_recording"));
        assert!(json.contains("accessibility"));
    }
}
