# Specification: OCR Pipeline

## Goal

Extract visible text from every screenshot using Apple Vision framework via swift-rs, store results in an FTS5 virtual table, and expose keyword search through Tauri commands -- turning Cortex's visual capture history into a searchable text archive.

## Proof of Life

**Scenario:** User captures a screen showing a specific error code (e.g., `E0308`), then calls `search_captures("E0308")` from a debug console or test harness and receives back the correct capture ID, timestamp, and a text snippet containing the error code.

**Validates:** The full pipeline works end-to-end -- Vision OCR extracts text, FTS5 indexes it, and search returns accurate results with acceptable latency.

**Must work before:** Search UI (spec #5) and semantic/vector search (spec #4) can begin.

## User Stories

- As a user, I want every new screenshot to be automatically OCR-processed so my capture history becomes searchable without any manual action.
- As a user, I want to search for a keyword I saw on screen hours ago and get back the exact capture where it appeared, with a relevant text snippet.
- As a user, I want my existing pre-OCR screenshots to be backfilled so my full history is searchable after upgrading.
- As a user, I want to filter search results by app name and time range so I can narrow down results quickly.
- As a user, I want to check how many captures are still pending OCR so I know when backfill is complete.

## Core Requirements

### Functional Requirements

- **Background OCR worker:** A dedicated thread in the Rust binary polls `captures` for rows with `ocr_status = 'pending'`, processes them through Apple Vision, and writes results to `captures_fts`. Must not block the capture loop.
- **Apple Vision bridge:** Call `VNRecognizeTextRequest` (accurate recognition level) via swift-rs FFI. Accept an image file path, return recognized text as a single concatenated `String`.
- **FTS5 storage:** Create virtual table `captures_fts(capture_id, ocr_text)` using the `unicode61` tokenizer. Insert one row per successfully processed capture.
- **Status tracking:** Add `ocr_status TEXT NOT NULL DEFAULT 'pending'` column to `captures` table. Valid states: `pending`, `processing`, `completed`, `failed`.
- **Search command:** `search_captures(query, app_filter, time_from, time_to)` returns a `Vec` of `{capture_id, timestamp, app_name, snippet, image_path}`. Uses FTS5 `MATCH` with `snippet()` function. Latency target: <200ms over 10,000+ captures.
- **Backfill command:** `backfill_ocr()` marks all captures with `ocr_status = 'pending'` that lack an FTS5 entry, then lets the worker process them. Newest captures first.
- **Status command:** `get_ocr_status()` returns counts of pending, completed, and failed captures.
- **Error handling:** After 3 failed attempts, mark a capture as `failed` and skip it. Never block the queue on a single bad image.
- **English-only** text recognition.

### Non-Functional Requirements

- Search latency under 200ms for 10,000+ indexed captures.
- OCR worker must not cause capture interval drift -- runs on a separate thread with its own timing.
- Worker processes newest captures first (most likely to be searched soon).
- Worker should sleep/poll at a reasonable interval (e.g., 2-5 seconds) when no pending work exists to avoid busy-waiting.
- Schema migration must be non-destructive -- existing v1 databases upgrade cleanly to v2.

## Visual Design

No UI in this spec. All interaction is through Tauri commands and the debug console.

## Conversion Design

Not applicable -- this is a backend pipeline with no user-facing interface.

## Reusable Components

### Existing Code to Leverage

- **`storage.rs`** -- `Database` struct with `Mutex<Connection>`, migration system (`run_migrations`, `schema_version`), and `CaptureRow` struct. Migration v2 extends the existing v1 migration chain.
- **`capture.rs`** -- `start_capture_loop` pattern shows how to spawn a background thread with `Arc<Database>` and a shared state mutex. OCR worker follows the same pattern.
- **`Cargo.toml`** -- `rusqlite` already included with `bundled` feature (includes FTS5). `swift-rs` is a transitive dependency via `screencapturekit`. `build.rs` already configures Swift runtime rpath.
- **`CaptureRow`** struct -- extend with `ocr_status` field for queries that need it.

### New Components Required

- **`ocr.rs`** -- Swift FFI bridge module. Declares the `extern "C"` function for Vision text recognition. Handles the Rust side of the swift-rs call.
- **`ocr_worker.rs`** -- Background worker module. Polling loop, status transitions, retry logic, FTS5 insertion.
- **`ocr.swift`** -- Swift source file implementing `VNRecognizeTextRequest` and exposing it via swift-rs `@_cdecl` convention.
- **Tauri commands** -- `search_captures`, `backfill_ocr`, `get_ocr_status` registered in the Tauri command handler.

## Technical Approach

### Database

- **Migration v1 to v2:** `ALTER TABLE captures ADD COLUMN ocr_status TEXT NOT NULL DEFAULT 'pending'`. Add index: `CREATE INDEX idx_captures_ocr_status ON captures(ocr_status)`.
- **FTS5 table:** `CREATE VIRTUAL TABLE IF NOT EXISTS captures_fts USING fts5(capture_id, ocr_text, tokenize='unicode61')`.
- **Search query:** Join `captures_fts` against `captures` using `capture_id`, apply optional `WHERE` clauses for `app_name` and timestamp range, use `snippet(captures_fts, 1, '<b>', '</b>', '...', 32)` for context extraction.
- Bump `CURRENT_SCHEMA_VERSION` from 1 to 2.

### API (Tauri Commands)

- `search_captures(query: String, app_filter: Option<String>, time_from: Option<String>, time_to: Option<String>) -> Vec<SearchResult>` -- FTS5 MATCH query with filters.
- `backfill_ocr() -> BackfillStatus` -- Sets all NULL/pending OCR captures to pending and returns the count queued.
- `get_ocr_status() -> OcrStatusCounts` -- Returns `{ pending: u64, completed: u64, failed: u64 }`.

### Swift Bridge

- New Swift file exposes a `@_cdecl("recognize_text")` function taking an image path (`SRString`) and returning recognized text (`SRString`).
- Uses `VNRecognizeTextRequest` with `.accurate` recognition level.
- Concatenates all recognized text observations into a single string, separated by newlines.
- Returns empty string on failure (Rust side interprets empty string + logs as a soft failure).

### Background Worker

- Spawns via `std::thread::spawn` during app initialization, similar to `start_capture_loop`.
- Receives `Arc<Database>` and a shared stop flag.
- Poll loop: query for batch of captures where `ocr_status = 'pending'`, ordered by `timestamp DESC` (newest first), limit 10 per batch.
- For each capture: set status to `processing`, call Swift bridge with `image_path`, on success insert into `captures_fts` and set status to `completed`, on failure increment retry count and set to `failed` after 3 attempts.
- Sleep 3 seconds between polls when no work is found.

### Testing

- **Unit tests for FTS5:** Insert known text, verify `MATCH` queries return correct capture IDs and snippets.
- **Unit tests for migration:** Open a v1 database, run migration, verify `ocr_status` column exists and defaults to `pending`.
- **Integration test for search:** Insert captures with OCR text, run `search_captures` with various filters, verify results and ordering.
- **Swift bridge test:** Call `recognize_text` with a known screenshot containing specific text, verify the returned string contains that text.
- **Worker test:** Insert captures with `pending` status, run one worker iteration, verify status transitions to `completed` and FTS5 rows exist.

## Out of Scope

- Semantic/vector search (spec #4)
- Search UI / timeline view (spec #5)
- Multi-language OCR support
- Screen understanding or summarization
- Image preprocessing or enhancement before OCR
- OCR confidence scores or per-word bounding boxes
- Real-time streaming OCR (only file-based processing)

## Success Criteria

- A freshly captured screenshot has its text extractable and searchable within 10 seconds of capture.
- `search_captures("specific_keyword")` returns correct results in under 200ms with 10,000+ indexed captures.
- `backfill_ocr()` processes existing captures without blocking the capture loop or degrading capture interval.
- Schema migration from v1 to v2 preserves all existing capture data and sets `ocr_status = 'pending'` on all rows.
- Failed OCR attempts are retried up to 3 times, then marked `failed` without blocking the queue.
- `get_ocr_status()` accurately reports pending, completed, and failed counts.
