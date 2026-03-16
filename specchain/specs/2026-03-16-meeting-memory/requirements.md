# Spec Requirements: Meeting Memory

## Initial Description
Meeting Memory -- A "Meeting Mode" toggle in the system tray that switches Cortex into high-fidelity capture mode. When active: screenshot interval drops to 2 seconds, system audio + mic recording auto-enable, and all captures/transcriptions are tagged with a shared meeting_id. When the user ends the meeting, Ollama generates a summary from the collected transcriptions, and a unified meeting record is stored and made searchable.

## Requirements Discussion

### First Round Questions

**Q1: Proof of Life**
**Answer:** User clicks "Start Meeting" in the tray. Cortex captures screenshots every 2 seconds and records audio. User clicks "End Meeting". Cortex calls Ollama to summarize all transcriptions tagged with that meeting_id and stores a meeting record with title, timestamps, and summary. The meeting appears in search results when the user queries "budget meeting" or similar.

**Q2: Tray Toggle**
**Answer:** Add two items to the existing system tray menu: "Start Meeting" (when idle) and "End Meeting" (when active). Starting a meeting generates a UUID meeting_id, overrides the capture interval to 2 seconds, and enables both audio sources. Ending reverses these: restore the 5-second interval and stop audio. The tray icon or label should indicate meeting mode is active.

**Q3: Capture Grouping**
**Answer:** Add an optional `meeting_id` column to the `captures` table and `transcriptions` table. When meeting mode is active, every new capture and transcription row gets the current meeting_id. This allows querying all content from a specific meeting with a simple WHERE clause.

**Q4: Summary Generation**
**Answer:** On "End Meeting", collect all transcription text where `meeting_id = ?`, concatenate chronologically, build a summarization prompt, and call Ollama `/api/generate`. The prompt asks for a structured summary: key topics, decisions, action items. Store the result in the `meetings` table. This reuses the existing Ollama client from `chat.rs`.

**Q5: Schema**
**Answer:** New `meetings` table: `id TEXT PRIMARY KEY` (UUID), `title TEXT`, `start_time TEXT`, `end_time TEXT`, `summary TEXT`, `participant_count INTEGER DEFAULT 1`. Add `meeting_id TEXT` column to `captures` and `transcriptions` tables. Schema migration v5. Add `meetings_fts` FTS5 virtual table on `title` and `summary` for search.

**Q6: Search Integration**
**Answer:** Extend the existing `search_captures` UNION in `search.rs` with a third query against `meetings_fts`. Meeting search results return with `result_type: "meeting"`. No new search infrastructure needed -- just another UNION branch.

**Q7: Out of Scope**
**Answer:** Speaker diarization, participant detection, calendar integration, meeting scheduling, auto-detection of meeting apps, video recording, real-time transcription display during meeting, meeting sharing/export.

### Existing Code to Reference
- **capture.rs** -- `CaptureState` with `interval_secs` field. `SharedCaptureState` (Arc<Mutex>) allows runtime interval changes. `start_capture_loop` reads interval from state.
- **audio.rs** -- `start_audio_worker` processes WAV files from pending directory. `insert_transcription` stores with optional `capture_id`.
- **chat.rs** -- `OLLAMA_BASE_URL`, `DEFAULT_MODEL`, `build_rag_prompt`, and the Ollama HTTP client pattern (reqwest blocking with `/api/generate`).
- **storage.rs** -- `CURRENT_SCHEMA_VERSION = 4`, migration pattern with version checks. `insert_capture`, `insert_transcription`.
- **search.rs** -- `search_captures` with UNION ALL pattern for OCR + transcription FTS5 queries.

## Requirements Summary

### Functional Requirements
- Tray menu items: "Start Meeting" / "End Meeting" toggle
- On start: generate UUID meeting_id, set capture interval to 2s, enable audio recording
- On end: restore 5s interval, stop audio, trigger summary generation
- Tag all captures and transcriptions during meeting with meeting_id
- Ollama summarization of all meeting transcriptions on end
- New `meetings` table with id, title, start_time, end_time, summary, participant_count
- `meeting_id` column added to captures and transcriptions tables
- `meetings_fts` FTS5 table on title + summary
- Search integration: meetings appear in unified search results
- Schema migration v5

### Tauri Commands
- `start_meeting()` -- Returns meeting_id, activates meeting mode
- `end_meeting(meeting_id: String)` -- Triggers summary, stores meeting record, deactivates meeting mode
- `get_meeting(meeting_id: String)` -- Returns meeting record with summary
- `list_meetings(limit: i64)` -- Returns recent meetings

### Schema Changes
- `meetings` table (new)
- `captures.meeting_id` column (ALTER TABLE)
- `transcriptions.meeting_id` column (ALTER TABLE)
- `meetings_fts` FTS5 virtual table (new)

### Scope Boundaries
**In Scope:**
- Tray toggle for meeting mode
- Higher-frequency capture during meetings
- Audio auto-enable during meetings
- meeting_id grouping across captures and transcriptions
- Ollama summary generation on meeting end
- meetings table + FTS5
- Unified search integration

**Out of Scope:**
- Speaker diarization or participant detection
- Calendar integration
- Auto-detection of meeting applications (Zoom, Meet, etc.)
- Video recording
- Real-time transcription display
- Meeting sharing or export
- Meeting title auto-generation (user can rename later)
