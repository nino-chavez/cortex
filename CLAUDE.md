# Cortex

Local-first Mac app for continuous screen/audio capture with on-device AI search and chat.

## Stack

- **Desktop:** Tauri v2 (Rust backend, native WebView)
- **Frontend:** SvelteKit (SPA mode) / Svelte 5 (runes) / TypeScript / Tailwind CSS v4
- **Backend:** Rust
- **OCR:** Apple Vision via swift-rs (`src-tauri/swift-lib/`)
- **Transcription:** whisper-rs (whisper.cpp with Metal acceleration)
- **Embeddings:** fastembed (all-MiniLM-L6-v2 via ONNX Runtime)
- **Vector search:** sqlite-vec (SQLite extension)
- **Full-text search:** SQLite FTS5
- **LLM:** Ollama localhost API
- **Database:** SQLite via rusqlite (single file at `~/.cortex/cortex.db`)

## Commands

```bash
npm install                    # Frontend dependencies
npm run dev -- --port 4173     # SvelteKit dev server
npm run build                  # Build frontend to build/
npx tauri dev                  # Run full Tauri app in dev mode
npx tauri build                # Production build
cargo test --manifest-path src-tauri/Cargo.toml --lib  # Run 25 tests
cargo check --manifest-path src-tauri/Cargo.toml       # Type check Rust
```

## Prerequisites

- Rust (stable) via rustup
- Node.js 18+ / npm
- CMake (`brew install cmake`) — required by whisper-rs
- Xcode Command Line Tools — required by swift-rs and screencapturekit

## Project Structure

```
src/                     # SvelteKit frontend (SPA mode, SSR disabled)
  routes/                # Pages: /, /search, /timeline, /chat
  lib/components/        # Svelte 5 components (search/, timeline/)
  app.css                # Tailwind CSS v4 entry

src-tauri/               # Rust backend
  src/
    lib.rs               # App entry, Tauri command registration, state management
    capture.rs           # ScreenCaptureKit capture loop + WebP encoding
    accessibility.rs     # NSWorkspace app/window metadata
    storage.rs           # SQLite DB, schema migrations (v1-v4), all queries
    ocr.rs               # Apple Vision FFI (swift-rs bridge)
    ocr_worker.rs        # Background OCR processing with retry logic
    audio.rs             # Whisper transcription, WAV save, audio worker
    embedding.rs         # fastembed EmbeddingEngine wrapper
    search.rs            # FTS5 + semantic search, unified UNION queries
    chat.rs              # Ollama RAG pipeline, prompt construction
    permissions.rs       # macOS Screen Recording + Accessibility checks
    tray.rs              # System tray: capture toggle, permissions, quit
  swift-lib/             # Swift package for Vision OCR
    Package.swift
    Sources/SwiftLib/ocr.swift
  build.rs               # Swift linker + tauri_build
  Cargo.toml             # Rust dependencies

specchain/               # Spec-driven development (SpecChain)
  product/               # Mission, roadmap, tech stack
  specs/                 # Feature specifications (10 specs)
```

## Schema Migrations

Migrations run automatically on startup in `storage.rs`:

| Version | What It Adds |
|---|---|
| v1 | `captures` table, `schema_version` table |
| v2 | `captures_fts` (FTS5), `ocr_status`/`ocr_retries` columns on captures |
| v3 | `transcriptions` table, `transcriptions_fts` (FTS5) |
| v4 | `vec_captures`/`vec_transcriptions` (sqlite-vec), `embedding_status` column |

## Tauri Commands (IPC)

All backend functions exposed to the frontend:

| Command | Purpose |
|---|---|
| `start_capture` | Start the screen capture loop |
| `pause_capture` | Pause capture |
| `get_capture_status` | Current capture state (Recording/Paused/Error) |
| `get_recent_captures` | Last 20 captures |
| `search_captures` | FTS5 + semantic search with filters |
| `get_ocr_status` | Pending/completed/failed OCR counts |
| `get_captures_for_day` | All captures for a date (timeline) |
| `get_capture_by_id` | Single capture by ID |
| `get_capture_ocr_text` | OCR text for a capture |
| `get_distinct_apps` | Unique app names for filters |
| `chat_message` | RAG chat via Ollama |
| `check_ollama_status` | Ollama availability check |
| `check_permissions` | Screen Recording + Accessibility status |
| `set_capture_interval` | Change capture interval (1-60s) |

## Design Tokens (Signal X Dark)

| Token | Value |
|---|---|
| Background | `#0A0A0A` |
| Surface | `#141414` |
| Border | `#262626` |
| Text primary | `#FAFAFA` |
| Text secondary | `#A3A3A3` |
| Text muted | `#525252` |

## Conventions

- npm (not pnpm)
- Svelte 5 runes (`$state`, `$derived`, `$effect`)
- No emoji in code or commits
- Descriptive, conventional-ish commit messages
- Dev server on port 4173 (avoids conflict with other projects)
- All data at `~/.cortex/`
