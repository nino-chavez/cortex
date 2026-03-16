# Spec Requirements: OCR Pipeline

## Initial Description
OCR Pipeline — Process each screenshot through Apple Vision framework (via swift-rs bridge) to extract all visible text. Store extracted text in SQLite with FTS5 full-text indexing. Support keyword search across captured text with app and time filters.

## Requirements Discussion

### First Round Questions

**Q1: Proof of Life**
**Answer:** Start capture, perform a task (e.g., look at a specific line of code in Cursor or a specific price on a website). Open a terminal or debug console, run a search command for that specific string, and receive the correct timestamp and file path to the screenshot where that text exists.

**Q2: Value Signal**
**Answer:** "Searchable History" — user remembers seeing a specific error code or hex color value three hours ago but can't find the tab. They search the string, and the system brings them exactly to that moment. Metric: search latency under 200ms for a local database of 10,000+ captures.

**Q3: Processing Mode**
**Answer:** Asynchronous background worker. The capture daemon must remain lightweight and focused on ScreenCaptureKit. A separate worker (background thread in same Rust binary) polls for "pending" OCR tasks. Prevents capture interval drift if OCR takes longer than 5 seconds during high CPU load.

**Q4: Apple Vision vs Alternatives**
**Answer:** Stick with Apple Vision. Hardware-accelerated on Neural Engine, better accuracy than Tesseract for screen content, and Swift infrastructure already in place via screencapturekit.

**Q5: Text Storage**
**Answer:** Separate FTS5 virtual table, NOT a column on captures. Schema: `captures_fts (capture_id, ocr_text)`. Keeps main captures table lean, separates data from index, cleaner for migrations.

**Q6: Search API**
**Answer:** Backend-only Tauri commands. `search_captures(query: String) -> Vec<CaptureResult>` returning capture_id, timestamp, and snippet of matching text. No UI in this spec.

**Q7: Backfill**
**Answer:** Yes, a `backfill_ocr` command is required. On first launch of OCR-enabled version, quietly process the backlog in the background. Users will have hundreds of screenshots from the daemon-only phase.

**Q8: Language**
**Answer:** English-only for now. Simplifies FTS5 tokenizer config and keeps initial build size down.

**Q9: Out of Scope**
**Answer:** Confirmed deferred: semantic/vector search (spec #4), search UI/timeline (spec #5), screen understanding/summarization.

### Existing Code to Reference

- **storage.rs** — Primary touchpoint. Need to add `ocr_status` column to captures table (PENDING, COMPLETED, FAILED) for worker progress tracking.
- **swift-rs** — Already a transitive dependency via screencapturekit. Use same bridge patterns for VNRecognizeTextRequest API.
- **build.rs** — Already has Swift runtime rpath configured.

### Follow-up Questions
None needed — answers were comprehensive.

## Visual Assets

No visual assets provided.

## Requirements Summary

### Functional Requirements
- Background OCR worker thread that processes screenshots asynchronously
- Apple Vision framework (VNRecognizeTextRequest) via swift-rs FFI bridge
- Extract all visible text from each captured screenshot
- Store OCR results in a separate FTS5 virtual table (`captures_fts`)
- Add `ocr_status` column to `captures` table (PENDING/COMPLETED/FAILED)
- Full-text keyword search via FTS5 with snippet extraction
- Search filters: app name, time range
- Search latency under 200ms for 10,000+ captures
- Backfill command to process existing captures without OCR text
- English-only text recognition

### Schema Changes
- Add `ocr_status TEXT NOT NULL DEFAULT 'pending'` to `captures` table (migration v2)
- Create FTS5 virtual table: `captures_fts(capture_id, ocr_text)`

### Search API (Tauri Commands)
- `search_captures(query, app_filter, time_from, time_to)` → Vec of {capture_id, timestamp, app_name, snippet, image_path}
- `backfill_ocr()` → triggers background processing of all pending captures
- `get_ocr_status()` → returns count of pending/completed/failed

### Scope Boundaries
**In Scope:**
- OCR processing pipeline (Swift bridge + background worker)
- FTS5 indexing and search
- Backfill for existing captures
- Search Tauri commands with filters

**Out of Scope:**
- Semantic/vector search
- Search UI (frontend)
- Timeline view
- Multi-language support
- Screen understanding / summarization

### Technical Considerations
- Swift bridge via swift-rs for VNRecognizeTextRequest
- Background worker must not block the capture loop
- Schema migration from v1 → v2
- FTS5 tokenizer: unicode61 (default, good for English)
- Worker should process newest captures first (most likely to be searched)
- Error handling: mark captures as FAILED after N retries, don't block the queue
