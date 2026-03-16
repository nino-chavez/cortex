# Spec Requirements: Clipboard History

## Initial Description
Clipboard History -- Background clipboard monitoring that watches the macOS pasteboard for changes, stores text and URL content in a new `clipboard_entries` table with FTS5 indexing, and integrates into the existing unified search. Extends Cortex's capture surface beyond screen and audio to include everything the user copies.

## Requirements Discussion

### First Round Questions

**Q1: Proof of Life**
**Answer:** User copies text in Chrome, switches to Cursor and copies code, copies a URL from Slack. All three entries appear in the clipboard history view with timestamps and content type labels. User searches "API key" in unified search and the clipboard entry containing that text appears alongside OCR and transcription results.

**Q2: Clipboard Monitoring**
**Answer:** Poll `NSPasteboard.generalPasteboard` every 1 second via a background Rust thread. Compare the pasteboard `changeCount` to the last seen value -- if different, read the content. This is the standard macOS approach for clipboard monitoring (no private APIs). Detect content type: plain text (`public.utf8-plain-text`), URL (`public.url`), or image (`public.png`/`public.tiff`). For MVP, store text and URL content. Image clipboard entries store a reference but skip FTS indexing.

**Q3: Storage Schema**
**Answer:** New `clipboard_entries` table: `id INTEGER PRIMARY KEY`, `timestamp TEXT NOT NULL`, `content_type TEXT NOT NULL` (text/url/image), `text_content TEXT`, `image_path TEXT`, `source_app TEXT` (the app that was active when the copy happened). New `clipboard_fts` FTS5 virtual table on `text_content`. Schema migration v6.

**Q4: Search Integration**
**Answer:** Extend `search_captures` UNION in `search.rs` with a branch querying `clipboard_fts`. Results returned with `result_type: "clipboard"`. Same pattern as OCR and transcription search branches.

**Q5: Privacy**
**Answer:** Clipboard monitoring respects the existing privacy mode. If capture is paused, clipboard monitoring is also paused. Password manager entries (detected by source app bundle ID matching known password managers like 1Password, Bitwarden) are automatically excluded.

**Q6: Out of Scope**
**Answer:** Rich text / HTML clipboard content, file path clipboard content, clipboard sync across devices, clipboard pinning or favorites, clipboard-to-clipboard paste (this is read-only monitoring, not a clipboard manager).

### Existing Code to Reference
- **capture.rs** -- Background thread pattern with `stop_flag` (AtomicBool), `CaptureState` for pause/resume. Mirror this pattern for clipboard watcher.
- **storage.rs** -- Migration pattern, FTS5 table creation, `insert_transcription` as template for insert method.
- **search.rs** -- `search_captures` UNION pattern. Add clipboard_fts branch.
- **accessibility.rs** -- `get_focused_app()` returns current app info. Use to tag clipboard entries with source app.

## Requirements Summary

### Functional Requirements
- Background thread polling NSPasteboard every 1 second
- Detect and store text and URL clipboard content
- Tag entries with timestamp, content type, and source application
- `clipboard_entries` table with id, timestamp, content_type, text_content, image_path, source_app
- `clipboard_fts` FTS5 virtual table on text_content
- Schema migration v6
- Extend unified search to include clipboard_fts results
- Respect privacy mode (pause when capture is paused)
- Exclude password manager entries by bundle ID

### Tauri Commands
- `get_clipboard_history(limit: i64)` -- Returns recent clipboard entries
- `search_clipboard(query: String)` -- FTS5 search on clipboard content
- `clear_clipboard_history()` -- Deletes all clipboard entries

### Schema Changes
- `clipboard_entries` table (new)
- `clipboard_fts` FTS5 virtual table (new)
- Schema migration v6

### Scope Boundaries
**In Scope:**
- Clipboard change detection via NSPasteboard changeCount polling
- Text and URL content storage
- FTS5 indexing of text content
- Unified search integration
- Source app tagging via accessibility API
- Privacy mode respect
- Password manager exclusion
- Basic clipboard history view

**Out of Scope:**
- Rich text / HTML content
- File path clipboard content
- Image clipboard content storage (detect but don't store for MVP)
- Clipboard sync across devices
- Clipboard pinning, favorites, or organization
- Active clipboard management (paste from history)
- Clipboard content deduplication
