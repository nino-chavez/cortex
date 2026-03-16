# Spec Requirements: Audio Capture & Transcription

## Initial Description
Audio Capture & Transcription — Capture system audio and microphone input via ScreenCaptureKit audio streams. Transcribe audio chunks using whisper.cpp/mlx-whisper with timestamps and speaker diarization. Store transcriptions in the same SQLite database with FTS5 indexing.

## Requirements Discussion

### First Round Questions

**Q1: Proof of Life**
**Answer:** User enables audio in the tray, speaks for 30 seconds or plays a video. A search for a spoken keyword returns the text snippet, the specific timestamp, and identifies whether it was from "System" (speakers) or "Mic" (user).

**Q2: Value Signal**
**Answer:** "Invisible Secretary" — user is in a deep-focus meeting and doesn't take notes. Later, they search for "action items" or a project name mentioned by a teammate, and get the exact quote. Metric: ability to scrub the timeline and hear the audio snippet to verify transcription accuracy.

**Q3: Audio Sources**
**Answer:** Capture both system audio and microphone as distinct streams. Store as separate files/records so search results clarify source (system vs mic). Both together gives full meeting conversation.

**Q4: Transcription Engine**
**Answer:** whisper-rs (whisper.cpp Rust bindings). Keeps binary self-contained within Rust ecosystem. Ship `base` or `small` model by default for good speed/accuracy balance on M-series. No Python sidecar needed.

**Q5: Audio Chunking**
**Answer:** 30-second fixed chunks. Simpler to map to screen capture timestamps. Aligns with capture_id from screen daemon for Timeline view later. VAD deferred.

**Q6: Speaker Diarization**
**Answer:** Deferred. Basic "Mic vs System" labeling provides 80% of the value. Pyannote would increase resource footprint and complexity.

**Q7: Storage Model**
**Answer:** Confirmed: separate `transcriptions` table with columns: id, capture_id (nullable FK), timestamp_start, timestamp_end, text, source (system|mic), audio_path. Plus `transcriptions_fts(transcription_id, text)` FTS5 table feeding into the same search pipeline as OCR.

**Q8: Audio File Storage**
**Answer:** Store as compressed Opus in `~/.cortex/audio/YYYY/MM/DD/`. Rolling 7-day retention (configurable) — keep audio for verification, then delete but keep text in SQLite forever. Text is cheap; audio is expensive.

**Q9: Always-on vs Toggle**
**Answer:** Separate toggles in the tray menu:
- Record Screen
- Record System Audio (Speakers)
- Record Microphone
Audio is higher privacy tier. Clear visual indicator when recording. Prevents "hot mic" anxiety.

**Q10: Out of Scope**
**Answer:** Confirmed: no real-time translation/subtitles, no LLM summarization (spec #8), no sentiment analysis.

### Existing Code to Reference

- **screencapturekit** crate — already supports audio streams (`.with_captures_audio(true)`, `.with_captures_microphone(true)`)
- **storage.rs** — migration system for schema v3
- **ocr_worker.rs** — background worker pattern for transcription worker
- **search.rs** — FTS5 pattern, extend search to include transcriptions
- **tray.rs** — extend with audio toggle menu items

### Follow-up Questions
None needed.

## Visual Assets
None provided.

## Requirements Summary

### Functional Requirements
- Capture system audio via ScreenCaptureKit `.with_captures_audio(true)`
- Capture microphone via ScreenCaptureKit `.with_captures_microphone(true)` (macOS 15+)
- Store audio as 30-second Opus chunks in `~/.cortex/audio/YYYY/MM/DD/`
- Separate system and mic streams, labeled in DB
- Transcribe audio chunks via whisper-rs (whisper.cpp bindings)
- Store transcriptions in `transcriptions` table with FTS5 indexing
- Link transcriptions to concurrent screen captures via nullable `capture_id`
- Extend search to include transcription text alongside OCR results
- 7-day rolling audio retention (configurable), keep text forever
- Separate tray toggles for screen, system audio, and microphone
- Ship whisper `base` or `small` model (download on first use or bundle)

### Schema (Migration v3)
- `transcriptions` table: id, capture_id (nullable), timestamp_start, timestamp_end, text, source (system|mic), audio_path
- `transcriptions_fts(transcription_id, text)` FTS5 virtual table
- Index on `transcriptions(source)` and `transcriptions(timestamp_start)`

### Tauri Commands
- `toggle_system_audio(enabled: bool)` — start/stop system audio capture
- `toggle_microphone(enabled: bool)` — start/stop mic capture
- `search_captures` — extend existing to also search transcriptions_fts
- `get_audio_status()` — returns system_audio and mic recording state

### Scope Boundaries
**In Scope:**
- Audio capture (system + mic as separate streams)
- 30-second chunking and Opus encoding
- Whisper transcription via whisper-rs
- FTS5 indexing of transcriptions
- Search across both OCR and transcription text
- Tray toggles for audio sources
- 7-day audio retention

**Out of Scope:**
- Speaker diarization (deferred)
- Real-time subtitles/translation
- Meeting summarization (spec #8)
- Sentiment analysis
- VAD-based chunking

### Technical Considerations
- whisper-rs crate for Rust bindings to whisper.cpp
- Opus encoding via opus crate or ogg/vorbis
- ScreenCaptureKit audio streams deliver raw PCM samples
- Need to convert PCM → WAV (for whisper) and PCM → Opus (for storage)
- whisper.cpp needs 16kHz mono WAV input
- Background transcription worker follows ocr_worker pattern
- Model file (~150MB for base, ~500MB for small) needs download/bundle strategy
- macOS 15.0+ required for microphone capture via ScreenCaptureKit
