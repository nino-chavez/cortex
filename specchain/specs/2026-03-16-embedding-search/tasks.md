# Task Breakdown: Embedding & Semantic Search

## Overview
Total Tasks: 4 groups, 28 subtasks
Strategy: squad
Depth: standard
Assigned roles: ml-engineer (candle integration + embedding module), database-engineer (schema + sqlite-vec + search), systems-engineer (worker + model management), testing-engineer (gap analysis + integration tests)

## Task List

### Proof of Life — Vertical Slice

#### Task Group 1: Embedding + Vector Search End-to-End
**Assigned implementer:** ml-engineer, database-engineer
**Dependencies:** OCR Pipeline spec (Task Groups 1-3) must be complete -- `captures` table with `ocr_status`, `captures_fts`, `search.rs`, `ocr_worker.rs`, and migration system at v3 exist.

This group delivers the Proof of Life scenario: insert text, generate an embedding via candle, store it in sqlite-vec, query with different but semantically related text, and get the correct result back. It is a vertical slice across candle inference, sqlite-vec storage, and semantic search.

- [ ] 1.0 Complete minimal end-to-end embedding + semantic search (candle model load -> embed text -> sqlite-vec insert -> cosine query -> search command)
  - [ ] 1.1 Update `src-tauri/Cargo.toml` -- Add dependencies: `candle-core`, `candle-nn`, `candle-transformers` (with `metal` feature for Apple Silicon acceleration), `tokenizers` (Hugging Face tokenizer crate), `sqlite-vec` (or `sqlite-vss` depending on available crate -- may need to vendor the `.dylib` and load via `rusqlite::Connection::load_extension`). Pin versions compatible with current `rusqlite 0.36`.
  - [ ] 1.2 Add sqlite-vec extension loading to `storage.rs` -- In `Database::open`, after `Connection::open`, call `conn.load_extension(path_to_vec0_dylib, None)` to load the sqlite-vec runtime extension. The `.dylib` should be bundled in the app resources or built from source via `build.rs`. Enable extension loading with `conn.execute_batch("SELECT load_extension('vec0')")` or equivalent rusqlite API.
  - [ ] 1.3 Add schema migration v4 in `storage.rs` -- In the migration chain, add `if version < 4` block:
    - `ALTER TABLE captures ADD COLUMN embedding_status TEXT NOT NULL DEFAULT 'pending'`
    - `ALTER TABLE captures ADD COLUMN embedding_retries INTEGER NOT NULL DEFAULT 0`
    - `ALTER TABLE transcriptions ADD COLUMN embedding_status TEXT NOT NULL DEFAULT 'pending'`
    - `ALTER TABLE transcriptions ADD COLUMN embedding_retries INTEGER NOT NULL DEFAULT 0`
    - `CREATE INDEX idx_captures_embedding_status ON captures(embedding_status)`
    - `CREATE INDEX idx_transcriptions_embedding_status ON transcriptions(embedding_status)`
    - `CREATE VIRTUAL TABLE vec_captures USING vec0(capture_id INTEGER NOT NULL, chunk_index INTEGER NOT NULL, embedding float[384])`
    - `CREATE VIRTUAL TABLE vec_transcriptions USING vec0(transcription_id INTEGER NOT NULL, chunk_index INTEGER NOT NULL, embedding float[384])`
    - Bump `CURRENT_SCHEMA_VERSION` to 4.
    - Set `embedding_status = 'pending'` for all existing captures where `ocr_status = 'completed'`.
  - [ ] 1.4 Create `src-tauri/src/embedding.rs` -- Implement the core embedding module:
    - `pub struct EmbeddingModel` that holds the loaded candle model and tokenizer in memory.
    - `pub fn load(model_dir: &Path) -> Result<Self>` -- Load `config.json`, `tokenizer.json`, and `model.safetensors` from the given directory using candle. Build the all-MiniLM-L6-v2 model graph.
    - `pub fn embed_text(&self, text: &str) -> Result<Vec<f32>>` -- Tokenize input, run inference, extract the 384-dim embedding vector (mean pooling over token embeddings), normalize to unit length for cosine similarity.
    - Handle the tokenizer's 512-token max input gracefully (truncate for now; chunking is Group 3).
  - [ ] 1.5 Add database methods to `storage.rs` for vector operations:
    - `pub fn insert_capture_embedding(&self, capture_id: i64, chunk_index: i32, embedding: &[f32]) -> Result<()>` -- Insert into `vec_captures`.
    - `pub fn insert_transcription_embedding(&self, transcription_id: i64, chunk_index: i32, embedding: &[f32]) -> Result<()>` -- Insert into `vec_transcriptions`.
    - `pub fn search_similar_captures(&self, query_embedding: &[f32], limit: i64) -> Result<Vec<(i64, f64)>>` -- Query `vec_captures` with cosine distance, return `(capture_id, distance)` pairs.
    - `pub fn set_embedding_status(&self, capture_id: i64, status: &str) -> Result<()>` -- Update `embedding_status` on captures.
    - `pub fn get_pending_embeddings(&self, limit: i64) -> Result<Vec<(i64, String)>>` -- Get captures with `ocr_status = 'completed'` and `embedding_status = 'pending'`, return `(capture_id, ocr_text)`. Newest first.
  - [ ] 1.6 Add semantic search branch to `search.rs` -- Extend `search_captures` with a `mode: &str` parameter (default `"keyword"`). When `mode = "semantic"`:
    - Load or access the embedding model (passed as shared state or lazy-initialized).
    - Embed the query string.
    - Call `search_similar_captures` to get ranked capture IDs.
    - Join back to `captures` table for metadata.
    - Return results as `Vec<SearchResult>` with `result_type = "semantic"`.
    - Preserve existing FTS5 behavior when `mode = "keyword"`.
  - [ ] 1.7 Update the `search_captures` Tauri command in `lib.rs` -- Add `mode` parameter to the command signature. Pass it through to the database method.
  - [ ] 1.8 Add `embedding` module to `lib.rs` -- Declare `mod embedding;` and ensure it compiles.
  - [ ] 1.9 Write 5 tests:
    - (a) Migration v4 adds `embedding_status` column with default `'pending'` and creates `vec_captures` virtual table.
    - (b) `embed_text` returns a 384-dim vector with finite float values (requires model files -- skip in CI if not present).
    - (c) Insert an embedding into `vec_captures` and retrieve it via cosine distance query -- verify correct capture_id is returned.
    - (d) Semantic search returns results ranked by similarity -- insert two captures with embeddings for "Rust programming language" and "cooking pasta recipe", query with "software development", verify the Rust capture ranks first.
    - (e) `search_captures` with `mode = "keyword"` still returns FTS5 results (regression test).

**Acceptance Criteria:**
- `cargo build --manifest-path src-tauri/Cargo.toml` compiles with candle and sqlite-vec dependencies
- Schema migration upgrades a v3 database to v4 without data loss
- `vec_captures` virtual table accepts vector inserts and cosine distance queries
- `embed_text("hello world")` returns a 384-element `Vec<f32>`
- `search_captures("e-commerce project", ..., "semantic")` returns a capture containing "BigCommerce API" text
- Existing keyword search is not broken
- All 5 tests pass

**Verification Steps:**
1. Build the project and confirm candle + sqlite-vec link successfully
2. Run the app, manually embed a capture's OCR text, insert into sqlite-vec, query semantically
3. Query the vec_captures table directly to verify stored vectors

**Verification Commands:**
```bash
# Build (verifies candle + sqlite-vec compile and link)
cargo build --manifest-path src-tauri/Cargo.toml

# Run tests
cargo test --manifest-path src-tauri/Cargo.toml --lib

# Verify schema migration
sqlite3 ~/.cortex/cortex.db "PRAGMA table_info(captures);" | grep embedding_status

# Verify vec_captures table exists
sqlite3 ~/.cortex/cortex.db "SELECT COUNT(*) FROM vec_captures;"

# Verify semantic search (after embedding at least one capture)
sqlite3 ~/.cortex/cortex.db "SELECT capture_id, distance FROM vec_captures WHERE embedding MATCH x'...' ORDER BY distance LIMIT 5;"
```

---

### Background Processing

#### Task Group 2: Background Embedding Worker
**Assigned implementer:** systems-engineer
**Dependencies:** Task Group 1 (embedding module, schema, and vector storage exist)

- [ ] 2.0 Complete background embedding worker with polling, batch processing, status tracking, and retry logic
  - [ ] 2.1 Create `src-tauri/src/embedding_worker.rs` -- Implement `start_embedding_worker(db: Arc<Database>, model: Arc<EmbeddingModel>, stop_flag: Arc<AtomicBool>)` that spawns a background thread. Mirror the `ocr_worker.rs` pattern: polling loop, batch processing, graceful shutdown.
  - [ ] 2.2 Implement polling loop -- Query `get_pending_embeddings(BATCH_SIZE)` for captures with `ocr_status = 'completed'` and `embedding_status = 'pending'`, ordered by `timestamp DESC` (newest first), limit 10 per batch. If no pending work, sleep 5 seconds before next poll.
  - [ ] 2.3 Implement capture embedding processing -- For each capture in the batch:
    - Set `embedding_status` to `'processing'`.
    - Retrieve OCR text from `captures_fts` (join by capture_id).
    - Call `model.embed_text(ocr_text)` to generate the embedding.
    - Insert into `vec_captures` with `chunk_index = 0` (single chunk for now; chunking is Group 3).
    - Set `embedding_status` to `'completed'`.
    - On failure: increment retry counter, set back to `'pending'` if retries < 3, else set to `'failed'`.
  - [ ] 2.4 Implement transcription embedding processing -- Same pattern for transcriptions: poll `transcriptions` where `embedding_status = 'pending'`, embed text, insert into `vec_transcriptions`.
  - [ ] 2.5 Add retry logic -- Add `get_embedding_retries` and `increment_embedding_retries` methods to `storage.rs`. After 3 failed attempts, set `embedding_status = 'failed'` and skip. Mirror `ocr_worker.rs` retry pattern.
  - [ ] 2.6 Implement backfill on startup -- On worker start, run a one-time query: `UPDATE captures SET embedding_status = 'pending' WHERE ocr_status = 'completed' AND embedding_status NOT IN ('completed', 'processing')`. Same for transcriptions. This ensures existing data gets processed.
  - [ ] 2.7 Wire worker startup into `lib.rs` -- Start the embedding worker during Tauri app initialization, after storage and embedding model are initialized. Pass `Arc<Database>`, `Arc<EmbeddingModel>`, and a stop flag. Worker must wait for model to be available (downloaded) before starting its poll loop.
  - [ ] 2.8 Write 4 tests:
    - (a) Worker processes a pending capture and sets `embedding_status` to `'completed'`.
    - (b) Worker skips captures with `embedding_status = 'completed'` or `'failed'`.
    - (c) Worker sets `embedding_status` to `'failed'` after 3 retry attempts.
    - (d) Worker only processes captures where `ocr_status = 'completed'` (does not try to embed captures still pending OCR).

**Acceptance Criteria:**
- Embedding worker runs on a dedicated background thread and does not block capture or OCR workers
- New captures are embedded automatically within 10 seconds of OCR completion
- Newest captures are processed first
- Failed embeddings are retried up to 3 times then marked `failed`
- Worker sleeps 5 seconds between polls when idle
- Worker stops cleanly when the stop flag is set
- Backfill processes all existing completed OCR captures on first launch

**Verification Steps:**
1. Start the app, capture several screens, wait for OCR to complete, then wait 15 seconds -- expect `embedding_status = 'completed'` for processed captures
2. Verify no capture interval or OCR drift while embeddings are processing
3. Provide a capture with empty OCR text -- expect graceful handling

**Verification Commands:**
```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib -- embedding_worker

# Check embedding processing status after running the app
sqlite3 ~/.cortex/cortex.db "SELECT embedding_status, COUNT(*) FROM captures GROUP BY embedding_status;"

# Verify vec_captures has entries
sqlite3 ~/.cortex/cortex.db "SELECT COUNT(*) FROM vec_captures;"

# Verify newest-first processing order
sqlite3 ~/.cortex/cortex.db "SELECT timestamp, embedding_status FROM captures WHERE ocr_status = 'completed' ORDER BY timestamp DESC LIMIT 20;"
```

---

### Chunking & Model Management

#### Task Group 3: Chunking & Model Management
**Assigned implementer:** ml-engineer
**Dependencies:** Task Groups 1 and 2 (embedding module and worker exist)

- [ ] 3.0 Complete text chunking, model download, and embedding status command
  - [ ] 3.1 Create `src-tauri/src/chunker.rs` -- Implement `pub fn chunk_text(text: &str, max_tokens: usize, overlap_pct: f32) -> Vec<String>`:
    - Use the tokenizer from the embedding model to count tokens accurately.
    - Sliding window: 512 tokens per chunk, 10% overlap (51 tokens).
    - If text fits in a single chunk (<=512 tokens), return it as-is in a single-element Vec.
    - Each chunk must be a valid string (no splitting mid-word or mid-UTF8 character).
    - Return `Vec<String>` of chunks.
  - [ ] 3.2 Integrate chunking into embedding worker -- Update `embedding_worker.rs` to use `chunk_text` before embedding. For each capture:
    - Chunk the OCR text.
    - Embed each chunk separately.
    - Insert each chunk embedding into `vec_captures` with sequential `chunk_index` (0, 1, 2, ...).
    - A capture is `completed` only when all chunks are embedded.
  - [ ] 3.3 Create `src-tauri/src/model_manager.rs` -- Implement model download and cache:
    - `pub fn model_dir() -> PathBuf` -- Returns `~/.cortex/models/embed/all-MiniLM-L6-v2/`.
    - `pub fn is_model_downloaded() -> bool` -- Checks for `config.json`, `tokenizer.json`, `model.safetensors`.
    - `pub async fn download_model(progress_cb: impl Fn(f32)) -> Result<PathBuf>` -- Download from `https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main/` via HTTP. Download three files: `config.json`, `tokenizer.json`, `model.safetensors`. Report progress (0.0 to 1.0) via callback. Write to `model_dir()`. Return the model directory path.
    - Do not re-download if files already exist and are non-empty.
  - [ ] 3.4 Wire model download into app startup -- During Tauri app initialization, check if model is downloaded. If not, start download in the background. Embedding worker waits for model availability before starting its poll loop. App remains usable during download (keyword search works, only semantic search is unavailable).
  - [ ] 3.5 Implement `get_embedding_status` Tauri command -- Query counts of pending, completed, and failed for both captures and transcriptions. Also report whether the model is downloaded and ready. Return `EmbeddingStatus { model_ready: bool, captures: { pending, completed, failed }, transcriptions: { pending, completed, failed } }`.
  - [ ] 3.6 Register `get_embedding_status` as a Tauri command in `lib.rs`.
  - [ ] 3.7 Add `chunker` and `model_manager` modules to `lib.rs`.
  - [ ] 3.8 Write 5 tests:
    - (a) `chunk_text` with short text (<512 tokens) returns a single chunk.
    - (b) `chunk_text` with 1024 tokens returns 3 chunks (0-511, 460-971, 920-1023) with correct overlap.
    - (c) `chunk_text` with empty string returns empty Vec.
    - (d) `is_model_downloaded` returns false when model dir is empty, true when files exist.
    - (e) `get_embedding_status` returns accurate counts matching database state.

**Acceptance Criteria:**
- Text over 512 tokens is correctly chunked with 10% overlap
- Short text is not unnecessarily chunked
- Model downloads on first use without blocking app startup
- `get_embedding_status` accurately reports pipeline progress and model readiness
- Embedding worker correctly handles multi-chunk captures

**Verification Steps:**
1. Delete `~/.cortex/models/embed/` and start the app -- model should download automatically
2. Insert a capture with very long OCR text (>1000 tokens), verify multiple chunks are created in `vec_captures`
3. Call `get_embedding_status` and verify counts match database

**Verification Commands:**
```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib -- chunker
cargo test --manifest-path src-tauri/Cargo.toml --lib -- model_manager

# Verify model files exist after first run
ls -la ~/.cortex/models/embed/all-MiniLM-L6-v2/

# Check chunk count for a capture
sqlite3 ~/.cortex/cortex.db "SELECT capture_id, COUNT(*) as chunks FROM vec_captures GROUP BY capture_id ORDER BY chunks DESC LIMIT 10;"

# Verify embedding status command
sqlite3 ~/.cortex/cortex.db "SELECT embedding_status, COUNT(*) FROM captures GROUP BY embedding_status;"
```

---

### Testing & Integration

#### Task Group 4: Test Review, Gap Analysis, Integration Verification
**Assigned implementer:** testing-engineer
**Dependencies:** Task Groups 1, 2, 3

- [ ] 4.0 Complete test coverage review and fill gaps with integration tests
  - [ ] 4.1 Review all tests from Groups 1-3 (14 total). Verify they compile and pass. Document any that are environment-dependent (e.g., tests requiring model files to be downloaded, tests requiring sqlite-vec extension).
  - [ ] 4.2 Integration test: full embedding pipeline -- Insert a capture, mark OCR as completed with known text, run the embedding worker for one cycle, call `search_captures` in semantic mode with a related query. Assert: (a) search returns at least one result, (b) result is the correct capture, (c) `embedding_status` is `completed`.
  - [ ] 4.3 Integration test: semantic vs keyword gap -- Insert two captures: one with text "BigCommerce API webhooks integration" and one with "cooking pasta carbonara recipe". Search with keyword mode for "e-commerce project" -- expect zero results. Search with semantic mode for "e-commerce project" -- expect the BigCommerce capture to rank first.
  - [ ] 4.4 Integration test: search latency -- Insert 10,000 synthetic embeddings (random 384-dim vectors) into `vec_captures`. Run semantic search 10 times with different query vectors. Assert average latency under 500ms.
  - [ ] 4.5 Integration test: chunking correctness -- Insert a capture with very long OCR text (2000+ tokens). Run embedding worker. Assert: (a) `vec_captures` contains multiple rows for the capture with sequential `chunk_index`, (b) querying with text related to content in a later chunk still returns the capture.
  - [ ] 4.6 Integration test: worker pipeline chaining -- Insert a capture with `ocr_status = 'pending'`. Simulate OCR completion (set `ocr_status = 'completed'`). Verify embedding worker picks it up and processes it. Assert the full chain: pending OCR -> completed OCR -> pending embedding -> completed embedding.
  - [ ] 4.7 Integration test: migration safety -- Create a v3 database with captures and transcriptions. Run migration to v4. Assert: (a) all existing rows preserved, (b) `embedding_status = 'pending'` on all rows, (c) `vec_captures` table exists, (d) `schema_version = 4`.
  - [ ] 4.8 Gap analysis -- Document untested paths: concurrent embedding + OCR under heavy CPU load, very large text blobs (>10,000 tokens), model file corruption, sqlite-vec extension load failure, disk full during vector insert, Metal vs CPU inference fallback, model download interrupted mid-file. File as future test TODOs in a comment block.

**Acceptance Criteria:**
- All tests from Groups 1-3 pass
- 6 new integration tests added and passing
- Gap analysis identifies at least 5 untested edge cases
- Total test count: 20+ (14 from Groups 1-3 + 6 new)
- Search latency confirmed under 500ms for 10,000 vectors
- Semantic search demonstrably finds results that keyword search misses

**Verification Steps:**
1. Run full test suite -- expect all tests pass
2. Review test output for flaky tests or warnings
3. Verify the "concept bridge" scenario works: semantic search finds results keyword search cannot

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

# Verify the concept bridge scenario manually
sqlite3 ~/.cortex/cortex.db "SELECT c.id, c.app_name, c.window_title FROM captures c JOIN vec_captures v ON c.id = v.capture_id LIMIT 10;"
```

---

## Execution Order

1. **Task Group 1: Embedding + Vector Search End-to-End** (ml-engineer + database-engineer) -- Proof of Life vertical slice. Must complete first. Delivers: "Insert text, generate embedding, store in sqlite-vec, query with different text, get semantically similar result back."
2. **Task Group 2: Background Embedding Worker** (systems-engineer) -- Depends on Group 1 for embedding module and schema.
3. **Task Group 3: Chunking & Model Management** (ml-engineer) -- Depends on Groups 1 and 2. Can partially overlap with Group 2: model download (3.3-3.4) can start while worker is being built, chunking integration (3.1-3.2) requires the worker.
4. **Task Group 4: Test Review & Integration** (testing-engineer) -- Depends on all prior groups completing.

**Parallel execution possible:** Groups 2 and 3 can partially overlap after Group 1 completes -- model download work (3.3-3.4) only depends on Group 1, while chunking integration (3.2) depends on Group 2's worker.
