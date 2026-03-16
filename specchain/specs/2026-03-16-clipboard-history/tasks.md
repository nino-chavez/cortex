# Task Breakdown: Clipboard History

## Overview
Total Tasks: 2 groups, 14 subtasks
Strategy: squad
Depth: standard

## Task List

### Proof of Life -- Backend

#### Task Group 1: Clipboard Watcher, Storage, FTS5, Search Integration
**Dependencies:** Capture Daemon (complete -- background thread pattern, SharedCaptureState for privacy mode check), Search UI (complete -- unified search UNION pattern exists)

This group delivers the full backend pipeline: clipboard polling, storage, FTS5 indexing, and unified search integration. No UI yet -- testable via Tauri commands and search queries.

- [ ] 1.0 Complete clipboard monitoring with storage, indexing, and search integration
  - [ ] 1.1 Schema migration v6 in `storage.rs` -- Bump `CURRENT_SCHEMA_VERSION` to 6. In `if version < 6` block:
    - `CREATE TABLE clipboard_entries (id INTEGER PRIMARY KEY, timestamp TEXT NOT NULL, content_type TEXT NOT NULL DEFAULT 'text', text_content TEXT, image_path TEXT, source_app TEXT NOT NULL DEFAULT '');`
    - `CREATE INDEX idx_clipboard_timestamp ON clipboard_entries(timestamp);`
    - `CREATE INDEX idx_clipboard_source ON clipboard_entries(source_app);`
    - `CREATE VIRTUAL TABLE clipboard_fts USING fts5(entry_id, text_content, tokenize='unicode61');`
  - [ ] 1.2 Add clipboard storage methods to `Database` in `storage.rs`:
    - `insert_clipboard_entry(timestamp, content_type, text_content, source_app) -> Result<i64>` -- inserts into `clipboard_entries` and `clipboard_fts`.
    - `get_recent_clipboard(limit) -> Result<Vec<ClipboardEntry>>`.
    - `search_clipboard(query) -> Result<Vec<ClipboardEntry>>` -- FTS5 search.
    - `clear_clipboard_history() -> Result<()>`.
    - Define `ClipboardEntry` struct: id, timestamp, content_type, text_content, source_app.
  - [ ] 1.3 Add Swift bridge for NSPasteboard -- Create a Swift file (alongside existing Vision bridge) with:
    - `get_clipboard_change_count() -> Int` -- returns `NSPasteboard.general.changeCount`.
    - `get_clipboard_text() -> SRString?` -- returns string content from pasteboard.
    - `get_clipboard_url() -> SRString?` -- returns URL string if pasteboard contains a URL type.
    - Declare extern functions in Rust via `swift-rs`.
  - [ ] 1.4 Create `src-tauri/src/clipboard.rs`:
    - `const PASSWORD_MANAGER_BUNDLES: &[&str]` -- list of known password manager bundle IDs to exclude.
    - `pub fn start_clipboard_watcher(db: Arc<Database>, capture_state: SharedCaptureState, stop_flag: Arc<AtomicBool>)` -- Background thread:
      - Track `last_change_count: i64`.
      - Every 1 second: call `get_clipboard_change_count()`. If unchanged, continue.
      - Check `capture_state` -- if paused, skip.
      - Get focused app via `accessibility::get_focused_app()`. If bundle_id matches password manager list, skip.
      - Read text/URL content. Classify content_type.
      - Call `db.insert_clipboard_entry(...)`.
  - [ ] 1.5 Extend `search_captures` in `search.rs` -- Add a UNION ALL branch:
    ```sql
    SELECT ce.id, ce.timestamp, ce.source_app,
           snippet(clipboard_fts, 1, '<b>', '</b>', '...', 32) as snippet,
           '' as image_path, 'clipboard' as result_type
    FROM clipboard_fts cfts
    JOIN clipboard_entries ce ON ce.id = cfts.entry_id
    WHERE clipboard_fts MATCH ?1
    ```
    Apply time filters on `ce.timestamp`. No app_filter for clipboard (source_app is informational).
  - [ ] 1.6 Register Tauri commands:
    - `#[tauri::command] get_clipboard_history(limit: i64)` -- returns Vec<ClipboardEntry>.
    - `#[tauri::command] search_clipboard(query: String)` -- returns Vec<ClipboardEntry>.
    - `#[tauri::command] clear_clipboard_history()` -- deletes all entries.
    - Add `mod clipboard;` in `lib.rs`. Start clipboard watcher in app setup alongside capture loop.
  - [ ] 1.7 Write 5 tests:
    - (a) Migration v6 creates clipboard_entries table and clipboard_fts.
    - (b) `insert_clipboard_entry` + `get_recent_clipboard` round-trip.
    - (c) `search_clipboard("API key")` returns matching entry.
    - (d) Unified `search_captures("API key")` returns clipboard results with `result_type: "clipboard"`.
    - (e) Password manager bundle ID is correctly excluded (unit test on the exclusion check).

**Acceptance Criteria:**
- Schema v6 migration runs cleanly on existing v5 databases
- Clipboard changes detected and stored within 1 second
- Clipboard entries indexed in FTS5 and searchable
- Unified search returns clipboard results alongside OCR and transcription results
- Password manager entries excluded
- Privacy mode respected
- All 5 tests pass

**Verification Commands:**
```bash
cargo build --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml --lib -- clipboard
cargo test --manifest-path src-tauri/Cargo.toml --lib -- search::tests::clipboard
cargo test --manifest-path src-tauri/Cargo.toml --lib -- storage::tests::migration
```

---

### Clipboard History UI

#### Task Group 2: Clipboard History View
**Dependencies:** Task Group 1 (clipboard storage and commands work)

- [ ] 2.0 Complete clipboard history UI view
  - [ ] 2.1 Create `/clipboard` route -- `src/routes/clipboard/+page.svelte`. Layout: header with title and "Clear History" button, search input, scrollable list of clipboard entries.
  - [ ] 2.2 Create `src/lib/components/ClipboardList.svelte`:
    - Renders a list of `ClipboardEntry` items.
    - Each entry shows: content preview (first 200 chars, or full URL), timestamp (relative: "2 minutes ago"), source app name with icon/badge, content type label (text/url).
    - URL entries render as clickable links.
    - Text entries show with monospace font if they look like code (heuristic: contains `{`, `(`, `;`, or indentation).
  - [ ] 2.3 Add search input -- Text field at the top that calls `search_clipboard(query)` with debouncing (300ms). Updates the list with filtered results.
  - [ ] 2.4 Add "Clear History" confirmation -- "Clear History" button shows a confirmation dialog. On confirm, calls `clear_clipboard_history()` and reloads.
  - [ ] 2.5 Add "Clipboard" to sidebar navigation.
  - [ ] 2.6 Add copy-to-clipboard action -- Each entry has a "Copy" button that copies the text_content back to the clipboard (via Tauri clipboard API or `navigator.clipboard.writeText`).
  - [ ] 2.7 Write 2 tests:
    - (a) Clipboard list renders entries with correct content preview, timestamp, and source app.
    - (b) Search input filters entries and shows matching results.

**Acceptance Criteria:**
- `/clipboard` route shows recent clipboard entries
- Each entry shows content preview, timestamp, and source app
- Search filters entries via FTS5
- "Clear History" button works with confirmation
- "Copy" button copies entry back to clipboard
- Clipboard link in sidebar navigation

**Verification Commands:**
```bash
npm run tauri dev
# Navigate to /clipboard, verify list and search
# Copy text in another app, verify it appears in the list
```

---

## Execution Order

1. **Task Group 1: Backend** -- Must complete first. Delivers clipboard watcher, storage, FTS5, and search integration.
2. **Task Group 2: UI** -- Depends on Group 1 for working storage and commands. Delivers the clipboard history view.
