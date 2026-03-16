# Task Breakdown: OCR Pipeline

## Overview
Total Tasks: 4 groups, 24 subtasks
Strategy: squad
Depth: standard
Assigned roles: api-engineer (Swift bridge + Tauri commands), database-engineer (schema + FTS5 + search), testing-engineer (gap analysis + integration tests)

## Task List

### Proof of Life — Vertical Slice

#### Task Group 1: End-to-End OCR + Search
**Assigned implementer:** api-engineer, database-engineer
**Dependencies:** Capture Daemon spec (Task Groups 1-2) must be complete — `captures` table and `storage.rs` with migration system exist.

This group delivers the Proof of Life scenario: user captures a screen showing specific text (e.g., error code `E0308`), calls `search_captures("E0308")`, and receives the correct capture ID, timestamp, and text snippet. It is a vertical slice across Swift bridge, schema migration, FTS5, OCR processing, and search.

- [ ] 1.0 Complete minimal end-to-end OCR pipeline (Swift bridge -> OCR processing -> FTS5 storage -> search command)
  - [ ] 1.1 Create `src-tauri/swift-lib/Package.swift` — Define a Swift package named `SwiftLib` with a library target. Set minimum macOS deployment target to 13.0. Add no external dependencies (Vision framework is a system framework).
  - [ ] 1.2 Create `src-tauri/swift-lib/Sources/SwiftLib/ocr.swift` — Implement `@_cdecl("recognize_text")` function that accepts an image path (`SRString`) and returns recognized text (`SRString`). Use `VNRecognizeTextRequest` with `.accurate` recognition level, English-only (`recognitionLanguages = ["en-US"]`). Load image from file path via `CGImage`, concatenate all recognized text observations separated by newlines. Return empty string on any failure.
  - [ ] 1.3 Update `src-tauri/Cargo.toml` — Add `swift-rs` as a build dependency under `[build-dependencies]`. Verify `rusqlite` already has `"bundled"` feature (which includes FTS5 support).
  - [ ] 1.4 Update `src-tauri/build.rs` — Import `SwiftLinker` from `swift-rs::build`. Call `SwiftLinker::new("13.0").with_package("swift-lib", "swift-lib/").link()` to compile and link the Swift package during build.
  - [ ] 1.5 Create `src-tauri/src/ocr.rs` — Declare the extern FFI function: `extern "C" { fn recognize_text(path: SRString) -> SRString; }`. Wrap it in a safe public function `pub fn recognize_text_from_file(path: &str) -> Option<String>` that calls the FFI, converts the result, and returns `None` if the result is empty.
  - [ ] 1.6 Add schema migration v1 to v2 in `src-tauri/src/storage.rs` — In the migration system, add migration v2: `ALTER TABLE captures ADD COLUMN ocr_status TEXT NOT NULL DEFAULT 'pending'`, `CREATE INDEX idx_captures_ocr_status ON captures(ocr_status)`, `CREATE VIRTUAL TABLE IF NOT EXISTS captures_fts USING fts5(capture_id, ocr_text, tokenize='unicode61')`. Bump `CURRENT_SCHEMA_VERSION` to 2.
  - [ ] 1.7 Add `process_single_capture_ocr()` to `storage.rs` — Given a capture ID: set `ocr_status` to `processing`, call `recognize_text_from_file` with the capture's `image_path`, on success insert into `captures_fts` and set status to `completed`, on failure set status back to `pending` (retry logic comes in Group 2).
  - [ ] 1.8 Create `src-tauri/src/search.rs` — Implement `search_captures(query: &str) -> Vec<SearchResult>` that executes an FTS5 `MATCH` query joining `captures_fts` against `captures`. Return `SearchResult { capture_id, timestamp, app_name, snippet, image_path }`. Use `snippet(captures_fts, 1, '<b>', '</b>', '...', 32)` for snippet extraction. No filters yet (app/time filters come in Group 3).
  - [ ] 1.9 Register `search_captures` as a Tauri command in `lib.rs` — Expose the search function to the frontend/debug console. Wire up the `Database` state access.
  - [ ] 1.10 Add `ocr` and `search` modules to `lib.rs` — Declare `mod ocr; mod search;` and ensure they compile.
  - [ ] 1.11 Write 4 tests: (a) FTS5 table creation and insert/query round-trip, (b) migration v1 to v2 adds `ocr_status` column with default `pending`, (c) search returns correct capture ID and snippet for a known inserted text, (d) search returns empty vec for non-matching query.

**Acceptance Criteria:**
- `cargo build --manifest-path src-tauri/Cargo.toml` compiles the Swift package and links successfully
- Schema migration upgrades a v1 database to v2 without data loss
- `captures_fts` virtual table is created and accepts FTS5 queries
- `recognize_text` FFI call processes a screenshot and returns extracted text
- `search_captures("E0308")` returns the correct capture with a snippet containing the search term
- All 4 tests pass

**Verification Steps:**
1. Build the project and confirm Swift linking succeeds
2. Run the app, capture a screen with known text, manually trigger OCR on that capture, then search for the text
3. Query the FTS5 table directly to verify indexed content

**Verification Commands:**
```bash
# Build (verifies Swift bridge compiles and links)
cargo build --manifest-path src-tauri/Cargo.toml

# Run tests
cargo test --manifest-path src-tauri/Cargo.toml --lib

# Verify schema migration
sqlite3 ~/.cortex/cortex.db "PRAGMA table_info(captures);" | grep ocr_status

# Verify FTS5 table exists
sqlite3 ~/.cortex/cortex.db "SELECT * FROM captures_fts LIMIT 1;"

# Verify search works (after processing at least one capture)
sqlite3 ~/.cortex/cortex.db "SELECT capture_id, snippet(captures_fts, 1, '<b>', '</b>', '...', 32) FROM captures_fts WHERE captures_fts MATCH 'test';"
```

---

### Background Processing

#### Task Group 2: Background Worker
**Assigned implementer:** api-engineer
**Dependencies:** Task Group 1 (Swift bridge, schema, and OCR processing exist)

- [ ] 2.0 Complete background OCR worker with polling, status transitions, retry logic, and non-blocking operation
  - [ ] 2.1 Create `src-tauri/src/ocr_worker.rs` — Implement `start_ocr_worker(db: Arc<Database>, stop_flag: Arc<AtomicBool>)` that spawns a background thread. Follow the same pattern as `start_capture_loop` in `capture.rs`.
  - [ ] 2.2 Implement polling loop — Query for a batch of captures where `ocr_status = 'pending'`, ordered by `timestamp DESC` (newest first), limit 10 per batch. If no pending work, sleep 3 seconds before next poll.
  - [ ] 2.3 Implement status transitions — For each capture in the batch: set `ocr_status` to `processing`, call `recognize_text_from_file`, on success insert into `captures_fts` and set to `completed`, on failure increment a retry counter.
  - [ ] 2.4 Implement retry logic — Add `ocr_retries INTEGER NOT NULL DEFAULT 0` to the captures table (as part of migration v2, update Group 1's migration if needed, or add as migration v3). After 3 failed attempts (`ocr_retries >= 3`), set `ocr_status` to `failed` and skip the capture permanently.
  - [ ] 2.5 Wire worker startup into `lib.rs` — Start the OCR worker during Tauri app initialization, after storage is initialized. Pass the shared `Arc<Database>` and a stop flag.
  - [ ] 2.6 Write 3 tests: (a) worker processes a pending capture and sets status to `completed`, (b) worker skips captures already marked `completed` or `failed`, (c) worker sets status to `failed` after 3 retry attempts.

**Acceptance Criteria:**
- OCR worker runs on a dedicated background thread and does not block the capture loop
- New captures are processed automatically without user intervention
- Newest captures are processed first (most likely to be searched soon)
- Failed captures are retried up to 3 times then marked `failed`
- Worker sleeps 3 seconds between polls when idle (no busy-waiting)
- Worker stops cleanly when the stop flag is set

**Verification Steps:**
1. Start the app, capture several screens, wait 10 seconds — expect `ocr_status = 'completed'` for recent captures
2. Verify no capture interval drift while OCR is processing
3. Provide an invalid image path — expect retry then `failed` status

**Verification Commands:**
```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib -- ocr_worker

# Check OCR processing status after running the app
sqlite3 ~/.cortex/cortex.db "SELECT ocr_status, COUNT(*) FROM captures GROUP BY ocr_status;"

# Verify newest-first processing order
sqlite3 ~/.cortex/cortex.db "SELECT timestamp, ocr_status FROM captures ORDER BY timestamp DESC LIMIT 20;"
```

---

### Search & Backfill

#### Task Group 3: Search Enhancements & Backfill
**Assigned implementer:** database-engineer
**Dependencies:** Task Groups 1 and 2 (search and worker exist)

- [ ] 3.0 Complete search filters, backfill command, and OCR status command
  - [ ] 3.1 Add app name filter to `search_captures` — Accept `app_filter: Option<String>` parameter. When provided, add `AND c.app_name = ?` to the search query joining `captures_fts` with `captures`.
  - [ ] 3.2 Add time range filter to `search_captures` — Accept `time_from: Option<String>` and `time_to: Option<String>` parameters (ISO-8601 strings). When provided, add `AND c.timestamp >= ?` and/or `AND c.timestamp <= ?` clauses.
  - [ ] 3.3 Update the `search_captures` Tauri command signature — Pass through the new optional filter parameters from the frontend/debug console.
  - [ ] 3.4 Implement `backfill_ocr()` — Set `ocr_status = 'pending'` on all captures that lack a corresponding `captures_fts` entry. Return a `BackfillStatus { queued: u64 }` with the count of captures marked for processing. The existing background worker handles the actual processing.
  - [ ] 3.5 Register `backfill_ocr` as a Tauri command in `lib.rs`.
  - [ ] 3.6 Implement `get_ocr_status()` — Query `SELECT ocr_status, COUNT(*) FROM captures GROUP BY ocr_status` and return `OcrStatusCounts { pending: u64, completed: u64, failed: u64 }`.
  - [ ] 3.7 Register `get_ocr_status` as a Tauri command in `lib.rs`.
  - [ ] 3.8 Write 4 tests: (a) search with app_filter returns only captures from that app, (b) search with time range returns only captures within the range, (c) backfill_ocr marks the correct captures as pending, (d) get_ocr_status returns accurate counts.

**Acceptance Criteria:**
- `search_captures("query", Some("Cursor"), None, None)` returns only results from the Cursor app
- `search_captures("query", None, Some("2026-03-16T00:00:00"), Some("2026-03-16T23:59:59"))` returns only results from that day
- `backfill_ocr()` correctly identifies and queues unprocessed captures
- `get_ocr_status()` returns accurate counts matching the database state
- All filters compose correctly (app + time range + query)

**Verification Steps:**
1. Insert test data with multiple apps and timestamps, search with filters, verify correct filtering
2. Run backfill_ocr on a database with unprocessed captures, verify they get queued
3. Check get_ocr_status matches manual SQL counts

**Verification Commands:**
```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib -- search

# Test search with filters (after populating data)
sqlite3 ~/.cortex/cortex.db "SELECT c.capture_id, c.app_name, snippet(f, 1, '<b>', '</b>', '...', 32) FROM captures_fts f JOIN captures c ON c.id = f.capture_id WHERE f.captures_fts MATCH 'test' AND c.app_name = 'Cursor';"

# Check OCR status counts
sqlite3 ~/.cortex/cortex.db "SELECT ocr_status, COUNT(*) FROM captures GROUP BY ocr_status;"
```

---

### Testing & Integration

#### Task Group 4: Test Review, Gap Analysis, Integration Verification
**Assigned implementer:** testing-engineer
**Dependencies:** Task Groups 1, 2, 3

- [ ] 4.0 Complete test coverage review and fill gaps with integration tests
  - [ ] 4.1 Review all tests from Groups 1-3 (11 total). Verify they compile and pass. Document any that are flaky or environment-dependent (e.g., Swift bridge tests require macOS with Vision framework).
  - [ ] 4.2 Integration test: full OCR pipeline — Start capture, wait for background worker to process, call `search_captures` with text known to be on screen. Assert: (a) search returns at least one result, (b) result contains a non-empty snippet, (c) `ocr_status` is `completed` for the processed capture.
  - [ ] 4.3 Integration test: search latency — Insert 1,000+ FTS5 rows with synthetic OCR text, run `search_captures` 10 times, assert average latency is under 200ms.
  - [ ] 4.4 Integration test: backfill correctness — Create 20 captures with `ocr_status = 'completed'` and 10 with `ocr_status = 'pending'` (no FTS5 entries). Run `backfill_ocr()`. Assert exactly 10 captures are queued.
  - [ ] 4.5 Integration test: migration safety — Create a v1 database with 5 capture rows. Run migration to v2. Assert: (a) all 5 rows still exist, (b) all have `ocr_status = 'pending'`, (c) `captures_fts` table exists, (d) schema_version = 2.
  - [ ] 4.6 Integration test: failed OCR handling — Insert a capture row pointing to a non-existent image file. Run the worker for 3 iterations. Assert `ocr_status = 'failed'` and `ocr_retries = 3`. Assert the worker continues processing other captures.
  - [ ] 4.7 Gap analysis — Document untested paths: concurrent OCR + capture under heavy CPU load, very large images (>4K), screenshots with no text (expect empty FTS5 entry or skip), disk full during FTS5 insert, Vision framework unavailable (e.g., Linux CI). File as future test TODOs in a comment block.

**Acceptance Criteria:**
- All tests from Groups 1-3 pass
- 5 new integration tests added and passing
- Gap analysis identifies at least 4 untested edge cases
- Total test count: 16+ (11 from Groups 1-3 + 5 new)
- Search latency confirmed under 200ms for 1,000+ rows

**Verification Steps:**
1. Run full test suite — expect all tests pass
2. Review test output for flaky tests or warnings
3. Verify gap analysis documents meaningful risk areas

**Verification Commands:**
```bash
# Run all tests
cargo test --manifest-path src-tauri/Cargo.toml --lib

# Run tests with output for debugging
cargo test --manifest-path src-tauri/Cargo.toml --lib -- --nocapture

# List all tests
cargo test --manifest-path src-tauri/Cargo.toml --lib -- --list 2>&1 | tail -1

# Build release to verify no compile warnings
cargo build --manifest-path src-tauri/Cargo.toml --release 2>&1 | grep warning
```

---

## Execution Order

1. **Task Group 1: End-to-End OCR + Search** (api-engineer + database-engineer) — Proof of Life vertical slice. Must complete first.
2. **Task Group 2: Background Worker** (api-engineer) — Depends on Group 1 for Swift bridge and schema.
3. **Task Group 3: Search Enhancements & Backfill** (database-engineer) — Depends on Groups 1 and 2. Can run in parallel with Group 2 if search filter work starts while worker is being built.
4. **Task Group 4: Test Review & Integration** (testing-engineer) — Depends on all prior groups completing.

**Parallel execution possible:** Groups 2 and 3 can partially overlap after Group 1 completes — the search filter work (3.1-3.3) only depends on Group 1, while backfill (3.4-3.5) depends on Group 2's worker.
