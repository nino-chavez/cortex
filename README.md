# Cortex

Local-first Mac app that silently captures your screen and audio, extracts text via OCR and transcription, and lets you search and chat with your entire digital history using private on-device AI.

**No cloud. No telemetry. No compromises.**

## What It Does

Cortex runs in the background on your Mac, capturing screenshots and audio at configurable intervals. Everything is processed locally:

- **Screen Capture** - Periodic screenshots via ScreenCaptureKit with app/window metadata
- **OCR** - Apple Vision extracts all visible text from every capture
- **Audio Transcription** - whisper.cpp transcribes system audio and microphone with Metal acceleration
- **Semantic Search** - Vector embeddings let you search by concept, not just keywords
- **AI Chat** - Ask questions about your history via local LLM (Ollama) with source citations
- **Meeting Memory** - Dedicated meeting mode with higher-frequency capture and auto-summarization
- **Smart Summaries** - AI summaries by time period, application, or topic
- **Clipboard History** - Indexed clipboard content integrated into search

Your data lives at `~/.cortex/` and never leaves your machine.

## Architecture

```
SvelteKit UI (Tauri WebView)
  - Search overlay (Cmd+Shift+Space)
  - Timeline view with filmstrip
  - AI Chat with citation badges
  - Settings & storage management
        |
  Tauri IPC / Commands
        |
Rust Backend (13 modules)
  - ScreenCaptureKit capture loop
  - Apple Vision OCR (swift-rs bridge)
  - whisper.cpp transcription (Metal GPU)
  - fastembed sentence embeddings (ONNX Runtime)
  - Ollama RAG pipeline
  - Meeting mode with grouped captures
  - Clipboard watcher (NSPasteboard)
  - SQLite + FTS5 + sqlite-vec
        |
~/.cortex/
  - cortex.db (metadata + FTS5 + vectors)
  - config.toml (user preferences)
  - screenshots/ (WebP)
  - audio/ (WAV/Opus)
  - models/ (whisper, embeddings)
```

## Stack

| Layer | Technology |
|---|---|
| Desktop | Tauri v2 |
| Frontend | SvelteKit / Svelte 5 (runes) / TypeScript / Tailwind CSS v4 |
| Backend | Rust |
| Screen capture | screencapturekit (Rust crate) |
| OCR | Apple Vision via swift-rs |
| Transcription | whisper-rs (whisper.cpp with Metal) |
| Embeddings | fastembed (all-MiniLM-L6-v2 via ONNX Runtime) |
| Vector search | sqlite-vec |
| Full-text search | SQLite FTS5 |
| LLM | Ollama (localhost API) |
| Database | SQLite via rusqlite |
| Config | TOML (via toml crate) |

## Prerequisites

- macOS 13.0+ (Ventura or later)
- Apple Silicon (M1/M2/M3/M4)
- [Rust](https://rustup.rs/) (stable)
- [Node.js](https://nodejs.org/) 18+
- [CMake](https://cmake.org/) (`brew install cmake`)
- [Ollama](https://ollama.ai/) (for AI chat and summary features)

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
4. For AI chat and summaries, install Ollama and pull a model:
   ```bash
   # Install Ollama from https://ollama.ai
   ollama pull llama3.1
   ```

### Global Hotkey

Press **Cmd+Shift+Space** from any application to open the search overlay.

## Features

### Screen Capture
Background daemon captures screenshots every 5 seconds (configurable 1-60s) with change detection to skip duplicate frames. Detects active app name, bundle ID, and window title. WebP compression at 80% quality keeps storage under 160MB for 8 hours.

### OCR Pipeline
Background worker processes each screenshot through Apple Vision (via swift-rs FFI bridge) and indexes extracted text in SQLite FTS5 for instant full-text search with snippet highlighting.

### Audio Transcription
Captures system audio and microphone as separate streams. Transcribes 30-second chunks via whisper.cpp with Metal GPU acceleration. Stored in FTS5 for unified search alongside OCR results.

### Semantic Search
fastembed generates 384-dimensional sentence embeddings stored in sqlite-vec. Search by concept ("that meeting about the budget") even when exact keywords don't match.

### Search UI
Raycast-style floating overlay (Cmd+Shift+Space). Hybrid keyword + semantic search with app and time filters. Result cards show screenshot thumbnails, app context, timestamps, and source badges (OCR/Audio).

### Timeline View
Full-window filmstrip UI with smart downsampling (1 thumbnail per minute, expandable to 5s granularity). Stage component shows full-resolution screenshot with OCR text panel. Date picker and keyboard navigation.

### AI Chat
RAG-powered chat via Ollama. Embeds your query, retrieves relevant context from the vector DB, and generates grounded answers with citation badges linking to specific captures in the Timeline.

### Meeting Memory
"Start Meeting" tray toggle increases capture frequency to 2 seconds and enables audio recording. On "End Meeting," generates an AI summary of the meeting transcription via Ollama.

### Smart Summaries
On-demand AI summaries via three commands: by time period ("summarize my morning"), by application ("what did I do in VS Code today"), or by topic ("everything about the auth refactor").

### Clipboard History
Monitors the macOS clipboard for changes every second. Indexes text and URL content in FTS5 for searchability alongside screen captures and transcriptions.

### Settings & Preferences
Configuration UI for capture interval, excluded apps (by bundle ID), audio source toggles, retention policies (screenshots: 30 days, audio: 7 days, text: forever), storage dashboard, and cleanup.

## Project Structure

```
src/                          # SvelteKit frontend
  routes/
    +page.svelte              # Main page with nav
    search/+page.svelte       # Search overlay route
    timeline/+page.svelte     # Timeline view
    chat/+page.svelte         # AI chat interface
    settings/+page.svelte     # Settings & preferences
  lib/components/
    search/                   # SearchOverlay, SearchInput, ResultCard
    timeline/                 # Stage, Filmstrip

src-tauri/                    # Rust backend
  src/
    lib.rs                    # Tauri app entry, 22 commands, state management
    capture.rs                # ScreenCaptureKit capture loop + WebP encoding
    accessibility.rs          # App/window metadata via NSWorkspace
    storage.rs                # SQLite DB, 6 schema migrations, all queries
    ocr.rs                    # Apple Vision FFI (swift-rs bridge)
    ocr_worker.rs             # Background OCR processing with retry
    audio.rs                  # Whisper transcription, WAV save, audio worker
    embedding.rs              # fastembed EmbeddingEngine wrapper
    search.rs                 # FTS5 + semantic search, unified UNION queries
    chat.rs                   # Ollama RAG pipeline, prompt construction
    meeting.rs                # Meeting mode lifecycle, grouped captures
    summary.rs                # AI summaries (period, app, topic)
    clipboard.rs              # NSPasteboard watcher, clipboard storage
    config.rs                 # TOML config, retention cleanup, storage stats
    permissions.rs            # macOS Screen Recording + Accessibility checks
    tray.rs                   # System tray menu with capture/meeting toggles
  swift-lib/                  # Swift package for Vision OCR
    Package.swift
    Sources/SwiftLib/ocr.swift
  build.rs                    # Swift linker + tauri_build

specchain/                    # Spec-driven development (SpecChain)
  product/                    # Mission, roadmap, tech stack
  specs/                      # 12 feature specifications
```

## Database Schema

Single SQLite database at `~/.cortex/cortex.db` with automatic migrations:

| Version | What It Adds |
|---|---|
| v1 | `captures` table, `schema_version` |
| v2 | `captures_fts` (FTS5), `ocr_status`/`ocr_retries` columns |
| v3 | `transcriptions`, `transcriptions_fts` (FTS5) |
| v4 | `vec_captures`/`vec_transcriptions` (sqlite-vec), `embedding_status` |
| v5 | `meetings` table, `meeting_id` columns on captures/transcriptions |
| v6 | `clipboard_entries`, `clipboard_fts` (FTS5) |

## Configuration

User preferences stored at `~/.cortex/config.toml`:

```toml
[general]
capture_interval_secs = 5
hotkey = "CommandOrControl+Shift+Space"

[retention]
screenshots_days = 30
audio_days = 7
keep_text_forever = true

[privacy]
excluded_apps = []

[audio]
system_audio_enabled = false
microphone_enabled = false
```

## Data Storage

```
~/.cortex/
  cortex.db           # SQLite database (metadata + FTS5 + vectors)
  config.toml         # User preferences
  screenshots/        # WebP captures (~250KB each)
    2026/03/16/       # Organized by date
  audio/              # Audio chunks (WAV)
    pending/          # Awaiting transcription
    2026/03/16/       # Processed and dated
  models/             # ML models
    whisper/          # whisper base.en (~148MB)
    embed/            # all-MiniLM-L6-v2 (~80MB)
```

**Storage estimate:** ~160MB for 8 hours of continuous screen capture at 5-second intervals.

## Development

```bash
# Install dependencies
npm install

# Frontend dev server only (port 4173)
npm run dev -- --port 4173

# Rust type checking
cargo check --manifest-path src-tauri/Cargo.toml

# Run all 33 tests
cargo test --manifest-path src-tauri/Cargo.toml --lib

# Build frontend
npm run build

# Full app dev mode
npx tauri dev

# Production build
npx tauri build
```

## Tests

33 unit tests covering:

- SQLite migrations v1-v6 and CRUD operations
- FTS5 full-text search with snippets, filters, and UNION queries
- Unified search across OCR + transcriptions + clipboard
- Change detection (xxhash pixel comparison)
- WebP encoding with RIFF header validation
- OCR worker status transitions and retry logic (3 attempts)
- Embedding dimension (384) and semantic similarity ranking
- RAG prompt construction with context and citations
- Meeting lifecycle (start/end, interval override, DB persistence)
- Clipboard entry storage and retrieval
- Config TOML serialization roundtrip
- Permission status and storage stats serialization

Run with: `cargo test --manifest-path src-tauri/Cargo.toml --lib`

## Roadmap

- [x] Capture Daemon
- [x] OCR Pipeline
- [x] Audio Capture & Transcription
- [x] Embedding & Semantic Search
- [x] Search UI
- [x] Timeline View
- [x] Local AI Chat
- [x] Meeting Memory
- [x] Smart Summaries
- [x] Clipboard History
- [x] Retention & Storage Management
- [x] Settings & Preferences

See `specchain/product/roadmap.md` for detailed specs.

## Inspiration

- [Screenpipe](https://github.com/screenpipe/screenpipe) - Architecture reference for Tauri + Rust + ScreenCaptureKit
- [OpenRewind](https://github.com/nicokimmel/open-rewind) - Community approach to local screen indexing
- [Rewind.ai](https://rewind.ai) / [Limitless](https://limitless.ai) - Pioneered searchable screen history (cloud-based)
- [Raycast](https://raycast.com) - UX patterns for the floating search overlay

## Privacy

Cortex is designed with privacy as a core architectural constraint, not a feature toggle:

- **Zero network calls** - Works identically with Wi-Fi disabled (except Ollama, which is also local)
- **No accounts** - No sign-up, no login, no telemetry
- **No cloud** - All processing runs on-device via Apple Silicon
- **Your data, your control** - Everything is in `~/.cortex/`. Delete the folder and it's gone.
- **Excluded apps** - Configure bundle IDs that should never be captured (e.g., password managers)
- **Open source** - Audit the code yourself

## License

MIT
