# Specification: Audio Capture & Transcription

## Goal

Add system audio and microphone capture to Cortex, transcribe audio locally via whisper-rs, and make transcriptions searchable alongside OCR text -- enabling users to recall anything said or heard on their Mac.

## Proof of Life

User enables "Record System Audio" in the tray, plays a YouTube video for 30 seconds, then searches for a phrase from the video. The search returns a text snippet with timestamp and "System" source label. Repeat with "Record Microphone" enabled and spoken words -- results show "Mic" source label.

## User Stories

- As a knowledge worker, I want meetings automatically transcribed so I can search for discussion topics later without taking notes.
- As a developer, I want to search for something a colleague said in a call by keyword rather than scrubbing through a recording.
- As a user, I want separate toggles for system audio and microphone so I control exactly what is captured.
- As a user, I want audio files auto-deleted after 7 days but transcription text kept forever so storage stays manageable.

## Core Requirements

### Functional Requirements

1. **Audio capture** -- Capture system audio via ScreenCaptureKit `.with_captures_audio(true)` and microphone via `.with_captures_microphone(true)` as separate streams.
2. **Chunking** -- Split raw PCM audio into 30-second fixed chunks aligned to wall-clock time.
3. **Encoding** -- Encode each chunk as Opus and store in `~/.cortex/audio/YYYY/MM/DD/`.
4. **Transcription** -- Process each chunk through whisper-rs (whisper.cpp Rust bindings) producing timestamped text.
5. **Model management** -- Download whisper `base` model (~150MB) on first use to `~/.cortex/models/whisper/`. No bundling.
6. **Storage** -- Insert transcriptions into `transcriptions` table with FTS5 indexing via `transcriptions_fts`.
7. **Search** -- Extend `search_captures` to query both `captures_fts` and `transcriptions_fts`, returning unified results sorted by timestamp.
8. **Tray toggles** -- Three independent toggles in the tray menu: Record Screen, Record System Audio, Record Microphone.
9. **Retention** -- 7-day rolling deletion of audio files (configurable). Transcription text kept indefinitely.
10. **Capture linking** -- Nullable `capture_id` FK on transcriptions to link audio chunks to concurrent screen captures.

### Non-Functional Requirements

- CPU usage during transcription stays under 30% on M1 (process chunks sequentially, not in parallel).
- Audio capture adds no perceptible latency to screen capture.
- Microphone capture requires macOS 15.0+. System audio works on macOS 14.0+. Gracefully disable mic toggle on older OS.
- Model download shows progress in tray status text.
- No network calls except the one-time model download.

## Visual Design

No new UI windows. Changes are limited to:

- **Tray menu** -- Add "Record System Audio" and "Record Microphone" toggle items below the existing "Start Capture" toggle. Show a status line like "Audio: System + Mic" when active.
- **Search results** -- Transcription results display with a speaker/mic icon, source label ("System" or "Mic"), and the text snippet. Interleaved with OCR results by timestamp.

## Conversion Design

Not applicable -- no onboarding flow. Audio capture is opt-in via tray toggles.

## Reusable Components

### Existing Code to Leverage

| File | What to reuse |
|---|---|
| `src-tauri/src/capture.rs` | `CaptureState` / `SharedCaptureState` pattern, `cortex_data_dir()`, stop-flag loop, SCStream setup |
| `src-tauri/src/ocr_worker.rs` | Background worker pattern (poll-process-retry loop, `AtomicBool` stop flag, batch processing, retry with max attempts) |
| `src-tauri/src/search.rs` | `SearchResult` struct, FTS5 query pattern with dynamic params, `insert_fts` pattern |
| `src-tauri/src/tray.rs` | `MenuItem::with_id` pattern, `update_menu_item` helper, menu event handler structure |
| `screencapturekit` crate | Already a dependency -- audio streams are a configuration flag on the existing `SCStream` |

### New Components Required

| Component | Purpose |
|---|---|
| `src-tauri/src/audio_capture.rs` | Audio stream management, PCM buffering, 30s chunk slicing, Opus encoding |
| `src-tauri/src/audio_worker.rs` | Background transcription worker (mirrors `ocr_worker.rs`) |
| `src-tauri/src/whisper.rs` | Thin wrapper around whisper-rs: model loading, PCM-to-text, model download |
| Migration v3 in `storage.rs` | `transcriptions` table, `transcriptions_fts` virtual table, indexes |

## Technical Approach

### Audio Pipeline

```
ScreenCaptureKit audio stream (PCM f32, 48kHz)
  -> Ring buffer accumulates 30s of samples
  -> On chunk boundary:
       1. Resample 48kHz -> 16kHz mono (whisper input format)
       2. Encode original as Opus -> save to ~/.cortex/audio/
       3. Queue chunk for transcription
```

### Transcription Worker

Follows `ocr_worker.rs` pattern exactly:
- Dedicated thread with `AtomicBool` stop flag
- Polls `transcriptions` table for rows with `status = 'pending'`
- Loads whisper model once at startup (lazy init on first chunk)
- Processes one chunk at a time, writes text back, marks `status = 'completed'`
- Retry up to 3 times on failure, then mark `status = 'failed'`

### Schema (Migration v3)

```sql
CREATE TABLE transcriptions (
    id INTEGER PRIMARY KEY,
    capture_id INTEGER REFERENCES captures(id),
    timestamp_start TEXT NOT NULL,
    timestamp_end TEXT NOT NULL,
    text TEXT NOT NULL DEFAULT '',
    source TEXT NOT NULL CHECK(source IN ('system', 'mic')),
    audio_path TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    retries INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX idx_transcriptions_source ON transcriptions(source);
CREATE INDEX idx_transcriptions_timestamp ON transcriptions(timestamp_start);

CREATE VIRTUAL TABLE transcriptions_fts USING fts5(
    transcription_id UNINDEXED,
    text,
    content=transcriptions,
    content_rowid=id
);
```

### Search Extension

`search_captures` returns a union of OCR and transcription results. Add a `result_type` field ("screen" | "audio") and `source` field (null for screen, "system" | "mic" for audio) to `SearchResult`.

### New Crate Dependencies

- `whisper-rs` -- whisper.cpp Rust bindings
- `opus` or `audiopus` -- Opus encoding
- `rubato` -- Audio resampling (48kHz to 16kHz)

### Tray Changes

Add two new `MenuItem`s with IDs `toggle_system_audio` and `toggle_microphone`. Each toggles independently. Status line updates to reflect active audio sources.

## Out of Scope

- Speaker diarization (deferred to a future spec)
- Real-time subtitles or live transcription overlay
- Meeting summarization (spec #8)
- Sentiment analysis
- VAD-based smart chunking (30s fixed chunks for v1)
- Real-time translation
- Audio playback in the UI

## Success Criteria

1. Enabling system audio capture and playing a podcast for 2 minutes produces 4 transcription rows in the DB with non-empty text.
2. Searching for a word spoken in the podcast returns the transcription snippet alongside any OCR results.
3. Audio files older than 7 days are automatically deleted; their transcription text remains searchable.
4. Toggling audio capture on/off does not interrupt or affect screen capture.
5. Whisper model downloads on first audio enable and is reused on subsequent launches.
6. CPU usage during transcription of a single 30s chunk stays under 30% on M1 MacBook Air.
7. Mic toggle is disabled with a "(macOS 15+)" label on systems running macOS 14.x.
