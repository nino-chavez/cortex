# Product Roadmap

1. [x] **Capture Daemon** — Background Rust service that takes periodic screenshots via ScreenCaptureKit, detects the active app/window title, and stores captures as compressed images with metadata in SQLite. Runs from the system tray with start/stop/pause controls. `L`
2. [x] **OCR Pipeline** — Process each screenshot through Apple Vision framework (via swift-rs bridge) to extract all visible text. Store extracted text in SQLite with FTS5 full-text indexing. Support keyword search across captured text with app and time filters. `M`
3. [x] **Audio Capture & Transcription** — Capture system audio and microphone input via ScreenCaptureKit audio streams. Transcribe audio chunks using whisper.cpp/mlx-whisper with timestamps and speaker diarization. Store transcriptions in the same SQLite database with FTS5 indexing. `L`
4. [x] **Embedding & Semantic Search** — Generate vector embeddings for all captured text (OCR + transcriptions) using nomic-embed-text via MLX. Store embeddings in sqlite-vec. Implement semantic similarity search so users can query with natural language ("the conversation about database migration") instead of exact keywords. `M`
5. [x] **Search UI** — SvelteKit interface for searching captured history. Combined keyword + semantic search with filters (date range, application, content type). Results display the matched text, source screenshot thumbnail, timestamp, and app context. Global hotkey opens a floating search overlay from any application. `L`
6. [x] **Timeline View** — Visual, scrollable timeline of the user's day organized by time. Thumbnails of screenshots with OCR text overlays, interspersed with transcription segments. Filterable by application, content type, or time range. Click any moment to see the full capture and surrounding context. `M`
7. [x] **Local AI Chat** — RAG-powered chat interface using Llama 3.1 8B (via MLX). User asks questions in natural language, system retrieves relevant context from the vector DB, and the LLM generates grounded answers citing specific captures and transcriptions. Streaming responses with source attribution. `L`
8. [x] **Meeting Memory** — Dedicated meeting mode that pairs continuous audio transcription with higher-frequency screen captures. Creates a unified, searchable meeting record with both what was said and what was shown on screen. Post-meeting summary generation via the local LLM. `M`
9. [x] **Smart Summaries** — On-demand AI summaries of time periods ("summarize my morning"), applications ("what did I do in VS Code today"), or topics ("everything about the auth refactor this week"). Uses the local LLM with relevant context retrieved from the vector DB. `M`
10. [x] **Clipboard History** — Capture and index clipboard content (text, images, URLs) alongside screen captures. Searchable clipboard history integrated into the main search and timeline views. `S`
11. [x] **Retention & Storage Management** — Configurable retention policies per content type (e.g., keep screenshots 30 days but keep OCR text indefinitely). Storage usage dashboard. Manual and automatic cleanup. Export filtered subsets as JSON/SQLite for backup or portability. `M`
12. [x] **Settings & Preferences** — Configuration UI for capture interval, excluded apps, audio sources, hotkey bindings, model selection, retention policies, and storage location. First-run onboarding that requests necessary macOS permissions (Screen Recording, Microphone, Accessibility). `M`

> Notes
> - 12 items covering the full north star vision
> - Ordered by technical dependency: capture → indexing → search → AI → polish
> - Each item is end-to-end functional and testable
> - Effort: XS (1 day), S (2-3 days), M (1 week), L (2 weeks), XL (3+ weeks)
