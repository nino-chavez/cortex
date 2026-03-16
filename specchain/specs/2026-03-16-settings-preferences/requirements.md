# Spec Requirements: Settings & Preferences

## Initial Description
Configuration UI for capture interval, excluded apps, audio sources, hotkey bindings, model selection, retention policies, and storage location. First-run onboarding that requests necessary macOS permissions. Settings route at /settings in the main Cortex window, with config persisted in `~/.cortex/config.toml` (shared with the retention module from feature #11).

## Requirements Discussion

### First Round Questions

**Q1: Proof of Life**
**Answer:** User opens /settings. General section shows capture interval slider (currently 5s). User changes it to 10s -- the capture daemon immediately uses the new interval. User adds "com.1password.1password" to the excluded apps list. 1Password windows are no longer captured. User toggles system audio off in the Audio section. Transcription stops for system audio. Settings persist across app restart.

**Q2: Settings Sections**
**Answer:** Five sections in a sidebar/tab layout:
- **General:** Capture interval (1-60s slider), launch at login toggle.
- **Privacy:** Excluded apps list (add by bundle ID or pick from running apps), private mode hotkey display.
- **Audio:** System audio toggle, microphone toggle, audio source selection (if multiple inputs).
- **Storage:** Retention policies and storage dashboard (delegates to feature #11's /settings/storage).
- **About:** App version, data directory path, links to documentation, "Reset to Defaults" button.

**Q3: Config Integration**
**Answer:** Extends `CortexConfig` from feature #11's `config.rs`:

```toml
[general]
capture_interval_secs = 5
launch_at_login = false

[privacy]
excluded_apps = ["com.1password.1password", "com.bitwarden.desktop"]

[audio]
system_audio_enabled = true
microphone_enabled = false

[retention]
screenshots_days = 30
audio_days = 7
text_days = 0
clipboard_days = 30

[storage]
data_dir = "~/.cortex"
```

All sections share the same `CortexConfig` struct and `config.toml` file.

**Q4: Excluded Apps**
**Answer:** A list of bundle IDs stored in config.toml. The capture daemon checks the focused app's bundle_id against this list before saving a capture. UI shows the list with "Remove" buttons and an "Add App" flow: either type a bundle ID manually, or pick from a list of currently running apps (via `accessibility::get_running_apps()` which uses NSWorkspace).

**Q5: First-Run Onboarding**
**Answer:** On first launch (detected by absence of config.toml), show a modal onboarding flow:
1. Welcome screen explaining what Cortex does.
2. Screen Recording permission -- check `permissions::check_all()`, prompt to grant if missing.
3. Accessibility permission -- same check and prompt.
4. Microphone permission -- optional, explain what it enables.
5. Done screen -- "Start Capturing" button that creates config.toml with defaults and begins capture.

Each step shows the permission status (granted/not granted) and a button to open System Settings to the relevant pane.

**Q6: Out of Scope**
**Answer:** Custom hotkey rebinding (Cmd+Shift+Space is hardcoded for now), model selection/download UI (models are managed separately), theme/appearance settings, notification preferences, account/license management (there are no accounts).

### Existing Code to Reference
- **config.rs** (feature #11) -- CortexConfig struct, load/save functions. Extend with general, privacy, and audio sections.
- **lib.rs** -- `set_capture_interval` command already exists. Permissions check on startup.
- **permissions.rs** -- `check_all()` returns PermissionStatus, `request_screen_recording()`.
- **capture.rs** -- `CaptureState.interval_secs` field, capture loop reads it each cycle.
- **accessibility.rs** -- `get_focused_app()` for excluded app filtering.

## Requirements Summary

### Functional Requirements
- Settings route at /settings with sidebar navigation between sections
- General: capture interval slider (1-60s), launch at login toggle
- Privacy: excluded apps list with add/remove, bundle ID or pick from running apps
- Audio: system audio toggle, microphone toggle
- Storage: link to /settings/storage (feature #11)
- About: version, data dir, reset to defaults
- Config changes persist to config.toml immediately
- Capture daemon respects excluded apps list and interval changes in real-time
- First-run onboarding flow when config.toml doesn't exist
- Onboarding checks and guides screen recording, accessibility, microphone permissions

### Tauri Commands
- `get_settings()` -- Returns full CortexConfig
- `update_settings(section: String, key: String, value: String)` -- Updates a single config value
- `get_excluded_apps()` -- Returns Vec<String> of excluded bundle IDs
- `add_excluded_app(bundle_id: String)` -- Adds to excluded list
- `remove_excluded_app(bundle_id: String)` -- Removes from excluded list
- `get_running_apps()` -- Returns list of running apps with name and bundle_id
- `is_first_run()` -- Returns true if config.toml doesn't exist

### Config Changes (extends feature #11)
- Add `GeneralConfig` section: capture_interval_secs, launch_at_login
- Add `PrivacyConfig` section: excluded_apps (Vec<String>)
- Add `AudioConfig` section: system_audio_enabled, microphone_enabled

### Scope Boundaries
**In Scope:**
- Settings UI with all sections
- Config persistence in shared config.toml
- Excluded apps management
- Capture interval live updates
- First-run onboarding with permission checks
- Audio source toggles
- About page with version info

**Out of Scope:**
- Custom hotkey rebinding
- Model selection/download UI
- Theme/appearance settings
- Notification preferences
- Account/license management
- Advanced audio device selection
