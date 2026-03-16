# Product Mission

## Pitch

Cortex is a local-first Mac app that silently captures your screen, audio, and meetings, extracts text and transcriptions, and lets you search and chat with your entire digital history using a private on-device AI — no cloud, no telemetry, no compromises.

## Users

### Primary Customers

- **Solo developer / knowledge worker**: Someone who spends 8+ hours/day across dozens of apps, tabs, conversations, and documents and needs to recall context from hours, days, or weeks ago.

### User Personas

**Nino** (30s)
- **Role:** Software engineer and organization operator
- **Context:** Juggles multiple codebases, design tools, communication channels, and administrative work daily. Frequently needs to recall "that thing I saw earlier" — a Slack message, a terminal output, a Figma comment, a meeting discussion.
- **Pain Points:** Context loss when switching between projects. Spending minutes searching through chat history, browser tabs, and notes for something seen hours ago. Meeting discussions that evaporate after the call ends.
- **Goals:** Instant recall of anything seen or heard on the machine. Semantic search ("the discussion about database migration") not just keyword search. Complete privacy — this data never leaves the machine.

## The Problem

### Digital Amnesia

Knowledge workers produce and consume massive amounts of information daily across dozens of applications. Most of this context is ephemeral — it scrolls past, gets buried in chat history, or lives in a tab that gets closed. Reconstructing context from even a few hours ago requires manual archaeology across multiple apps.

Existing solutions (Rewind, Limitless, Littlebird) capture this context but route it through cloud infrastructure, creating a complete record of your digital life on someone else's servers.

**Our Solution:** Capture everything locally. Index everything locally. Search and chat with everything locally. The entire pipeline — screen capture, OCR, transcription, embeddings, and LLM inference — runs on-device using Apple Silicon, with zero network dependency.

## Differentiators

### Absolute Privacy by Architecture

Unlike cloud-based alternatives (Rewind, Limitless), Cortex has no network component whatsoever. No accounts, no sync, no telemetry. The app functions identically with Wi-Fi disabled. Your second brain is a folder on your Mac — delete it and it's gone.

### Apple Silicon Native Performance

Unlike Electron-based alternatives, Cortex uses Tauri (native WebView) for the UI and MLX (Apple's ML framework) for all inference. This means lower memory usage, better battery life, and faster inference than any cross-platform alternative.

### Single-File Storage

Unlike tools that scatter data across multiple databases and file formats, Cortex stores everything — metadata, full-text index, and vector embeddings — in a single SQLite database with sqlite-vec. Simple to back up, simple to delete, simple to understand.

## Key Features

### Core Features

- **Continuous Screen Capture:** Background daemon captures screenshots at configurable intervals using macOS ScreenCaptureKit, with intelligent change detection to avoid redundant captures
- **Local OCR:** Apple Vision framework extracts all visible text from every screenshot, indexed for instant full-text search
- **Audio Transcription:** Whisper (via MLX) transcribes system audio and microphone input, capturing meetings, calls, and media with speaker diarization
- **Semantic Search:** Vector embeddings (nomic-embed-text) enable natural language queries like "the Slack conversation about API rate limits" across your entire history
- **Local AI Chat:** RAG-powered conversational interface using a local LLM (Llama 3.1 8B via MLX) that can answer questions grounded in your captured context

### Context Features

- **App-Aware Indexing:** Each capture is tagged with the active application, window title, and URL (for browsers), enabling filtered searches like "show me everything from Figma last Tuesday"
- **Timeline View:** Visual timeline of your day, browsable by time, application, or content type
- **Meeting Memory:** Dedicated meeting mode that pairs audio transcription with screen captures, creating a searchable record of every meeting with both what was said and what was shown

### Advanced Features

- **Smart Summaries:** On-demand AI summaries of time periods ("summarize my morning"), applications ("what did I do in VS Code today"), or topics ("everything related to the auth refactor this week")
- **Clipboard History:** Captures and indexes clipboard content alongside screen captures
- **Hotkey Recall:** Global keyboard shortcut to instantly search your history from any application, with results appearing in a floating overlay
- **Retention Policies:** Configurable auto-cleanup rules — keep last 30 days of screenshots but keep OCR text and transcriptions indefinitely, or define custom policies per content type
- **Export & Backup:** Export your entire database or filtered subsets as structured data (JSON/SQLite) for portability or archival
