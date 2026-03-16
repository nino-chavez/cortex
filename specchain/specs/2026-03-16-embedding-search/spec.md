# Specification: Embedding & Semantic Search

## Goal

Generate vector embeddings for all captured text (OCR + transcriptions) using fastembed (all-MiniLM-L6-v2 via ONNX Runtime), store them in sqlite-vec virtual tables, and implement semantic similarity search so users can query with natural language instead of exact keywords -- bridging the gap between what the user remembers and the words that actually appeared on screen.

## Proof of Life

**Scenario:** User searches "that project about e-commerce logs." The system returns a screenshot where OCR text contains "BigCommerce API" or "Webhooks" even though "project" or "e-commerce" never appeared on screen.

**Validates:** The full embedding pipeline works end-to-end -- candle generates vectors, sqlite-vec stores and queries them, and cosine similarity surfaces semantically related results that keyword search would miss.

**Must work before:** RAG/Chat (spec #7) and hybrid search ranking can begin.

## User Stories

- As a user, I want to search using natural language and find captures where the exact words I used never appeared, so I can find things by concept rather than verbatim keywords.
- As a user, I want to toggle between keyword and semantic search so I can use exact match when I know the term and fuzzy match when I don't.
- As a user, I want embeddings generated automatically in the background after OCR completes so my captures become semantically searchable without manual action.
- As a user, I want existing OCR-completed captures to be backfilled with embeddings on first launch so my full history is semantically searchable.
- As a user, I want to check embedding pipeline status so I know how many captures are still being processed.

## Core Requirements

### Functional Requirements

- **Embedding generation:** Use `candle-core`, `candle-nn`, and `candle-transformers` crates to load all-MiniLM-L6-v2 (384-dimensional, ~80MB) and generate embeddings from text. No Python or Swift bridge required.
- **Vector storage:** Load `sqlite-vec` as a runtime extension into the existing rusqlite `Connection`. Create `vec_captures` and `vec_transcriptions` virtual tables storing 384-dim float32 vectors.
- **Semantic search:** Query sqlite-vec using cosine distance. Accept a natural language query string, embed it, and return captures ranked by similarity. Limit to top 50 results.
- **Search mode toggle:** Extend `search_captures` with a `mode` parameter: `"keyword"` (existing FTS5 behavior) or `"semantic"` (cosine similarity via sqlite-vec).
- **Background embedding worker:** A dedicated thread polls for captures/transcriptions with `ocr_status = 'completed'` but `embedding_status = 'pending'`. Follows the same pattern as `ocr_worker.rs`.
- **Chunking:** 512-token sliding window with 10% overlap (51 tokens) for long OCR text. Each chunk gets its own embedding row linked to the source capture. Transcription segments (already ~30s chunks) are embedded individually without further splitting.
- **Model management:** Download all-MiniLM-L6-v2 model files on first use to `~/.cortex/models/embed/`. Report download progress. No inference until model is fully downloaded.
- **Status tracking:** Add `embedding_status` column to `captures` and `transcriptions` tables. States: `pending`, `processing`, `completed`, `failed`.
- **Backfill:** On worker startup, all records with `ocr_status = 'completed'` and `embedding_status = 'pending'` are eligible for processing. Newest first.
- **Retry logic:** After 3 failed attempts, mark as `failed` and skip. Never block the queue on a single bad record.
- **Status command:** `get_embedding_status()` returns counts of pending, completed, and failed for both captures and transcriptions.

### Non-Functional Requirements

- Embedding generation should process at least 10 captures per second on Apple Silicon (M1+).
- Semantic search latency under 500ms for up to 100,000 vectors.
- Embedding worker must not block the capture loop or OCR worker -- runs on its own thread.
- Worker processes newest captures first.
- Worker sleeps 5 seconds between polls when no pending work exists.
- Schema migration must be non-destructive -- existing v3 databases upgrade cleanly to v4.
- Model download must not block app startup. Worker waits for download to complete before processing.

## Visual Design

No UI in this spec. All interaction is through Tauri commands and the debug console. Search UI is a separate spec.

## Conversion Design

Not applicable -- this is a backend pipeline with no user-facing interface.

## Reusable Components

### Existing Code to Leverage

- **`storage.rs`** -- `Database` struct with `Mutex<Connection>`, migration chain (currently at v3). Migration v4 extends the existing chain.
- **`ocr_worker.rs`** -- Background worker pattern with polling loop, batch processing, retry logic, and stop flag. The embedding worker mirrors this structure exactly.
- **`search.rs`** -- `search_captures` function with FTS5 MATCH queries, `SearchResult` struct. Extended with `mode` parameter; semantic branch added alongside existing keyword branch.
- **`Cargo.toml`** -- `rusqlite` with `bundled` feature already included. Add candle crates and sqlite-vec.

### New Components Required

- **`embedding.rs`** -- Embedding module. Loads the all-MiniLM-L6-v2 model via candle. Exposes `pub fn embed_text(text: &str) -> Result<Vec<f32>>` that returns a 384-dim vector. Handles tokenization and inference.
- **`embedding_worker.rs`** -- Background worker. Polls for records needing embeddings, generates them in batches, inserts into sqlite-vec. Mirrors `ocr_worker.rs` structure.
- **`chunker.rs`** -- Text chunking module. Implements 512-token sliding window with 10% overlap. Returns `Vec<String>` of chunks from input text.
- **`model_manager.rs`** -- Model download and cache management. Downloads model from Hugging Face Hub to `~/.cortex/models/embed/` on first use. Reports progress via callback or channel.

## Technical Approach

### Database

- **Migration v3 to v4:**
  1. Load `sqlite-vec` extension via `conn.load_extension("vec0", None)` (or load the compiled `.dylib`).
  2. `ALTER TABLE captures ADD COLUMN embedding_status TEXT NOT NULL DEFAULT 'pending'`.
  3. `ALTER TABLE captures ADD COLUMN embedding_retries INTEGER NOT NULL DEFAULT 0`.
  4. `ALTER TABLE transcriptions ADD COLUMN embedding_status TEXT NOT NULL DEFAULT 'pending'`.
  5. `ALTER TABLE transcriptions ADD COLUMN embedding_retries INTEGER NOT NULL DEFAULT 0`.
  6. `CREATE INDEX idx_captures_embedding_status ON captures(embedding_status)`.
  7. `CREATE INDEX idx_transcriptions_embedding_status ON transcriptions(embedding_status)`.
  8. `CREATE VIRTUAL TABLE vec_captures USING vec0(capture_id INTEGER NOT NULL, chunk_index INTEGER NOT NULL, embedding float[384])`.
  9. `CREATE VIRTUAL TABLE vec_transcriptions USING vec0(transcription_id INTEGER NOT NULL, chunk_index INTEGER NOT NULL, embedding float[384])`.
- Bump `CURRENT_SCHEMA_VERSION` from 3 to 4.
- Set `embedding_status = 'pending'` for all existing records where `ocr_status = 'completed'` (backfill trigger).

### API (Tauri Commands)

- `search_captures(query, app_filter, time_from, time_to, mode)` -- When `mode = "keyword"`, existing FTS5 behavior. When `mode = "semantic"`, embed the query text, query `vec_captures` for nearest neighbors by cosine distance, join back to `captures` for metadata. Apply app/time filters as WHERE clauses on the join.
- `get_embedding_status()` -- Returns `{ captures: { pending, completed, failed }, transcriptions: { pending, completed, failed } }`.

### Embedding Pipeline

- Load model once on worker startup. Keep in memory for the worker's lifetime.
- For each capture: retrieve OCR text from `captures_fts`, run through chunker (512-token window, 10% overlap), embed each chunk, insert all chunk embeddings into `vec_captures` with `(capture_id, chunk_index, embedding)`.
- For each transcription: retrieve text from `transcriptions`, embed directly (already chunked at ~30s), insert into `vec_transcriptions`.
- Batch processing: process up to 10 records per poll cycle.

### Semantic Search Query

- Embed the user's query string using the same model.
- Query: `SELECT capture_id, distance FROM vec_captures WHERE embedding MATCH ? ORDER BY distance LIMIT 50`.
- Join results back to `captures` table for metadata (timestamp, app_name, image_path).
- Return as `Vec<SearchResult>` with `result_type = "semantic"`.

### Model Management

- Model path: `~/.cortex/models/embed/all-MiniLM-L6-v2/`.
- Required files: `config.json`, `tokenizer.json`, `model.safetensors`.
- Download from Hugging Face Hub (`sentence-transformers/all-MiniLM-L6-v2`) on first use.
- Worker blocks until model is available. Does not retry download on failure -- user must trigger retry.

### Testing

- **Unit tests for embedding:** Generate embedding for known text, verify it returns a 384-dim vector with valid float values.
- **Unit tests for chunking:** Verify 512-token window with 10% overlap produces correct chunk boundaries and overlap.
- **Unit tests for sqlite-vec:** Insert a vector, query with cosine distance, verify the correct row is returned.
- **Integration test for semantic search:** Insert two captures with different but semantically related text, query with a third phrase, verify the semantically closer result ranks higher.
- **Worker tests:** Mirror the pattern from `ocr_worker.rs` tests -- verify status transitions, skip logic, retry behavior.

## Out of Scope

- RAG/Chat interface (spec #7)
- Multi-modal image embeddings (embedding raw screenshots)
- Hybrid ranking with Reciprocal Rank Fusion (RRF)
- Fine-tuning the embedding model
- Real-time streaming embeddings
- Model selection UI (hardcoded to all-MiniLM-L6-v2 for now)
- Upgrading to nomic-embed-text (future enhancement once pipeline is proven)

## Success Criteria

- Searching "that project about e-commerce logs" returns a capture containing "BigCommerce API" text when keyword search returns zero results.
- Embedding generation processes at least 10 captures/second on Apple Silicon.
- Semantic search returns results in under 500ms with up to 100,000 stored vectors.
- Background worker processes new captures within 10 seconds of OCR completion.
- Schema migration from v3 to v4 preserves all existing data.
- Failed embedding attempts are retried up to 3 times, then marked `failed`.
- `get_embedding_status()` accurately reports pipeline progress.
