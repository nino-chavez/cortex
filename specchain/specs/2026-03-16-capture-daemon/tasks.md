# Task Breakdown: Capture Daemon

## Overview
Total Tasks: 5 groups, 27 subtasks
Strategy: squad
Depth: standard
Assigned roles: api-engineer (Rust core + IPC), database-engineer (storage), ui-designer (tray + permissions UX), testing-engineer (gap analysis)

## Task List

### Proof of Life — Vertical Slice

#### Task Group 1: End-to-End Capture Loop
**Assigned implementer:** api-engineer
**Dependencies:** None

This group delivers the Proof of Life scenario: user clicks Start in system tray, works for 30 seconds, opens Cortex and sees timestamped screenshots tagged with correct app names. It is a vertical slice across capture, storage, tray, and accessibility.

- [ ] 1.0 Complete minimal end-to-end capture pipeline (capture -> detect app -> compress -> store -> tray control)
  - [ ] 1.1 Create `src-tauri/src/capture.rs` — Initialize ScreenCaptureKit, capture primary display at 5s intervals on a background thread. Use `screencapturekit-rs` crate. Return raw pixel buffers.
  - [ ] 1.2 Create `src-tauri/src/accessibility.rs` — Query macOS Accessibility API for focused window's `app_name`, `bundle_id`, and `window_title`. Wrap in a `FocusedAppInfo` struct. Handle cases where Accessibility is not yet granted (return "Unknown").
  - [ ] 1.3 Create `src-tauri/src/storage.rs` — Initialize SQLite at `~/.cortex/cortex.db` via `rusqlite`. Create `captures` table with columns: `id` (INTEGER PRIMARY KEY), `timestamp` (TEXT ISO-8601), `app_name` (TEXT), `bundle_id` (TEXT), `window_title` (TEXT), `display_id` (INTEGER), `image_path` (TEXT), `image_hash` (TEXT), `is_private` (INTEGER DEFAULT 0). Provide `insert_capture()` function.
  - [ ] 1.4 Implement WebP encoding — Encode captured pixel buffer as WebP at 80% quality using the `image` crate with WebP support. Write to `~/.cortex/screenshots/YYYY/MM/DD/{timestamp}_{display_id}.webp`. Create directory structure as needed.
  - [ ] 1.5 Implement basic change detection — Compute a fast pixel hash (e.g., xxhash or simple CRC) of each frame. Compare to previous hash for the same display. Skip storage if identical.
  - [ ] 1.6 Create `src-tauri/src/tray.rs` — Set up Tauri system tray with menu items: "Start" / "Pause" toggle, status text (Recording / Paused), "Open Cortex", "Quit". Wire Start/Pause to start/stop the capture background thread.
  - [ ] 1.7 Wire everything together in `src-tauri/src/lib.rs` — Register tray, initialize storage on app startup, expose `start_capture` and `pause_capture` as Tauri commands. Ensure capture loop runs on a dedicated thread managed by shared state (`Arc<Mutex<CaptureState>>`).
  - [ ] 1.8 Write 4 tests: (a) SQLite table creation and insert round-trip, (b) change detection skips identical hashes, (c) WebP encoding produces valid file on disk, (d) accessibility info struct serializes correctly.

**Acceptance Criteria:**
- User clicks "Start" in system tray and capture begins within 2 seconds
- After 30 seconds of app switching, `~/.cortex/screenshots/` contains timestamped WebP files
- `~/.cortex/cortex.db` `captures` table has rows with correct `app_name` and `window_title`
- Unchanged frames are skipped (verified by hash comparison)
- User clicks "Pause" and capture stops; "Start" resumes it

**Verification Steps:**
1. Run the app via Tauri dev, click Start, switch between 2-3 apps for 30 seconds — expect 4-6 WebP files in screenshots directory
2. Query SQLite — expect rows with distinct app_name values matching the apps used
3. Leave a static screen for 15 seconds — expect change detection to skip most frames (fewer files than interval would suggest)

**Verification Commands:**
```bash
# Build and run
npx tauri dev

# Check screenshots directory
ls -la ~/.cortex/screenshots/$(date +%Y)/$(date +%m)/$(date +%d)/

# Query database
sqlite3 ~/.cortex/cortex.db "SELECT timestamp, app_name, window_title, image_path FROM captures ORDER BY timestamp DESC LIMIT 10;"

# Run unit tests
cargo test --manifest-path src-tauri/Cargo.toml
```

---

### Storage Layer

#### Task Group 2: Robust Storage & Schema
**Assigned implementer:** database-engineer
**Dependencies:** Task Group 1 (basic storage exists)

- [ ] 2.0 Complete production-quality storage layer with migrations, queries, and file management
  - [ ] 2.1 Add schema versioning — Create a `schema_version` table. On startup, check version and run migrations if needed. Initial version = 1.
  - [ ] 2.2 Add `get_recent_captures(limit: i64)` query — Return the N most recent captures ordered by timestamp descending. Used by the frontend debug view and the `get_recent_captures` Tauri command.
  - [ ] 2.3 Add `get_captures_by_app(app_name: &str)` query — Filter captures by application name.
  - [ ] 2.4 Add `get_last_hash_for_display(display_id: u32)` query — Retrieve the most recent image_hash for a given display, used by change detection to compare across app restarts.
  - [ ] 2.5 Ensure atomic file + DB writes — If WebP write succeeds but DB insert fails, clean up the orphaned file. If DB insert succeeds but file is missing, mark the row or skip it.
  - [ ] 2.6 Write 3 tests: (a) migration creates tables correctly on fresh DB, (b) `get_recent_captures` returns correct order and limit, (c) atomic cleanup removes orphaned files on DB insert failure.

**Acceptance Criteria:**
- Database has schema versioning that supports future migrations
- Query functions return correct results for recent captures and app filtering
- No orphaned files or dangling DB rows on partial failures

**Verification Steps:**
1. Delete `~/.cortex/cortex.db`, start app — expect fresh DB with schema_version = 1
2. Run capture for 10 seconds, query recent captures — expect ordered results
3. Simulate a DB insert failure — expect no orphaned WebP file on disk

**Verification Commands:**
```bash
cargo test --manifest-path src-tauri/Cargo.toml -- storage

sqlite3 ~/.cortex/cortex.db "SELECT * FROM schema_version;"
sqlite3 ~/.cortex/cortex.db "SELECT COUNT(*) FROM captures;"
```

---

### Rust Core — Advanced Capture

#### Task Group 3: Multi-Display, Focus Change, Performance
**Assigned implementer:** api-engineer
**Dependencies:** Task Group 1

- [ ] 3.0 Complete advanced capture features: multi-display, focus-change triggers, and performance optimization
  - [ ] 3.1 Implement multi-display capture — Enumerate displays via ScreenCaptureKit. Capture primary display AND the display containing the focused window. Deduplicate if they are the same display.
  - [ ] 3.2 Implement focus-change capture — Subscribe to `NSWorkspace.didActivateApplicationNotification` via Objective-C bridge (`objc2` or `cocoa` crate). On notification, trigger an immediate capture outside the interval timer.
  - [ ] 3.3 Add configurable capture interval — Store interval in shared state (default 5s, range 1-60s). Expose a `set_capture_interval` Tauri command for future settings UI.
  - [ ] 3.4 Add graceful error recovery — If ScreenCaptureKit returns a transient error, log it and retry on next tick instead of crashing. Track consecutive failures and set tray status to "Error" after 3 consecutive failures.
  - [ ] 3.5 Write 3 tests: (a) multi-display deduplication logic (same display returns 1 capture, different displays return 2), (b) focus-change trigger fires capture, (c) error recovery does not crash after simulated failures.

**Acceptance Criteria:**
- On a multi-monitor setup, both relevant displays are captured per tick
- App switching (Cmd-Tab) triggers an immediate capture within 500ms
- Capture interval is configurable between 1-60 seconds
- Transient ScreenCaptureKit errors are recovered gracefully; tray shows "Error" only after repeated failures

**Verification Steps:**
1. Connect a second display, start capture, switch apps — expect captures from both display IDs in DB
2. Rapidly Cmd-Tab between apps — expect capture timestamps that are closer together than the 5s interval
3. Disconnect display mid-capture — expect error logged but no crash, capture resumes on remaining display

**Verification Commands:**
```bash
cargo test --manifest-path src-tauri/Cargo.toml -- capture

# Check multi-display captures
sqlite3 ~/.cortex/cortex.db "SELECT display_id, COUNT(*) FROM captures GROUP BY display_id;"

# Check focus-change captures (timestamps closer than 5s apart)
sqlite3 ~/.cortex/cortex.db "SELECT timestamp FROM captures ORDER BY timestamp DESC LIMIT 20;"
```

---

### Permissions & Onboarding

#### Task Group 4: macOS Permission Checks and First-Launch Flow
**Assigned implementer:** ui-designer
**Dependencies:** Task Group 1

- [ ] 4.0 Complete permission checking and first-launch onboarding flow
  - [ ] 4.1 Create `src-tauri/src/permissions.rs` — Check Screen Recording permission via `CGPreflightScreenCaptureAccess()` / `CGRequestScreenCaptureAccess()`. Check Accessibility permission via `AXIsProcessTrusted()`. Return a `PermissionStatus` struct with both booleans.
  - [ ] 4.2 Expose `check_permissions` Tauri command — Frontend can call this to get current permission state.
  - [ ] 4.3 Implement first-launch detection — Check if `~/.cortex/cortex.db` exists. If not, this is a first launch. Trigger permission requests and set tray status to "Needs Setup" if either permission is denied.
  - [ ] 4.4 Handle denied permissions gracefully — If Screen Recording is denied, set capture state to Paused with status "Screen Recording permission required". If Accessibility is denied, capture works but `app_name` / `window_title` fields are "Unknown". Show a tray menu item "Grant Permissions..." that opens System Preferences to the relevant pane.
  - [ ] 4.5 Write 2 tests: (a) permission status struct returns correct shape, (b) denied-permission state sets tray status correctly (mock the permission check).

**Acceptance Criteria:**
- On first launch, macOS permission dialogs appear for Screen Recording and Accessibility
- If Screen Recording is denied, daemon pauses with clear status message in tray
- If Accessibility is denied, capture still works but metadata fields show "Unknown"
- Tray includes "Grant Permissions..." menu item that opens System Preferences

**Verification Steps:**
1. Reset permissions in System Preferences, launch app — expect permission prompts
2. Deny Screen Recording — expect tray shows "Screen Recording permission required" and capture is paused
3. Deny Accessibility only — expect captures work but app_name shows "Unknown"
4. Click "Grant Permissions..." — expect System Preferences opens to correct pane

**Verification Commands:**
```bash
cargo test --manifest-path src-tauri/Cargo.toml -- permissions

# Reset screen recording permission for testing (requires app bundle ID)
# tccutil reset ScreenCapture com.cortex.app

npx tauri dev
```

---

### Testing & Integration

#### Task Group 5: Test Review, Gap Analysis, Integration Verification
**Assigned implementer:** testing-engineer
**Dependencies:** Task Groups 1, 2, 3, 4

- [ ] 5.0 Complete test coverage review and fill gaps with up to 10 additional tests
  - [ ] 5.1 Review all tests from Groups 1-4 (12 total). Verify they compile and pass. Document any that are flaky or environment-dependent.
  - [ ] 5.2 Integration test: full capture cycle — Start capture, wait 10 seconds with app switching, stop capture. Assert: (a) WebP files exist on disk, (b) DB rows match file count, (c) app_name is not empty for any row, (d) timestamps are monotonically increasing.
  - [ ] 5.3 Integration test: change detection effectiveness — Display a static screen for 20 seconds at 2s interval. Assert fewer than 10 captures stored (most skipped by hash).
  - [ ] 5.4 Integration test: storage budget estimation — Capture 60 seconds of typical use, measure total bytes stored, extrapolate to 8 hours. Assert projection is under 500MB.
  - [ ] 5.5 Test: DB and filesystem consistency — Insert 5 captures, manually delete 2 WebP files, query recent captures. Assert the system does not crash when referencing missing files.
  - [ ] 5.6 Test: concurrent start/pause commands — Rapidly toggle Start/Pause 10 times. Assert no panics, no thread leaks, final state is deterministic.
  - [ ] 5.7 Test: capture interval boundaries — Set interval to 1s, capture for 5 seconds. Assert 4-6 captures. Set to 60s, capture for 5 seconds. Assert 0-1 captures.
  - [ ] 5.8 Gap analysis — Document any untested paths (e.g., multi-display with >2 monitors, permission revocation mid-capture, disk full scenarios). File as future test TODOs in a comment block.

**Acceptance Criteria:**
- All tests from Groups 1-4 pass
- 6 new integration/stress tests added and passing
- Gap analysis document identifies remaining risk areas
- Total test count: 18+ (12 from Groups 1-4 + 6 new)

**Verification Steps:**
1. Run full test suite — expect all tests pass
2. Review test output for any warnings or flaky indicators
3. Check gap analysis identifies at least 3 untested edge cases

**Verification Commands:**
```bash
# Run all tests
cargo test --manifest-path src-tauri/Cargo.toml

# Run tests with output
cargo test --manifest-path src-tauri/Cargo.toml -- --nocapture

# Check test count
cargo test --manifest-path src-tauri/Cargo.toml -- --list 2>&1 | tail -1

# Build release to verify no compile warnings
cargo build --manifest-path src-tauri/Cargo.toml --release 2>&1 | grep warning
```

---

## Execution Order

1. **Task Group 1: End-to-End Capture Loop** (api-engineer) — Proof of Life vertical slice. Must complete first.
2. **Task Group 2: Robust Storage & Schema** (database-engineer) — Depends on Group 1 for base storage module.
3. **Task Group 3: Multi-Display, Focus Change, Performance** (api-engineer) — Depends on Group 1 for base capture module. Can run in parallel with Group 2.
4. **Task Group 4: macOS Permission Checks and First-Launch Flow** (ui-designer) — Depends on Group 1 for tray and capture state. Can run in parallel with Groups 2 and 3.
5. **Task Group 5: Test Review & Integration** (testing-engineer) — Depends on all prior groups completing.

**Parallel execution possible:** Groups 2, 3, and 4 can run concurrently after Group 1 completes.
