# Task Breakdown: Meeting Memory

## Overview
Total Tasks: 3 groups, 18 subtasks
Strategy: squad
Depth: standard

## Task List

### Proof of Life -- Vertical Slice

#### Task Group 1: Meetings Table, Tray Toggle, Grouped Captures, Basic Meeting Record
**Dependencies:** Capture Daemon (complete), Audio Transcription (complete), Local AI Chat Task Group 1 (Ollama client exists in chat.rs)

This group delivers the core meeting flow: user clicks "Start Meeting" in the tray, captures happen at 2s intervals with a shared meeting_id, user clicks "End Meeting", and a meeting record is created in the database.

- [ ] 1.0 Complete meeting mode toggle with grouped captures and meeting record storage
  - [ ] 1.1 Schema migration v5 in `storage.rs` -- Bump `CURRENT_SCHEMA_VERSION` to 5. In the `if version < 5` block:
    - `ALTER TABLE captures ADD COLUMN meeting_id TEXT;`
    - `ALTER TABLE transcriptions ADD COLUMN meeting_id TEXT;`
    - `CREATE TABLE meetings (id TEXT PRIMARY KEY, title TEXT NOT NULL DEFAULT 'Untitled Meeting', start_time TEXT NOT NULL, end_time TEXT NOT NULL, summary TEXT NOT NULL DEFAULT '', participant_count INTEGER NOT NULL DEFAULT 1);`
    - `CREATE VIRTUAL TABLE meetings_fts USING fts5(meeting_id, title, summary, tokenize='unicode61');`
    - `CREATE INDEX idx_captures_meeting ON captures(meeting_id);`
    - `CREATE INDEX idx_transcriptions_meeting ON transcriptions(meeting_id);`
  - [ ] 1.2 Add meeting storage methods to `Database` in `storage.rs`:
    - `insert_meeting(id, title, start_time, end_time, summary, participant_count) -> Result<()>`
    - `update_meeting_summary(id, summary) -> Result<()>`
    - `get_meeting(id) -> Result<Option<MeetingRow>>`
    - `list_meetings(limit) -> Result<Vec<MeetingRow>>`
    - `get_meeting_transcriptions(meeting_id) -> Result<Vec<String>>` -- returns transcription texts ordered by timestamp
    - Define `MeetingRow` struct: id, title, start_time, end_time, summary, participant_count.
  - [ ] 1.3 Extend `insert_capture` and `insert_transcription` signatures to accept `Option<&str>` for meeting_id. Update all existing call sites to pass `None`.
  - [ ] 1.4 Create `src-tauri/src/meeting.rs`:
    - `MeetingState { active: bool, meeting_id: Option<String>, start_time: Option<String> }`
    - `pub type SharedMeetingState = Arc<Mutex<MeetingState>>;`
    - `pub fn start_meeting(meeting_state, capture_state) -> String` -- Generate UUID, set meeting_id, override capture interval to 2s, return meeting_id.
    - `pub fn end_meeting(meeting_state, capture_state, db) -> Result<MeetingRow>` -- Restore 5s interval, create meeting record with start/end time, return it.
  - [ ] 1.5 Register Tauri commands:
    - `#[tauri::command] start_meeting(...)` -- calls `meeting::start_meeting`, returns meeting_id.
    - `#[tauri::command] end_meeting(...)` -- calls `meeting::end_meeting`, returns MeetingRow.
    - `#[tauri::command] get_meeting(meeting_id)` -- returns MeetingRow.
    - `#[tauri::command] list_meetings(limit)` -- returns Vec<MeetingRow>.
    - Register in `lib.rs`, add `mod meeting;`, add `SharedMeetingState` to Tauri managed state.
  - [ ] 1.6 Add tray menu items in `lib.rs` -- Add "Start Meeting" to the system tray menu. On click: invoke start_meeting, update menu item to "End Meeting". On "End Meeting" click: invoke end_meeting, update menu item back to "Start Meeting".
  - [ ] 1.7 Wire meeting_id into capture pipeline -- In `capture.rs` `process_frame`, read the current meeting_id from `SharedMeetingState` and pass it to `insert_capture`. Similarly wire into `audio.rs` `start_audio_worker` for `insert_transcription`.
  - [ ] 1.8 Write 4 tests:
    - (a) Migration v5 creates meetings table and adds meeting_id columns to captures and transcriptions.
    - (b) `insert_meeting` + `get_meeting` round-trip with all fields.
    - (c) `start_meeting` sets interval to 2s and generates a valid UUID meeting_id.
    - (d) `end_meeting` restores interval to 5s and creates a meeting record.

**Acceptance Criteria:**
- Schema v5 migration runs cleanly on existing v4 databases
- "Start Meeting" in tray activates 2s capture interval
- Captures during meeting mode have meeting_id set
- "End Meeting" restores 5s interval and inserts a meeting record
- All 4 tests pass

**Verification Commands:**
```bash
cargo build --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml --lib -- meeting
cargo test --manifest-path src-tauri/Cargo.toml --lib -- storage::tests::migration
```

---

### Summary Generation

#### Task Group 2: Ollama Summary on Meeting End
**Dependencies:** Task Group 1 (meetings table exists, meeting records created)

- [ ] 2.0 Generate AI summary from meeting transcriptions via Ollama
  - [ ] 2.1 Add `generate_meeting_summary` function in `meeting.rs`:
    - Fetch all transcriptions for the meeting_id via `db.get_meeting_transcriptions(meeting_id)`.
    - Concatenate texts chronologically.
    - Build summarization prompt (key topics, decisions, action items format).
    - Call Ollama `/api/generate` using the reqwest pattern from `chat.rs` (blocking, non-streaming, 120s timeout).
    - Return the summary text.
  - [ ] 2.2 Integrate summary into `end_meeting` flow -- After creating the meeting record, spawn a background task to generate the summary. On completion, call `db.update_meeting_summary(meeting_id, summary)` and insert into `meetings_fts`.
  - [ ] 2.3 Add `meeting_summary_status` tracking -- Add a `summary_status TEXT DEFAULT 'pending'` column to meetings table (in migration v5). Update to 'completed' or 'failed' after Ollama call. Expose status in `MeetingRow`.
  - [ ] 2.4 Add Tauri event `meeting:summary_ready` -- Emit when async summary completes so the frontend can refresh if viewing the meeting.
  - [ ] 2.5 Write 3 tests:
    - (a) Summary prompt includes all transcription texts in chronological order.
    - (b) `update_meeting_summary` correctly updates the meetings table and inserts into meetings_fts.
    - (c) Meeting with no transcriptions gets a "No transcript available" summary.

**Acceptance Criteria:**
- Ending a meeting triggers async Ollama summarization
- Summary is stored in meetings table and indexed in meetings_fts
- Meeting end is not blocked by summary generation
- Frontend receives `meeting:summary_ready` event when summary completes

**Verification Commands:**
```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib -- meeting::tests::summary
# Manual: start Ollama, start meeting, speak, end meeting, check meetings table
```

---

### Search Integration + Meeting List

#### Task Group 3: Meeting Search + Meeting List View
**Dependencies:** Task Group 2 (meetings with summaries exist)

- [ ] 3.0 Integrate meetings into unified search and provide a meeting list view
  - [ ] 3.1 Extend `search_captures` in `search.rs` -- Add a third UNION ALL branch querying `meetings_fts`. Map to `SearchResult` with `result_type: "meeting"`, `capture_id` as the meeting row (use a negative ID or string prefix to distinguish), `snippet` from the summary FTS snippet, `app_name` as "Meeting", `timestamp` as start_time.
  - [ ] 3.2 Create `/meetings` route -- `src/routes/meetings/+page.svelte`. List view showing recent meetings: title, date/time, duration, summary preview (first 200 chars), summary_status badge. Click navigates to meeting detail.
  - [ ] 3.3 Create meeting detail view -- `src/routes/meetings/[id]/+page.svelte`. Shows: title (editable), start/end time, duration, full summary (markdown rendered), and a timeline of captures tagged with this meeting_id (reuse timeline components).
  - [ ] 3.4 Add "Meetings" to sidebar navigation in the main app layout.
  - [ ] 3.5 Write 2 tests:
    - (a) `search_captures("budget")` returns meeting results when a meeting summary contains "budget".
    - (b) `list_meetings(10)` returns meetings ordered by start_time descending.

**Acceptance Criteria:**
- Searching for meeting content returns meeting results alongside OCR and transcription results
- `/meetings` route shows a list of meetings with summaries
- Meeting detail view shows full summary and linked captures
- Meetings link in sidebar navigation

**Verification Commands:**
```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib -- search::tests::meeting
npm run tauri dev
# Navigate to /meetings, verify list and detail views
```

---

## Execution Order

1. **Task Group 1: PoL** -- Schema, tray toggle, grouped captures. Must complete first.
2. **Task Group 2: Summary Generation** -- Depends on Group 1 for meeting records and transcription grouping.
3. **Task Group 3: Search + UI** -- Depends on Group 2 for meetings with summaries to search and display.
