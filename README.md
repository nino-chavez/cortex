# Cortex

Local-first Mac app that silently captures your screen and audio, extracts text via OCR and transcription, and lets you search and chat with your entire digital history using private on-device AI.

**No cloud. No telemetry. No compromises.**

## What It Does

Cortex runs in the background on your Mac, capturing screenshots and audio at configurable intervals. Everything is processed locally:

- **Screen Capture** - Periodic screenshots via ScreenCaptureKit with app/window metadata
- **OCR** - Apple Vision extracts all visible text from every capture
- **Audio Transcription** - whisper.cpp transcribes system audio and microphone with Metal acceleration
- **Semantic Search** - Vector embeddings let you search by concept, not just keywords
- **AI Chat** - Ask questions about your history via local LLM (Ollama)

Your data lives at `~/.cortex/` and never leaves your machine.

## Architecture

```
SvelteKit UI (Tauri WebView)
  - Search overlay (Cmd+Shift+Space)
  - Timeline view
  - AI Chat with citations
        |
  Tauri IPC / Commands
        |
Rust Backend
  - ScreenCaptureKit capture loop
  - Apple Vision OCR (swift-rs bridge)
  - whisper.cpp transcription (Metal GPU)
  - fastembed sentence embeddings
  - Ollama RAG pipeline
  - SQLite + FTS5 + sqlite-vec
        |
~/.cortex/
  - cortex.db (metadata + FTS5 + vectors)
  - screenshots/ (WebP)
  - audio/ (WAV/Opus)
  - models/ (whisper, embeddings)
```

## Stack

| Layer | Technology |
|---|---|
| Desktop | Tauri v2 |
| Frontend | SvelteKit / Svelte 5 / TypeScript / Tailwind CSS v4 |
| Backend | Rust |
| Screen capture | screencapturekit (Rust crate) |
| OCR | Apple Vision via swift-rs |
| Transcription | whisper-rs (whisper.cpp with Metal) |
| Embeddings | fastembed (all-MiniLM-L6-v2 via ONNX Runtime) |
| Vector search | sqlite-vec |
| Full-text search | SQLite FTS5 |
| LLM | Ollama (localhost API) |
| Database | SQLite via rusqlite |

## Prerequisites

- macOS 13.0+ (Monterey or later)
- Apple Silicon (M1/M2/M3/M4)
- [Rust](https://rustup.rs/) (stable)
- [Node.js](https://nodejs.org/) 18+
- [CMake](https://cmake.org/) (`brew install cmake`)
- [Ollama](https://ollama.ai/) (for AI chat features)

## Getting Started

```bash
# Clone
git clone https://github.com/nino-chavez/cortex.git
cd cortex

# Install frontend dependencies
npm install

# Build and run in dev mode
npx tauri dev

# Or build for production
npx tauri build
```

### First Launch

1. Grant **Screen Recording** permission when prompted (System Settings > Privacy & Security)
2. Grant **Accessibility** permission for window title detection
3. Click the Cortex icon in the menu bar > **Start Capture**
4. For AI chat: install Ollama and pull a model:
   ```bash
   # Install Ollama from https://ollama.ai
   ollama pull llama3.1
   ```

### Global Hotkey

Press **Cmd+Shift+Space** from any application to open the search overlay.

## Project Structure

```
src/                          # SvelteKit frontend
  routes/
    +page.svelte              # Main page with nav
    search/+page.svelte       # Search overlay route
    timeline/+page.svelte     # Timeline view
    chat/+page.svelte         # AI chat interface
  lib/components/
    search/                   # Search overlay components
    timeline/                 # Timeline components

src-tauri/                    # Rust backend
  src/
    lib.rs                    # Tauri app entry + command registration
    capture.rs                # ScreenCaptureKit capture loop
    accessibility.rs          # App/window metadata via NSWorkspace
    storage.rs                # SQLite DB, migrations, queries
    ocr.rs                    # Apple Vision FFI bridge
    ocr_worker.rs             # Background OCR processing
    audio.rs                  # Whisper transcription + audio worker
    embedding.rs              # fastembed sentence embeddings
    search.rs                 # FTS5 + semantic search, unified results
    chat.rs                   # Ollama RAG pipeline
    permissions.rs            # macOS permission checks
    tray.rs                   # System tray setup
  swift-lib/                  # Swift package for Vision OCR
    Sources/SwiftLib/ocr.swift

specchain/                    # Spec-driven development workflow
  product/                    # Mission, roadmap, tech stack
  specs/                      # Feature specifications
```

## Database Schema

Cortex uses a single SQLite database at `~/.cortex/cortex.db` with automatic migrations:

| Version | Tables Added |
|---|---|
| v1 | `captures`, `schema_version` |
| v2 | `captures_fts` (FTS5), `ocr_status` + `ocr_retries` columns |
| v3 | `transcriptions`, `transcriptions_fts` (FTS5) |
| v4 | `vec_captures`, `vec_transcriptions` (sqlite-vec), `embedding_status` column |

## Data Storage

All data is stored locally at `~/.cortex/`:

```
~/.cortex/
  cortex.db           # SQLite database
  screenshots/        # WebP captures (~250KB each)
    2026/03/16/       # Organized by date
  audio/              # Audio chunks
  models/             # ML models (whisper, embeddings)
```

**Storage estimate:** ~160MB for 8 hours of continuous screen capture at 5-second intervals.

## Development

```bash
# Frontend dev server only
npm run dev

# Rust type checking
cargo check --manifest-path src-tauri/Cargo.toml

# Run tests (25 passing)
cargo test --manifest-path src-tauri/Cargo.toml --lib

# Build frontend
npm run build
```

## Tests

25 unit tests covering:

- SQLite migrations and CRUD operations
- FTS5 full-text search with snippets and filters
- Change detection (pixel hashing)
- WebP encoding
- OCR worker status transitions and retry logic
- Unified search across OCR + transcriptions
- Embedding dimension and semantic similarity
- RAG prompt construction
- Permission status serialization

Run with: `cargo test --manifest-path src-tauri/Cargo.toml --lib`

## Roadmap

- [x] Capture Daemon
- [x] OCR Pipeline
- [x] Audio Capture & Transcription
- [x] Embedding & Semantic Search
- [x] Search UI
- [x] Timeline View
- [x] Local AI Chat
- [ ] Meeting Memory
- [ ] Smart Summaries
- [ ] Clipboard History
- [ ] Retention & Storage Management
- [ ] Settings & Preferences

See `specchain/product/roadmap.md` for details.

## Inspiration

- [Screenpipe](https://github.com/screenpipe/screenpipe) - Architecture reference for Tauri + Rust + ScreenCaptureKit
- [Rewind.ai](https://rewind.ai) / [Limitless](https://limitless.ai) - Pioneered searchable screen history (cloud-based)
- [Raycast](https://raycast.com) - UX patterns for the floating search overlay

## Privacy

Cortex is designed with privacy as a core architectural constraint, not a feature toggle:

- **Zero network calls** - Works identically with Wi-Fi disabled
- **No accounts** - No sign-up, no login, no telemetry
- **No cloud** - All processing runs on-device (Apple Silicon)
- **Your data, your control** - Everything is in `~/.cortex/`. Delete the folder and it's gone.
- **Open source** - Audit the code yourself

## License

MIT
