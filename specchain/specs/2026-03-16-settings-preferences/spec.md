# Specification: Settings & Preferences

## Goal

Provide a unified settings UI at /settings for configuring capture behavior, privacy exclusions, audio sources, and storage policies. Include a first-run onboarding flow that guides new users through macOS permission grants. All config persisted in `~/.cortex/config.toml`, shared with the retention module.

## Proof of Life

**Scenario:** Fresh install -- user launches Cortex for the first time. Onboarding modal appears: welcome screen, then permission checks for Screen Recording (not granted), Accessibility (not granted), Microphone (not granted). User clicks "Open System Settings" for each, grants permissions, and sees the status update to "Granted." User clicks "Start Capturing." App creates config.toml with defaults and begins capturing. User opens /settings, changes capture interval from 5s to 15s, adds "com.tinyspeck.slackmacgap" to excluded apps. Slack windows stop being captured. Interval change takes effect immediately. User restarts the app -- settings are preserved.

**Validates:** Onboarding flow, permission detection, config persistence, real-time setting application, excluded app filtering.

**Must work before:** Any user-facing release (this is the configuration surface for all other features).

## User Stories

- As a new user, I want to be guided through granting macOS permissions so I don't have to figure it out myself.
- As a user, I want to change how often screenshots are taken without restarting the app.
- As a user, I want to exclude specific apps from being captured for privacy.
- As a user, I want to toggle audio capture sources on and off.
- As a user, I want all my settings in one place and persisted across restarts.

## Core Requirements

### Config Extension

Extend the `CortexConfig` struct from feature #11's `config.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CortexConfig {
    pub general: GeneralConfig,
    pub privacy: PrivacyConfig,
    pub audio: AudioConfig,
    pub retention: RetentionConfig,  // from feature #11
    pub storage: StorageConfig,       // from feature #11
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub capture_interval_secs: u64,  // default 5, range 1-60
    pub launch_at_login: bool,       // default false
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyConfig {
    pub excluded_apps: Vec<String>,  // bundle IDs
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioConfig {
    pub system_audio_enabled: bool,  // default true
    pub microphone_enabled: bool,    // default false
}
```

### Excluded Apps Integration

- Capture loop in `capture.rs`: before saving a capture, check if `bundle_id` is in `config.privacy.excluded_apps`. If yes, skip.
- Load config once per capture cycle (not on every capture -- cache and reload periodically or on config change).
- `get_running_apps()` Tauri command: use `NSWorkspace.shared.runningApplications` via swift-rs bridge to return a list of `{name: String, bundle_id: String}` for currently running apps. Filter to regular apps (not background daemons).

### First-Run Onboarding

- Detect first run: `!config_path().exists()`.
- Frontend: modal/overlay component that blocks interaction until complete.
- Steps (each is a screen):
  1. **Welcome:** "Cortex captures your screen, audio, and clipboard to build a searchable memory." Brief, not overwhelming.
  2. **Screen Recording:** Show `permissions.screen_recording` status. "Open System Settings" button calls `permissions::request_screen_recording()`. Poll status every 2 seconds to update UI when granted.
  3. **Accessibility:** Show `permissions.accessibility` status. "Open System Settings" button. Explain: "Required for detecting active app and window title."
  4. **Microphone (optional):** Show status. Explain: "Optional -- enables meeting transcription and audio capture." Skip button available.
  5. **Ready:** "All set! Cortex will now capture in the background." "Start Capturing" button creates config.toml and begins capture loop.
- If user closes the modal without completing, show a banner on the main page: "Some permissions are missing. Open Settings to fix."

### Settings UI Layout

- Route: `/settings` with sub-routes or tab navigation.
- Sidebar with section links: General, Privacy, Audio, Storage, About.
- Each section is a card or panel with labeled controls.

**General Section:**
- Capture interval: slider or number input (1-60), label shows current value in seconds.
- Launch at login: toggle switch. Uses `tauri-plugin-autostart` or `SMAppService` to register/unregister login item.

**Privacy Section:**
- Excluded apps list: table with app name, bundle ID, and "Remove" button.
- "Add App" button: opens a picker showing running apps (from `get_running_apps()`), or a text field for manual bundle ID entry.

**Audio Section:**
- System audio: toggle (on/off).
- Microphone: toggle (on/off).
- Note: actual audio capture implementation is in feature #3 (audio transcription). These toggles set config values that the audio module reads.

**Storage Section:**
- Embeds or links to /settings/storage from feature #11.

**About Section:**
- App version (from Cargo.toml or tauri.conf.json).
- Data directory path (clickable to open in Finder).
- "Reset to Defaults" button with confirmation dialog. Deletes config.toml and reloads.

### Testing

- Unit test: `CortexConfig` default includes all sections with correct defaults.
- Unit test: excluded app check correctly filters bundle IDs.
- Unit test: `get_running_apps` returns a non-empty list (integration test, macOS only).
- Unit test: `is_first_run` returns true when config.toml doesn't exist, false when it does.

## Out of Scope

- Custom hotkey rebinding
- Model selection or download
- Theme or appearance settings
- Notification preferences
- Account management
- Advanced per-device audio configuration

## Success Criteria

- First-run onboarding appears on fresh install and guides through all permissions.
- Settings changes take effect immediately without restart.
- Excluded apps are not captured.
- Capture interval changes are reflected in the capture loop within one cycle.
- All settings persist across app restarts.
- About page shows correct version and data directory.
