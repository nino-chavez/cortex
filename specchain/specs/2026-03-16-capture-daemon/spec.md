# Specification: Capture Daemon

## Goal

Build a background Rust daemon that continuously captures screenshots via macOS ScreenCaptureKit, detects the active application and window context, and stores compressed images with metadata in SQLite -- forming the foundational data pipeline for Cortex's "visual time travel" capability.

## Proof of Life

**Scenario:** User clicks "Start" in the system tray, works normally for 30 seconds across 2-3 apps (e.g., Cursor, Chrome, Slack), then opens the `~/.cortex/screenshots/` directory and the SQLite database.
**Validates:** Timestamped WebP screenshots exist on disk, each row in `captures` has the correct `app_name`, `bundle_id`, and `window_title` for the app that was focused at capture time, and duplicate/unchanged frames were skipped.
**Must work before:** OCR pipeline, semantic search, timeline UI, or any feature that consumes captured screenshots.

## User Stories

- As a knowledge worker, I want my screen silently captured in the background so that I can recall exactly what I saw earlier without manual screenshots.
- As a privacy-conscious user, I want capture to run entirely on-device with no network calls so that my screen data never leaves my machine.
- As a user with limited disk space, I want change detection and efficient compression so that 8 hours of capture stays under 500MB.
- As a multi-monitor user, I want the daemon to capture both my primary display and the display with the active window so that context is never missed.
- As a user switching between apps, I want an immediate capture on window focus change so that I never lose the moment of transition.

## Core Requirements

### Functional Requirements

1. **Periodic capture** -- Screenshot at configurable intervals (default 5s, range 1-60s).
2. **Focus-change capture** -- Trigger an immediate capture when the active window changes (e.g., Cmd-Tab), independent of the interval timer.
3. **Multi-display awareness** -- Capture primary display + the display containing the focused window. Deduplicate if they are the same display.
4. **Change detection** -- Compute a pixel hash per capture; skip storage if the hash matches the previous capture for that display.
5. **App context detection** -- Use macOS Accessibility API to resolve `app_name`, `bundle_id`, and `window_title` for the focused window.
6. **Image compression** -- Encode captures as WebP at 80% quality.
7. **SQLite metadata storage** -- Write one row per saved capture to the `captures` table.
8. **System tray UI** -- Tray menu with: Start/Pause toggle, status indicator (Recording / Paused / Error), "Open Cortex" action, Quit action.
9. **Permission handling** -- On first launch, prompt the user to grant both Screen Recording and Accessibility permissions. Display clear guidance if either is denied.

### Non-Functional Requirements

1. **Storage budget** -- Less than 500MB for 8 hours of continuous use at default settings.
2. **CPU/memory overhead** -- Capture loop must not noticeably impact system performance; target <5% CPU average.
3. **Reliability** -- Daemon recovers gracefully from transient ScreenCaptureKit errors without crashing.
4. **Startup** -- Daemon begins capturing within 2 seconds of user clicking "Start."
5. **macOS version** -- Requires macOS 12.3+ (ScreenCaptureKit minimum).

## Visual Design

N/A for this spec. Tray icon concept: a small recording-dot indicator, styled with the Cortex brand. Detailed tray icon design is deferred to a design task.

## Conversion Design

N/A -- this is an internal background service with no user-facing conversion flow.

## Reusable Components

### Existing Code to Leverage

- **Tauri v2 scaffold** -- `src-tauri/src/lib.rs` has the base `tauri::Builder` with logging plugin already configured.
- **Cargo.toml** -- Already includes `tauri`, `serde`, `serde_json`, `log`, and `tauri-plugin-log`.
- This is a greenfield project; no existing capture or SQLite modules to reuse.

### New Components Required

- `capture` module -- ScreenCaptureKit integration, capture loop, change detection.
- `accessibility` module -- macOS Accessibility API wrapper for app/window metadata.
- `storage` module -- SQLite database setup, migrations, insert/query operations via rusqlite.
- `tray` module -- System tray menu setup and event handling via Tauri's tray plugin.
- `permissions` module -- Check and request Screen Recording + Accessibility permissions.

## Technical Approach

- **Database:** SQLite via `rusqlite`, stored at `~/.cortex/cortex.db`. Single `captures` table with columns: `id` (INTEGER PRIMARY KEY), `timestamp` (TEXT ISO-8601), `app_name` (TEXT), `bundle_id` (TEXT), `window_title` (TEXT), `display_id` (INTEGER), `image_path` (TEXT), `image_hash` (TEXT), `is_private` (INTEGER DEFAULT 0). Create table on first run.
- **Tauri Commands (IPC):** Expose commands for the frontend to control the daemon -- `start_capture`, `pause_capture`, `get_capture_status`, `get_recent_captures`. The frontend invokes these via `@tauri-apps/api`. The capture loop itself runs on a background Rust thread, not driven by frontend calls.
- **Frontend:** Minimal for this spec. The system tray is the primary interface. A basic SvelteKit debug/status page may display capture count and recent entries, but the full timeline UI is out of scope.
- **Capture Pipeline:** `screencapturekit-rs` grabs display frames -> hash the pixel buffer -> compare to previous hash -> if changed, encode to WebP via the `webp` or `image` crate -> write file to `~/.cortex/screenshots/YYYY/MM/DD/{timestamp}_{display_id}.webp` -> insert metadata row into SQLite.
- **Focus Change Detection:** Subscribe to `NSWorkspace.didActivateApplicationNotification` (via Objective-C bridge or `cocoa` crate) to fire an immediate capture outside the interval timer.
- **Testing:** Unit tests for change detection (hash comparison), SQLite operations, and image encoding. Integration test: run capture loop for N seconds, assert expected number of files and database rows.

## Out of Scope

- OCR / text extraction from screenshots
- Vector embeddings / semantic search
- Audio capture or transcription
- Clipboard monitoring
- Timeline UI / search UI / chat interface
- Retention policies / auto-cleanup
- Multi-platform support (macOS only)
- Browser URL detection (requires per-browser integration)

## Success Criteria

1. Daemon captures and stores WebP screenshots at the configured interval with correct app metadata.
2. Change detection skips redundant frames (verified by hash comparison in tests).
3. Window focus change triggers an immediate capture within 500ms.
4. Multi-display capture works correctly: primary + focused window display, deduplicated.
5. 8 hours of typical use (mixed app switching, some idle) produces less than 500MB of stored images.
6. System tray provides functional Start/Pause, status indicator, Open Cortex, and Quit controls.
7. Permission prompts appear on first launch and the daemon handles denial gracefully (paused state with user-facing guidance).
