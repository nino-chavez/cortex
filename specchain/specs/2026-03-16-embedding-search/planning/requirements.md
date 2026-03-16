# Spec Requirements: Embedding & Semantic Search

## Initial Description
Embedding & Semantic Search — Generate vector embeddings for all captured text (OCR + transcriptions) using nomic-embed-text via MLX. Store embeddings in sqlite-vec. Implement semantic similarity search so users can query with natural language instead of exact keywords.

## Requirements Discussion

### First Round Questions

**Q1: Proof of Life**
**Answer:** Search for "that project about e-commerce logs." The system returns a screenshot where OCR text contains "BigCommerce API" or "Webhooks" even though "project" or "e-commerce" never appeared on screen.

**Q2: Value Signal**
**Answer:** "Concept Bridge" — user searches "meeting about the move" and finds transcription where actual text was "relocating to the Austin office." Metric: semantic hit surfaced when keyword search returns zero results.

**Q3: Embedding Model**
**Answer:** Use the `candle` crate (Hugging Face Rust ML framework). Zero Python dependencies, runs GGUF-quantized models directly in Rust. Metal support on Apple Silicon. Models: nomic-embed-text or all-MiniLM-L6-v2.

**Q4: sqlite-vec**
**Answer:** Confirmed. Brute-force L2/cosine similarity. Under 100k vectors is lightning fast. Keeps DB portable, implementation simple.

**Q5: Embedding Scope & Chunking**
**Answer:** One embedding per capture (OCR text). 512-token sliding window with 10% overlap for text-heavy screens. Transcriptions already chunked at 30s — embed each segment individually.

**Q6: Search API**
**Answer:** Start with toggle: `search_captures(query, mode: "keyword" | "semantic")`. Build backend to support future Hybrid Search with RRF (Reciprocal Rank Fusion). Keep simple sorting for now.

**Q7: Background Processing**
**Answer:** Follow worker pattern. Trigger: once ocr_status == COMPLETED. Backfill on first launch for all existing completed OCR/transcriptions.

**Q8: Out of Scope**
**Answer:** RAG/Chat (spec #7), multi-modal image embeddings, hybrid ranking/RRF, fine-tuning.

### Existing Code to Reference
- **storage.rs** — Migration system for schema v4, worker pattern
- **ocr_worker.rs** — Background processing pattern to mirror
- **search.rs** — Extend with semantic search mode
- **sqlite-vec** — Loads as runtime extension into existing rusqlite connection

## Requirements Summary

### Functional Requirements
- Generate vector embeddings for OCR text and transcription text via candle
- Store embeddings in sqlite-vec virtual table
- Semantic similarity search via cosine distance
- Background embedding worker triggered by ocr_status == COMPLETED
- Backfill existing completed captures/transcriptions
- Search mode toggle: keyword (FTS5) or semantic (vector)
- 512-token chunking with 10% overlap for long OCR text
- Download embedding model on first use

### Schema (Migration v4)
- `embeddings` virtual table via sqlite-vec: capture_id/transcription_id, vector blob
- `embedding_status` column on captures and transcriptions tables
- Index linking embeddings to source records

### Tauri Commands
- Extend `search_captures` with `mode` parameter ("keyword" | "semantic")
- `get_embedding_status()` — counts of pending/completed/failed

### Technical Stack
- `candle-core`, `candle-transformers`, `candle-nn` — Rust ML inference
- `sqlite-vec` — SQLite extension for vector search (loaded at runtime)
- Model: all-MiniLM-L6-v2 (384-dim, ~80MB) or nomic-embed-text (768-dim, ~275MB)
- Start with all-MiniLM-L6-v2 for smaller size and faster inference

### Scope Boundaries
**In Scope:**
- Text embedding via candle
- sqlite-vec vector storage and search
- Background embedding worker
- Semantic search Tauri command
- Model download on first use
- Backfill for existing data

**Out of Scope:**
- RAG/Chat (spec #7)
- Multi-modal image embeddings
- Hybrid ranking (RRF)
- Fine-tuning
- Real-time streaming embeddings
