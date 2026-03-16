# Task Breakdown: Audio Capture & Transcription

## Overview
Total Tasks: 4 groups, 27 subtasks
Strategy: squad
Depth: standard
Assigned roles: api-engineer (audio capture + whisper integration + tray), database-engineer (schema + FTS5 + search extension), testing-engineer (gap analysis + integration tests)

## Task List

### Proof of Life — Vertical Slice

#### Task Group 1: Audio Capture + Transcription + Search
**Assigned implementer:** api-engineer, database-engineer
**Dependencies:** OCR Pipeline spec (Task Groups 1-2) must be complete — `captures` table with migration system, FTS5 search, and `ocr_worker.rs` pattern exist.

This group delivers the Proof of Life scenario: user enables audio capture in the tray, speaks or plays audio for 30-60 seconds, searches for a spoken word, and gets back the transcription with timestamp and source label. It is a vertical slice across audio capture, whisper transcription, schema migration, FTS5 indexing, and unified search.

- [ ] 1.0 Complete minimal end-to-end audio capture -> transcription -> search pipeline
  - [ ] 1.1 Update `src-tauri/Cargo.toml` — Add new dependencies: `whisper-rs = { version = "0.15", features = ["metal"] }` (whisper.cpp with Metal GPU acceleration), `hound = "3.5"` (WAV read/write). Verify `rusqlite` already has `"bundled"` feature (includes FTS5).
  - [ ] 1.2 Add schema migration v2 to v3 in `src-tauri/src/storage.rs` — Create `transcriptions` table: `id INTEGER PRIMARY KEY`, `capture_id INTEGER REFERENCES captures(id)` (nullable), `timestamp_start TEXT NOT NULL`, `timestamp_end TEXT NOT NULL`, `text TEXT NOT NULL DEFAULT ''`, `source TEXT NOT NULL CHECK(source IN ('system', 'mic'))`, `audio_path TEXT NOT NULL`, `status TEXT NOT NULL DEFAULT 'pending'`, `retries INTEGER NOT NULL DEFAULT 0`. Create indexes: `idx_transcriptions_source ON transcriptions(source)`, `idx_transcriptions_timestamp ON transcriptions(timestamp_start)`. Create FTS5 virtual table: `CREATE VIRTUAL TABLE transcriptions_fts USING fts5(transcription_id UNINDEXED, text, content=transcriptions, content_rowid=id)`. Bump `CURRENT_SCHEMA_VERSION` to 3.
  - [ ] 1.3 Create `src-tauri/src/whisper.rs` — Implement whisper model wrapper. `WhisperContext` struct holds a loaded model. `pub fn load_model(model_path: &Path) -> Result<WhisperContext>` loads `ggml-base.en.bin`. `pub fn transcribe(ctx: &WhisperContext, pcm_data: &[f32]) -> Result<String>` runs whisper inference on 16kHz mono f32 samples and returns concatenated text. Use `whisper_rs::WhisperContext` and `FullParams` with `Strategy::Greedy`, language "en", no translate. Return empty string if no speech detected.
  - [ ] 1.4 Create `src-tauri/src/audio_capture.rs` — Implement system audio capture via ScreenCaptureKit. Configure existing `SCStream` with `.with_captures_audio(true)`. Accumulate PCM f32 samples from `CMSampleBuffer` audio callbacks into a ring buffer. On 30-second boundary: copy buffer into a `Vec<f32>`, resample from 48kHz stereo to 16kHz mono (average channels, then decimate by factor of 3), write resampled PCM as WAV via `hound` to `~/.cortex/audio/YYYY/MM/DD/{timestamp}.wav`, insert a row into `transcriptions` table with `status = 'pending'` and `source = 'system'`. Use `cortex_data_dir()` from `capture.rs` for base path. Use `AtomicBool` stop flag pattern from `capture.rs`.
  - [ ] 1.5 Create `src-tauri/src/audio_worker.rs` — Implement background transcription worker following `ocr_worker.rs` pattern exactly. `pub fn start_audio_worker(db: Arc<Database>, stop_flag: Arc<AtomicBool>)` spawns a dedicated thread. Poll `transcriptions` table for rows with `status = 'pending'`, ordered by `timestamp_start DESC`, limit 5 per batch. For each: set `status = 'processing'`, load WAV via `hound`, call `whisper::transcribe()`, on success write text back to `transcriptions` row and insert into `transcriptions_fts`, set `status = 'completed'`. On failure increment `retries`, if `retries >= 3` set `status = 'failed'`. Sleep 3 seconds between polls when idle. Lazy-init whisper model on first chunk (load from `~/.cortex/models/whisper/ggml-base.en.bin`).
  - [ ] 1.6 Extend `src-tauri/src/search.rs` — Add `result_type` field (`"screen"` | `"audio"`) and `source` field (`Option<String>`: `None` for screen, `Some("system")` or `Some("mic")` for audio) to `SearchResult` struct. Modify `search_captures()` to UNION the existing `captures_fts` query with a new query against `transcriptions_fts JOIN transcriptions`. Return unified results sorted by timestamp descending. Use `snippet(transcriptions_fts, 1, '<b>', '</b>', '...', 32)` for audio snippets.
  - [ ] 1.7 Add basic tray toggle for system audio in `src-tauri/src/tray.rs` — Add `MenuItem::with_id("toggle_system_audio", "Record System Audio")` to the tray menu below the existing capture toggle. Handle click to start/stop audio capture via `audio_capture.rs`. Update menu item text to show checkmark when active.
  - [ ] 1.8 Wire audio modules into `src-tauri/src/lib.rs` — Declare `mod whisper; mod audio_capture; mod audio_worker;`. Start audio worker during Tauri app initialization after storage init. Pass shared `Arc<Database>` and stop flag.
  - [ ] 1.9 Write 6 tests: (a) migration v2 to v3 creates `transcriptions` table with correct columns, (b) migration v2 to v3 creates `transcriptions_fts` virtual table, (c) insert into `transcriptions` and `transcriptions_fts`, then FTS5 MATCH query returns correct row, (d) `search_captures` returns both OCR and transcription results in a single query, (e) `search_captures` results include correct `result_type` and `source` fields, (f) search returns empty vec when no transcription matches.

**Acceptance Criteria:**
- `cargo build --manifest-path src-tauri/Cargo.toml` compiles with whisper-rs (metal feature) and hound
- Schema migration upgrades a v2 database to v3 without data loss
- `transcriptions` and `transcriptions_fts` tables are created with correct schema
- Audio capture via ScreenCaptureKit produces 30-second WAV chunks in `~/.cortex/audio/`
- Whisper transcription of a WAV chunk produces non-empty text for audible speech
- `search_captures("spoken_word")` returns transcription results with `result_type = "audio"` and `source = "system"`
- Tray toggle starts/stops system audio capture independently of screen capture
- All 6 tests pass

**Verification Steps:**
1. Build the project and confirm whisper-rs with metal links successfully
2. Enable "Record System Audio" in the tray, play a YouTube video for 30 seconds, wait for transcription worker to process
3. Search for a word spoken in the video -- expect a result with audio source label
4. Query the database directly to verify transcription rows and FTS5 entries

**Verification Commands:**
```bash
# Build (verifies whisper-rs metal and hound compile and link)
cargo build --manifest-path src-tauri/Cargo.toml

# Run tests
cargo test --manifest-path src-tauri/Cargo.toml --lib

# Verify schema migration
sqlite3 ~/.cortex/cortex.db "PRAGMA table_info(transcriptions);"

# Verify FTS5 table exists
sqlite3 ~/.cortex/cortex.db "SELECT * FROM transcriptions_fts LIMIT 1;"

# Verify transcription rows after audio capture
sqlite3 ~/.cortex/cortex.db "SELECT id, source, status, substr(text, 1, 80) FROM transcriptions ORDER BY timestamp_start DESC LIMIT 10;"

# Verify unified search returns both types
sqlite3 ~/.cortex/cortex.db "SELECT capture_id, snippet(transcriptions_fts, 1, '<b>', '</b>', '...', 32) FROM transcriptions_fts WHERE transcriptions_fts MATCH 'test';"

# Check audio files on disk
ls -la ~/.cortex/audio/$(date +%Y/%m/%d)/
```

---

### Microphone Capture

#### Task Group 2: Microphone Capture & Dual Stream
**Assigned implementer:** api-engineer
**Dependencies:** Task Group 1 (audio capture, transcription worker, and schema exist)

- [ ] 2.0 Complete microphone capture as a separate stream with source labeling and macOS version gating
  - [ ] 2.1 Add microphone stream to `src-tauri/src/audio_capture.rs` — Add a separate capture path using ScreenCaptureKit `.with_captures_microphone(true)` (macOS 15.0+ only). Mic and system audio run as independent streams, each writing separate WAV chunks. Mic chunks are inserted into `transcriptions` with `source = 'mic'`. Each stream has its own ring buffer and 30-second boundary tracking.
  - [ ] 2.2 Implement macOS version check — At startup, detect macOS version. If < 15.0, disable microphone capture and mark the tray item as unavailable. Add helper `fn supports_microphone_capture() -> bool` that checks `NSProcessInfo.processInfo.operatingSystemVersion.majorVersion >= 15`.
  - [ ] 2.3 Handle macOS Microphone permission — Before starting mic capture, request microphone access via `AVCaptureDevice.requestAccess(for: .audio)`. If denied, show a notification explaining the permission is needed and keep the toggle disabled. Store permission state to avoid repeated prompts.
  - [ ] 2.4 Add mic toggle to `src-tauri/src/tray.rs` — Add `MenuItem::with_id("toggle_microphone", "Record Microphone")` below the system audio toggle. On macOS < 15.0, append " (macOS 15+)" to the label and make it non-interactive. Handle click to start/stop mic capture. Update tray status line to show active sources (e.g., "Audio: System + Mic").
  - [ ] 2.5 Verify dual-stream independence — Ensure starting/stopping mic does not affect system audio stream and vice versa. Ensure starting/stopping either audio stream does not affect screen capture. Each stream writes its own chunks with correct `source` label.
  - [ ] 2.6 Write 4 tests: (a) mic transcription rows have `source = 'mic'`, system rows have `source = 'system'`, (b) search results from mic include `source = Some("mic")`, (c) `supports_microphone_capture()` returns a bool without crashing, (d) stopping one audio stream does not affect the other's status in the database.

**Acceptance Criteria:**
- Microphone capture produces separate transcription rows with `source = 'mic'`
- System and mic streams operate independently (start/stop one without affecting the other)
- Screen capture continues unaffected while audio streams are active
- On macOS < 15.0, mic toggle is disabled with "(macOS 15+)" label
- Microphone permission is requested before first mic capture
- Search results correctly distinguish between "System" and "Mic" sources
- All 4 tests pass

**Verification Steps:**
1. Enable both system audio and microphone in the tray
2. Play audio and speak simultaneously for 30 seconds
3. Search for a spoken word -- expect results with both "system" and "mic" source labels
4. Stop mic capture, verify system audio continues producing transcriptions

**Verification Commands:**
```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib -- audio

# Verify both source types in database
sqlite3 ~/.cortex/cortex.db "SELECT source, COUNT(*) FROM transcriptions GROUP BY source;"

# Verify source labels in search results
sqlite3 ~/.cortex/cortex.db "SELECT t.source, snippet(f, 1, '<b>', '</b>', '...', 32) FROM transcriptions_fts f JOIN transcriptions t ON t.id = f.transcription_id WHERE f.transcriptions_fts MATCH 'hello';"

# Check both audio source directories have files
ls -la ~/.cortex/audio/$(date +%Y/%m/%d)/
```

---

### Storage & Retention

#### Task Group 3: Audio Storage & Retention
**Assigned implementer:** api-engineer, database-engineer
**Dependencies:** Task Groups 1 and 2 (audio capture pipeline and transcription working)

- [ ] 3.0 Complete Opus encoding, retention policy, cleanup worker, and model download management
  - [ ] 3.1 Add Opus encoding dependency to `src-tauri/Cargo.toml` — Add `ogg-opus = "0.4"` (or `audiopus` + `ogg`) for Opus encoding. Update `audio_capture.rs` to encode each 30-second chunk as Opus (`.opus`) instead of WAV for long-term storage. Keep a temporary WAV copy for whisper processing, delete after transcription completes.
  - [ ] 3.2 Update audio file path scheme — Store Opus files at `~/.cortex/audio/YYYY/MM/DD/{timestamp}_{source}.opus`. Update `audio_path` in `transcriptions` table to point to the `.opus` file. Update `audio_worker.rs` to read the temporary WAV for transcription, then delete the WAV after successful transcription.
  - [ ] 3.3 Implement 7-day rolling retention in `src-tauri/src/storage.rs` — Add `pub fn cleanup_old_audio(db: &Database, retention_days: u64) -> Result<u64>` that: queries `transcriptions` where `timestamp_start < (now - retention_days)`, deletes the audio file at `audio_path` from disk, sets `audio_path` to empty string (keep the text row). Returns count of files deleted.
  - [ ] 3.4 Add configurable retention policy — Store `audio_retention_days` in a `settings` table (create if not exists as part of migration v3, or add a lightweight config file at `~/.cortex/config.json`). Default to 7 days. Expose `set_audio_retention(days: u64)` and `get_audio_retention() -> u64` as Tauri commands.
  - [ ] 3.5 Implement auto-cleanup worker — Run `cleanup_old_audio` on a schedule: once at app startup, then every hour on a background timer. Log the number of files cleaned up. Use the same `AtomicBool` stop-flag pattern.
  - [ ] 3.6 Implement model download management in `src-tauri/src/whisper.rs` — On first use, if `~/.cortex/models/whisper/ggml-base.en.bin` does not exist: download from `https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin` (~148MB). Show download progress via tray status text (e.g., "Downloading whisper model... 45%"). Create the directory if needed. Verify file size after download. If download fails or is interrupted, delete partial file and retry on next attempt.
  - [ ] 3.7 Write 4 tests: (a) `cleanup_old_audio` deletes files older than retention period and preserves newer files, (b) `cleanup_old_audio` keeps transcription text rows after audio file deletion, (c) retention days config read/write round-trip, (d) model path resolution returns correct path under `~/.cortex/models/whisper/`.

**Acceptance Criteria:**
- Audio chunks are stored as Opus files (smaller than WAV) at the correct path
- Temporary WAV files are deleted after successful transcription
- Audio files older than 7 days are automatically deleted
- Transcription text remains searchable after audio file deletion
- Retention period is configurable (default 7 days)
- Whisper model downloads automatically on first use with progress indicator
- Partial/failed model downloads are cleaned up
- All 4 tests pass

**Verification Steps:**
1. Enable audio capture, verify `.opus` files appear in `~/.cortex/audio/`
2. Verify no leftover `.wav` files after transcription completes
3. Manually set retention to 0 days, trigger cleanup, verify audio files deleted but text rows remain
4. Delete the whisper model file, restart with audio enabled, verify it re-downloads

**Verification Commands:**
```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib -- cleanup
cargo test --manifest-path src-tauri/Cargo.toml --lib -- retention

# Verify Opus files on disk
find ~/.cortex/audio/ -name "*.opus" | head -5

# Verify no stale WAV files
find ~/.cortex/audio/ -name "*.wav" | wc -l

# Check transcription text survives audio deletion
sqlite3 ~/.cortex/cortex.db "SELECT id, audio_path, substr(text, 1, 60) FROM transcriptions WHERE audio_path = '' AND text != '' LIMIT 5;"

# Verify model file
ls -lh ~/.cortex/models/whisper/ggml-base.en.bin

# Check retention config
sqlite3 ~/.cortex/cortex.db "SELECT * FROM settings WHERE key = 'audio_retention_days';"
```

---

### Testing & Integration

#### Task Group 4: Test Review, Gap Analysis, Integration Verification
**Assigned implementer:** testing-engineer
**Dependencies:** Task Groups 1, 2, 3

- [ ] 4.0 Complete test coverage review and fill gaps with integration tests
  - [ ] 4.1 Review all tests from Groups 1-3 (14 total). Verify they compile and pass. Document any that are environment-dependent (e.g., whisper model must be downloaded, microphone permission required, Metal GPU required for whisper-rs).
  - [ ] 4.2 Integration test: full audio pipeline — Start system audio capture, play audio for 30 seconds, wait for transcription worker to process. Assert: (a) at least one `transcriptions` row with `status = 'completed'`, (b) FTS5 entry exists for the transcription, (c) `search_captures` with a word from the audio returns a result with `result_type = "audio"`.
  - [ ] 4.3 Integration test: unified search ranking — Insert 5 OCR captures and 5 transcription rows with overlapping keywords. Run `search_captures`. Assert results are interleaved by timestamp (not grouped by type) and both `result_type` values appear.
  - [ ] 4.4 Integration test: audio retention cleanup — Insert transcription rows with `timestamp_start` set to 10 days ago and audio files on disk. Run `cleanup_old_audio(7)`. Assert: (a) audio files are deleted from disk, (b) transcription text rows remain in the database, (c) `audio_path` is cleared, (d) FTS5 search still returns the text.
  - [ ] 4.5 Integration test: migration v2 to v3 safety — Create a v2 database with 5 capture rows and FTS5 entries. Run migration to v3. Assert: (a) all 5 capture rows still exist with OCR data intact, (b) `transcriptions` table exists, (c) `transcriptions_fts` table exists, (d) `schema_version = 3`.
  - [ ] 4.6 Integration test: audio capture independence — Start screen capture and system audio capture simultaneously. Stop audio capture after 30 seconds. Assert screen capture continues producing new rows. Start audio capture again. Assert both resume without errors.
  - [ ] 4.7 Gap analysis — Document untested paths: whisper model download interruption mid-transfer, disk full during audio chunk write, 48kHz to 16kHz resampling accuracy, audio capture on M1 vs M2 vs M3 Metal performance, simultaneous mic + system audio under high CPU, Opus encoding of silence (empty audio), very noisy audio producing garbage transcriptions, concurrent transcription worker + OCR worker contention on SQLite. File as future test TODOs in a comment block in test files.

**Acceptance Criteria:**
- All tests from Groups 1-3 pass (14 unit tests)
- 5 new integration tests added and passing
- Gap analysis identifies at least 6 untested edge cases
- Total test count: 19+ (14 from Groups 1-3 + 5 new)
- Unified search returns correctly interleaved OCR + audio results
- Audio retention cleanup preserves searchable text

**Verification Steps:**
1. Run full test suite -- expect all tests pass
2. Review test output for flaky tests or environment warnings
3. Verify gap analysis documents meaningful risk areas

**Verification Commands:**
```bash
# Run all tests
cargo test --manifest-path src-tauri/Cargo.toml --lib

# Run tests with output for debugging
cargo test --manifest-path src-tauri/Cargo.toml --lib -- --nocapture

# Run only audio-related tests
cargo test --manifest-path src-tauri/Cargo.toml --lib -- audio
cargo test --manifest-path src-tauri/Cargo.toml --lib -- transcription
cargo test --manifest-path src-tauri/Cargo.toml --lib -- whisper

# List all tests
cargo test --manifest-path src-tauri/Cargo.toml --lib -- --list 2>&1 | tail -1

# Build release to verify no compile warnings
cargo build --manifest-path src-tauri/Cargo.toml --release 2>&1 | grep warning
```

---

## Execution Order

1. **Task Group 1: Audio Capture + Transcription + Search** (api-engineer + database-engineer) -- Proof of Life vertical slice. Must complete first.
2. **Task Group 2: Microphone Capture & Dual Stream** (api-engineer) -- Depends on Group 1 for audio capture infrastructure and schema.
3. **Task Group 3: Audio Storage & Retention** (api-engineer + database-engineer) -- Depends on Groups 1 and 2. Can partially overlap with Group 2: Opus encoding (3.1-3.2) only depends on Group 1, while retention (3.3-3.5) can start after Group 1.
4. **Task Group 4: Test Review & Integration** (testing-engineer) -- Depends on all prior groups completing.

**Parallel execution possible:** Groups 2 and 3 can partially overlap after Group 1 completes -- the Opus encoding work (3.1-3.2) and model download (3.6) only depend on Group 1, while mic-specific tests in Group 2 are independent of retention work in Group 3.
