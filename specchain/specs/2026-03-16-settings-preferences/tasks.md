# Task Breakdown: Settings & Preferences

## Overview
Total Tasks: 2 groups, 14 subtasks
Strategy: squad
Depth: standard

## Task List

### Proof of Life -- Settings Route, Config Integration, Core Controls

#### Task Group 1: Settings route, config extension, capture interval, excluded apps
**Dependencies:** Retention & Storage Management feature #11 (config.rs exists with load/save and RetentionConfig/StorageConfig), Capture Daemon (capture loop running)

This group delivers the /settings route with General and Privacy sections, config integration, and real-time setting application. Onboarding is deferred to Group 2.

- [ ] 1.0 Complete settings route with config integration and core controls
  - [ ] 1.1 Extend `CortexConfig` in `config.rs` -- Add `GeneralConfig` (capture_interval_secs: u64 = 5, launch_at_login: bool = false), `PrivacyConfig` (excluded_apps: Vec<String> = vec![]), `AudioConfig` (system_audio_enabled: bool = true, microphone_enabled: bool = false). Add `impl Default` for each. Update `CortexConfig` struct to include `general`, `privacy`, `audio` fields alongside existing `retention` and `storage`.
  - [ ] 1.2 Add Tauri commands for settings:
    - `#[tauri::command] get_settings()` -- returns `config::load_config()`.
    - `#[tauri::command] update_settings(section: String, key: String, value: String)` -- load config, match on section/key, parse value, update field, save. Return success/error.
    - `#[tauri::command] get_excluded_apps()` -- returns `load_config().privacy.excluded_apps`.
    - `#[tauri::command] add_excluded_app(bundle_id: String)` -- load, push to excluded_apps if not already present, save.
    - `#[tauri::command] remove_excluded_app(bundle_id: String)` -- load, retain all except matching, save.
    - `#[tauri::command] is_first_run()` -- returns `!config::config_path().exists()`.
    - Register all in `invoke_handler` in `lib.rs`.
  - [ ] 1.3 Add Swift bridge for running apps -- New function `get_running_applications()` via swift-rs:
    - Use `NSWorkspace.shared.runningApplications` to get list of apps.
    - Filter to `activationPolicy == .regular` (visible apps only).
    - Return array of `{name, bundle_id}` pairs.
    - Register `get_running_apps` Tauri command that calls this bridge.
  - [ ] 1.4 Integrate excluded apps into capture loop -- In `capture.rs`, before calling `db.insert_capture()`:
    - Load excluded apps list (cache in CaptureState, reload every 60 seconds or on config change).
    - If current `bundle_id` matches any excluded app, skip capture (log at debug level).
  - [ ] 1.5 Create `/settings` route layout -- `src/routes/settings/+layout.svelte`:
    - Sidebar with section links: General, Privacy, Audio, Storage, About.
    - Content area renders the active section.
    - Add "Settings" link to main app navigation/sidebar.
  - [ ] 1.6 Create General section -- `src/routes/settings/+page.svelte` (or `general/+page.svelte`):
    - On mount, call `get_settings()` to load current config.
    - Capture interval: range input (1-60) with number display. On change, call `update_settings("general", "capture_interval_secs", value)` and `set_capture_interval(value)` (existing command for live update).
    - Launch at login: toggle switch. On change, call `update_settings("general", "launch_at_login", value)`.
  - [ ] 1.7 Create Privacy section -- `src/routes/settings/privacy/+page.svelte`:
    - Display excluded apps list from `get_excluded_apps()`. Each row: app name (if known) + bundle ID + "Remove" button.
    - "Add App" button: popover/modal with two options:
      - "Pick from running apps" -- calls `get_running_apps()`, shows list, click to add.
      - "Enter bundle ID" -- text input with "Add" button.
    - On add/remove, call respective commands and refresh list.
  - [ ] 1.8 Write 4 tests:
    - (a) Extended `CortexConfig` round-trips through save/load with all sections.
    - (b) `add_excluded_app` adds to list, `remove_excluded_app` removes it.
    - (c) `is_first_run` returns true when no config file, false after save.
    - (d) Excluded app filtering: mock capture with excluded bundle_id is skipped.

**Acceptance Criteria:**
- /settings route renders with sidebar navigation
- Capture interval changes take effect immediately in capture loop
- Excluded apps list persists to config.toml
- Excluded app captures are skipped
- Running apps picker shows currently open applications
- All 4 tests pass

**Verification Commands:**
```bash
cargo build --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml --lib -- config
cargo test --manifest-path src-tauri/Cargo.toml --lib -- capture::tests
npm run tauri dev
# Navigate to /settings, change interval, add excluded app, verify behavior
```

---

### Audio Settings, Onboarding, About

#### Task Group 2: Audio section, first-run onboarding flow, about page
**Dependencies:** Task Group 1 (settings route and config integration work), Permissions module (check_all exists)

- [ ] 2.0 Complete audio settings, onboarding flow, and about page
  - [ ] 2.1 Create Audio section -- `src/routes/settings/audio/+page.svelte`:
    - System audio toggle: reads `config.audio.system_audio_enabled`. On change, calls `update_settings("audio", "system_audio_enabled", value)`.
    - Microphone toggle: reads `config.audio.microphone_enabled`. On change, calls `update_settings("audio", "microphone_enabled", value)`.
    - Note text: "Audio capture requires the Microphone permission to be granted in System Settings."
  - [ ] 2.2 Create About section -- `src/routes/settings/about/+page.svelte`:
    - Display app version (read from `tauri::api::app::get_version()` or package.json).
    - Display data directory path (`~/.cortex`). "Open in Finder" button using Tauri shell plugin `open` command.
    - "Reset to Defaults" button: confirmation dialog, then delete config.toml, reload page.
  - [ ] 2.3 Create onboarding component -- `src/lib/components/Onboarding.svelte`:
    - Multi-step modal overlay. Steps: Welcome, Screen Recording, Accessibility, Microphone, Ready.
    - Each permission step: call `check_permissions()` on mount and poll every 2 seconds. Show green checkmark when granted, amber warning when not.
    - "Open System Settings" button for each permission (use Tauri shell open with the relevant `x-apple.systempreferences` URL).
    - "Skip" button on Microphone step.
    - "Start Capturing" on final step: calls `start_capture()` and closes modal.
  - [ ] 2.4 Integrate onboarding into app layout -- In `src/routes/+layout.svelte`:
    - On mount, call `is_first_run()`. If true, show Onboarding modal.
    - After onboarding completes (config.toml created), dismiss modal and proceed to main UI.
    - If permissions are missing but not first run, show a dismissible banner: "Some permissions are missing."
  - [ ] 2.5 Add Storage section link -- `src/routes/settings/storage/+page.svelte`:
    - If feature #11 UI exists, this page is already built. If not, create a placeholder that links to /settings/storage.
    - Ensure sidebar navigation includes "Storage" section.
  - [ ] 2.6 Write 2 tests:
    - (a) Onboarding component renders all 5 steps in sequence.
    - (b) `is_first_run` integration: onboarding shows when config.toml absent, hides when present.

**Acceptance Criteria:**
- Audio toggles update config.toml and reflect current state
- About page shows correct version and data directory
- "Reset to Defaults" deletes config and reloads with defaults
- Onboarding appears on first launch with all permission steps
- Permission status updates in real-time when user grants in System Settings
- Onboarding dismisses after completion and doesn't reappear

**Verification Commands:**
```bash
npm run tauri dev
# Delete ~/.cortex/config.toml, relaunch app
# Verify onboarding appears, walk through steps
# Navigate to /settings/audio, toggle settings
# Navigate to /settings/about, verify version and reset
```

---

## Execution Order

1. **Task Group 1: Settings + Config** -- Settings route, config extension, General and Privacy sections, excluded apps integration.
2. **Task Group 2: Audio + Onboarding** -- Depends on Group 1 for settings infrastructure. Delivers audio controls, onboarding, and about page.
