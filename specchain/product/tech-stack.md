# Tech Stack

## Application Shell

| Layer | Technology | Purpose |
|---|---|---|
| Desktop framework | **Tauri v2** | Native Mac app shell, system tray, IPC, auto-updater |
| Frontend | **SvelteKit** (SPA mode, SSR disabled) | Search UI, chat interface, timeline, settings |
| Frontend language | **TypeScript** | Type-safe frontend code |
| Styling | **Tailwind CSS v4** | Utility-first styling |
| Backend | **Rust** | Native macOS API access, capture daemon, inference orchestration |
| Internal API | **Tauri commands** + **Axum** (localhost HTTP) | IPC between frontend and backend |

## macOS Native APIs

| Capability | Technology | Notes |
|---|---|---|
| Screen capture | **screencapturekit-rs** (Rust crate) | Rust bindings for Apple ScreenCaptureKit |
| OCR | **Apple Vision framework** via **swift-rs** | Swift bridge from Rust to Vision's text recognition |
| Audio capture | **ScreenCaptureKit** (audio streams) | System audio + microphone capture |
| Accessibility | **macOS Accessibility API** | Active app name, window title, browser URL detection |
| Global hotkey | **Tauri global shortcut plugin** | System-wide keyboard shortcut for instant recall |

## AI / ML Pipeline

| Capability | Technology | Model | Notes |
|---|---|---|---|
| Chat / RAG | **MLX** (mlx-swift or llama.cpp via Rust FFI) | Llama 3.1 8B (Q4_K_M) | On-device conversational AI |
| Embeddings | **MLX** (mlx-embeddings) | nomic-embed-text-v1.5 | 768-dim vectors, Matryoshka support |
| Transcription | **MLX** (mlx-whisper) or **whisper.cpp** | whisper-large-v3 | Audio → text with timestamps |
| Dev fallback | **Ollama** | Same models | OpenAI-compatible API for faster dev iteration |

## Storage

| Layer | Technology | Notes |
|---|---|---|
| Database | **SQLite** (via rusqlite) | Single-file DB for all structured data |
| Vector search | **sqlite-vec** (SQLite extension) | Embedded vector similarity search |
| Full-text search | **SQLite FTS5** | Built-in full-text search |
| Screenshot storage | **Filesystem** (compressed PNG/WebP) | `~/.cortex/screenshots/` |
| Audio storage | **Filesystem** (MP4/Opus chunks) | `~/.cortex/audio/` |
| Model storage | **Filesystem** (GGUF/MLX weights) | `~/.cortex/models/` |

## Data Directory Structure

```
~/.cortex/
├── cortex.db          # SQLite: metadata + FTS5 + sqlite-vec embeddings
├── screenshots/       # Compressed screen captures (PNG/WebP)
│   └── 2026/03/15/    # Organized by date
├── audio/             # Audio chunks (Opus/MP4)
│   └── 2026/03/15/
├── models/            # Local ML models
│   ├── llm/           # Chat model (GGUF or MLX format)
│   ├── embed/         # Embedding model
│   └── whisper/       # Whisper model
└── config.toml        # User preferences
```

## Build & Development

| Tool | Purpose |
|---|---|
| **Cargo** | Rust dependency management and build |
| **npm** | Frontend dependency management |
| **Vite** | Frontend dev server and bundler (via SvelteKit) |
| **Tauri CLI** | App bundling, signing, notarization |

## Inspiration & References

### Open Source Projects

| Project | Influence | Link |
|---|---|---|
| **Screenpipe** | Architecture reference — validated Tauri + Rust + ScreenCaptureKit + SQLite stack for 24/7 screen capture. Their `screencapturekit-rs` crate and Vision OCR bridging patterns were directly referenced. | [github.com/screenpipe/screenpipe](https://github.com/screenpipe/screenpipe) |
| **OpenRewind** | Community Rewind.ai clone — studied their approach to local-first screen indexing and timeline UI patterns. | [github.com/nicokimmel/open-rewind](https://github.com/nicokimmel/open-rewind) |

### Commercial Products (Studied for UX Patterns)

| Product | What We Learned |
|---|---|
| **Rewind.ai** | Pioneered the "searchable screen history" concept. We studied their UX for timeline scrubbing and keyword search. Cortex differentiates by being 100% local with zero cloud dependency. |
| **Limitless (formerly Rewind)** | Meeting memory and AI summarization patterns. Informed our Meeting Memory spec (#8). |
| **Raycast** | The gold standard for macOS floating search overlays. Our Search UI (Cmd+Shift+Space) directly follows their interaction model. |

### Key Crates & Tools

| Tool | How We Used It |
|---|---|
| **screencapturekit** (Rust) | Core screen capture — ScreenCaptureKit bindings by svtlabs |
| **swift-rs** | FFI bridge for Apple Vision OCR (VNRecognizeTextRequest) |
| **whisper-rs** | whisper.cpp Rust bindings with Metal acceleration for audio transcription |
| **fastembed** | all-MiniLM-L6-v2 sentence embeddings via ONNX Runtime — chosen over candle for stability and simplicity |
| **sqlite-vec** | Vector similarity search as a SQLite extension — keeps everything in one database file |
| **rusqlite** | SQLite driver with bundled FTS5 for full-text search |
| **Ollama** | Local LLM inference server for RAG chat — avoids bundling model weights in the binary |
| **Tauri v2** | Desktop app framework — native WebView, system tray, global shortcuts |

## Key Crates (Rust)

- `tauri` — Desktop app framework
- `axum` — HTTP server (optional internal API)
- `rusqlite` — SQLite bindings
- `screencapturekit-rs` — macOS screen capture
- `swift-rs` — Swift interop for Vision framework
- `whisper-rs` — whisper.cpp Rust bindings
- `serde` / `serde_json` — Serialization
- `tokio` — Async runtime
- `chrono` — Date/time handling
