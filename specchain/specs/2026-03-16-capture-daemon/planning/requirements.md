# Spec Requirements: Capture Daemon

## Initial Description
Capture Daemon — Background Rust service that takes periodic screenshots via ScreenCaptureKit, detects the active app/window title, and stores captures as compressed images with metadata in SQLite. Runs from the system tray with start/stop/pause controls.

## Requirements Discussion

### First Round Questions

**Q1: Proof of Life**
**Answer:** User toggles "Start" in the tray. After 30 seconds of activity, they open a basic debug view in Cortex (or check a local folder) and see a sequence of timestamped images that accurately reflect what was on their screen, tagged with the correct application name (e.g., "Cursor," "Slack," "Chrome").

**Q2: Value Signal**
**Answer:** "Visual Time Travel" — a user thinks "What was that specific error message I saw in the terminal 10 minutes ago?" They open the capture viewer, scrub back, and see the exact pixels. Metric: storage footprint for 8 hours of work is <500MB so the user never feels the need to turn it off.

**Q3: Capture Interval & Change Detection**
**Answer:** 5-second default is confirmed. Yes to pixel hash/diff check for skipping redundant captures. Additionally, trigger an immediate capture on Window Focus Change (alt-tab). If switching from IDE to browser, capture instantly rather than waiting for the next tick.

**Q4: Multi-display**
**Answer:** Primary display + Active window display. If user has three monitors, don't capture all three every tick. Capture the primary monitor AND whichever monitor holds the focused window. If they are the same, just one capture.

**Q5: Image Format & Compression**
**Answer:** WebP at 80% quality. Lossless PNG is overkill. WebP provides best balance for OCR readiness later while keeping efficient, high-performance footprint.

**Q6: SQLite Schema**
**Answer:** Proposed schema is solid. Add two columns:
- `bundle_id`: Store `com.apple.Terminal` — more reliable for logic than just the display name "Terminal"
- `is_private`: Boolean flag (default false) for eventual "Incognito" or "Sensitive App" filtering

**Q7: System Tray Controls**
**Answer:** Keep minimal but functional:
- Start/Pause toggle
- Status indicator (Recording/Paused/Error)
- "Open Cortex" (opens main app/settings)
- "Quit"

**Q8: macOS Permissions**
**Answer:** Request BOTH Screen Recording AND Accessibility on first launch. Accessibility is needed now for window_title detection via UI Elements. Better to ask once during onboarding than nag again later when needing Chrome tab detection.

**Q9: Out of Scope**
**Answer:** Confirmed deferred:
- OCR/Text Extraction
- Vector Embeddings/Search
- Audio/Meeting recording
- Clipboard history
- Advanced UI (timeline scrubbing is its own spec)

### Existing Code to Reference

- **RallyHQ** — reference for SQLite patterns
- **ninochavez.co** — aesthetic reference for tray icons
- This is a greenfield project, no existing code to reuse directly

### Follow-up Questions
None needed — answers were comprehensive.

## Visual Assets

No visual assets provided. Tray icon concept: similar to macOS "Screen Recording" dot but styled with the Signal X "X" logo.

## Requirements Summary

### Functional Requirements
- Background Rust daemon that captures screenshots at configurable intervals (default 5s)
- Uses macOS ScreenCaptureKit for screen capture
- Captures primary display + active window's display (deduplicated if same)
- Pixel hash/diff change detection to skip redundant captures
- Immediate capture triggered on window focus change (not just interval-based)
- WebP compression at 80% quality
- Detects active app name, bundle_id, and window title via Accessibility API
- Stores captures as files on disk with metadata in SQLite
- System tray with Start/Pause toggle, status indicator, Open Cortex, Quit
- Requests Screen Recording + Accessibility permissions on first launch
- Storage target: <500MB for 8 hours of continuous use

### SQLite Schema (captures table)
- `id` — primary key
- `timestamp` — capture time
- `app_name` — display name of active application
- `bundle_id` — macOS bundle identifier (e.g., com.apple.Terminal)
- `window_title` — title of the focused window
- `display_id` — which display was captured
- `image_path` — path to WebP file on disk
- `image_hash` — for deduplication/change detection
- `is_private` — boolean flag for sensitive app filtering (default false)

### Scope Boundaries
**In Scope:**
- Screenshot capture daemon (Rust)
- ScreenCaptureKit integration
- Accessibility API for app/window metadata
- Change detection (pixel diff + focus change)
- Multi-display awareness (primary + focused)
- WebP compression pipeline
- SQLite storage with metadata
- System tray UI (Tauri)
- macOS permission handling (Screen Recording + Accessibility)

**Out of Scope:**
- OCR / text extraction from screenshots
- Vector embeddings / semantic search
- Audio capture / transcription
- Clipboard monitoring
- Timeline UI / search UI / chat UI
- Retention policies / auto-cleanup

### Technical Considerations
- ScreenCaptureKit requires macOS 12.3+
- Accessibility API requires user permission grant
- screencapturekit-rs crate for Rust bindings
- Tauri system tray plugin for tray controls
- Image encoding via webp crate or image crate with WebP support
- rusqlite for SQLite integration
- Window focus change detection via NSWorkspace notifications or Accessibility observers
