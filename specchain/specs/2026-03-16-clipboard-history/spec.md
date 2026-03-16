# Specification: Clipboard History

## Goal

Capture and index macOS clipboard content (text and URLs) in real-time, store in a FTS5-indexed table, and integrate into unified search. Extends Cortex's capture surface to include everything the user copies, without requiring any new external dependencies or ML infrastructure.

## Proof of Life

**Scenario:** User copies a code snippet in Cursor, copies a URL in Chrome, copies a paragraph from a Notion doc. Each entry appears in the clipboard history view within 1 second with the correct content, timestamp, and source app. User searches "database migration" in unified search and a clipboard entry containing that text appears in results alongside OCR and transcription matches.

**Validates:** Clipboard polling detects changes, content is stored and FTS-indexed, and search integration returns clipboard results in the unified result set.

**Must work before:** Any clipboard management features (paste from history, pinning, etc.).

## User Stories

- As a user, I want everything I copy to be automatically captured and searchable so I never lose clipboard content.
- As a user, I want to see my clipboard history with timestamps and source apps so I can find what I copied and when.
- As a user, I want clipboard content to appear in search results alongside screen captures and transcriptions for a complete picture.
- As a user, I want clipboard monitoring to pause when I pause Cortex so my privacy preferences are respected.

## Core Requirements

### Functional Requirements

- **Clipboard watcher:** Background Rust thread polling `NSPasteboard.generalPasteboard.changeCount` every 1 second. When changeCount differs from last seen value, read content. Use `swift-rs` or `objc` crate to access NSPasteboard API.
- **Content detection:** Check pasteboard for `public.utf8-plain-text` and `public.url` types. Classify as "text" or "url". Skip if only image data present (log but don't store for MVP).
- **Source app tagging:** Call `accessibility::get_focused_app()` when a clipboard change is detected to record which app the user copied from.
- **Storage:** Insert into `clipboard_entries` table with timestamp, content_type, text_content, and source_app. Simultaneously insert into `clipboard_fts` for text content.
- **Privacy mode:** Check `CaptureState.status` before storing. If paused, skip the clipboard entry.
- **Password exclusion:** Maintain a list of known password manager bundle IDs (`com.1password.1password`, `com.bitwarden.desktop`, `com.lastpass.lastpass`). If source app matches, skip the entry.
- **Schema migration v6:** Bump `CURRENT_SCHEMA_VERSION` to 6. Create `clipboard_entries` table and `clipboard_fts` virtual table.
- **Search integration:** Add a UNION ALL branch to `search_captures` querying `clipboard_fts`. Return with `result_type: "clipboard"`, `app_name` from source_app, `snippet` from FTS snippet function.

### Non-Functional Requirements

- Clipboard polling must not noticeably impact system performance (< 0.1% CPU).
- Clipboard content should appear in search within 2 seconds of copying.
- Watcher thread must handle pasteboard errors gracefully (log and continue, never crash).

## Reusable Components

### Existing Code to Leverage

- **`capture.rs`** -- Background thread pattern: `std::thread::spawn` with `AtomicBool` stop flag and periodic sleep. Mirror for clipboard watcher.
- **`accessibility.rs`** -- `get_focused_app()` for source app detection.
- **`storage.rs`** -- Migration pattern, FTS5 table creation (`captures_fts`, `transcriptions_fts` as templates).
- **`search.rs`** -- `search_captures` UNION ALL pattern, `SearchResult` struct.

### New Code Required

- **`src-tauri/src/clipboard.rs`** -- Clipboard watcher thread, NSPasteboard polling, content classification.
- **Migration v6** in `storage.rs` -- clipboard_entries table, clipboard_fts.
- **UNION branch** in `search.rs` -- clipboard_fts query.
- **`src/routes/clipboard/+page.svelte`** -- Clipboard history list view.

## Technical Approach

### NSPasteboard Access

Use `swift-rs` (already in the project for Vision OCR) to bridge to NSPasteboard:

```swift
@_cdecl("get_clipboard_change_count")
func getClipboardChangeCount() -> Int {
    return NSPasteboard.general.changeCount
}

@_cdecl("get_clipboard_text")
func getClipboardText() -> SRString? {
    guard let text = NSPasteboard.general.string(forType: .string) else { return nil }
    return SRString(text)
}
```

Rust side polls `get_clipboard_change_count()`, compares to stored value, and reads content on change.

### Watcher Thread

```rust
pub fn start_clipboard_watcher(db: Arc<Database>, capture_state: SharedCaptureState, stop_flag: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        let mut last_change_count: i64 = -1;
        loop {
            if stop_flag.load(Ordering::Relaxed) { break; }
            // Check capture state -- skip if paused
            // Poll NSPasteboard changeCount
            // If changed: read content, check exclusions, store
            std::thread::sleep(Duration::from_secs(1));
        }
    });
}
```

### Schema

```sql
CREATE TABLE clipboard_entries (
    id INTEGER PRIMARY KEY,
    timestamp TEXT NOT NULL,
    content_type TEXT NOT NULL DEFAULT 'text',
    text_content TEXT,
    image_path TEXT,
    source_app TEXT NOT NULL DEFAULT ''
);
CREATE INDEX idx_clipboard_timestamp ON clipboard_entries(timestamp);
CREATE INDEX idx_clipboard_source ON clipboard_entries(source_app);
CREATE VIRTUAL TABLE clipboard_fts USING fts5(entry_id, text_content, tokenize='unicode61');
```

### Testing

- Unit test: migration v6 creates clipboard_entries table and clipboard_fts.
- Unit test: insert + query clipboard entry round-trip.
- Unit test: clipboard FTS search returns matching entries.
- Unit test: password manager exclusion skips entries from known bundle IDs.
- Integration test: unified search returns clipboard results alongside OCR results.

## Out of Scope

- Rich text / HTML clipboard content
- File path clipboard monitoring
- Image clipboard storage
- Clipboard sync across devices
- Active clipboard management (paste from history)
- Clipboard pinning, favorites, or tagging
- Content deduplication
- Clipboard entry editing

## Success Criteria

- Clipboard changes detected within 1 second of copy action.
- Text and URL content stored with correct timestamps and source app.
- Clipboard content searchable via unified search.
- Privacy mode pauses clipboard monitoring.
- Password manager entries excluded.
- Clipboard history view shows recent entries with metadata.
