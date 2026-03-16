# Specification: Meeting Memory

## Goal

Provide a dedicated meeting capture mode that groups screenshots and audio transcriptions under a shared meeting_id, captures at higher frequency, and generates an AI summary when the meeting ends. Composes existing capture, audio, and Ollama infrastructure -- no new ML pipelines or external services.

## Proof of Life

**Scenario:** User clicks "Start Meeting" in the system tray. The tray label updates to show meeting mode is active. Screenshots now capture every 2 seconds. Audio recording is enabled. After 5 minutes, user clicks "End Meeting". Cortex collects all transcriptions tagged with the meeting_id, sends them to Ollama for summarization, and stores a meeting record. User searches "standup" and the meeting summary appears in results.

**Validates:** Tray toggle works, capture interval dynamically changes, meeting_id groups content correctly, Ollama summarization produces a stored summary, and meetings are searchable via FTS5.

**Must work before:** Any meeting-specific UI views, meeting analytics, or meeting export features.

## User Stories

- As a user, I want to toggle meeting mode from the tray so I capture more detail during important conversations.
- As a user, I want all captures and transcriptions from a meeting grouped together so I can review them as a unit.
- As a user, I want an AI-generated summary of what was discussed when the meeting ends so I don't have to re-listen to the recording.
- As a user, I want to search for meetings by topic and find relevant summaries.

## Core Requirements

### Functional Requirements

- **Tray toggle:** Add "Start Meeting" item to existing tray menu. When clicked, generate a UUID meeting_id, store it in app state, override `CaptureState.interval_secs` to 2, and enable audio capture. Tray item changes to "End Meeting". Clicking "End Meeting" reverses the state and triggers summarization.
- **Capture grouping:** Add `meeting_id TEXT` column to `captures` and `transcriptions` tables (nullable, NULL when not in a meeting). During meeting mode, `insert_capture` and `insert_transcription` calls include the current meeting_id.
- **Summary generation:** On meeting end:
  1. Query all transcriptions WHERE meeting_id = ?.
  2. Concatenate text chronologically.
  3. Build a summarization prompt: "Summarize this meeting transcript. Include: key topics discussed, decisions made, action items."
  4. Call Ollama `/api/generate` (non-streaming, reuse `chat.rs` client pattern).
  5. Store result in `meetings` table.
- **Meetings table:** `id TEXT PRIMARY KEY` (UUID), `title TEXT NOT NULL DEFAULT 'Untitled Meeting'`, `start_time TEXT NOT NULL`, `end_time TEXT NOT NULL`, `summary TEXT NOT NULL DEFAULT ''`, `participant_count INTEGER NOT NULL DEFAULT 1`.
- **FTS5 index:** `meetings_fts` virtual table indexing `meeting_id`, `title`, and `summary` for full-text search.
- **Search integration:** Add a third UNION branch to `search_captures` in `search.rs` that queries `meetings_fts`. Results returned with `result_type: "meeting"`.
- **Schema migration v5:** Bump `CURRENT_SCHEMA_VERSION` to 5. Add `meeting_id` columns, create `meetings` table, create `meetings_fts`.

### Non-Functional Requirements

- Meeting start/end must feel instant (< 200ms) -- no blocking on summary generation.
- Summary generation runs asynchronously after meeting end. User can continue using the app.
- Meeting mode should not noticeably increase CPU usage beyond the expected 2x capture rate.
- Summary generation timeout: 120 seconds via Ollama.

## Reusable Components

### Existing Code to Leverage

- **`capture.rs`** -- `SharedCaptureState` with `interval_secs`. Modify at runtime to switch between 5s (normal) and 2s (meeting).
- **`audio.rs`** -- `start_audio_worker` already processes pending WAV files. Meeting mode just needs to ensure WAV files are being produced.
- **`chat.rs`** -- `OLLAMA_BASE_URL`, `DEFAULT_MODEL`, reqwest client pattern, `OllamaRequest`/`OllamaResponse` structs. Reuse for summary generation.
- **`storage.rs`** -- Migration pattern, `insert_capture`, `insert_transcription`. Extend with meeting_id parameter.
- **`search.rs`** -- `search_captures` UNION pattern. Add meeting branch.

### New Code Required

- **`src-tauri/src/meeting.rs`** -- Meeting state management, start/end logic, summary generation.
- **Tray menu additions** in `lib.rs` -- "Start Meeting" / "End Meeting" items.
- **Migration v5** in `storage.rs` -- meetings table, meeting_id columns, meetings_fts.

## Technical Approach

### Meeting State

Store active meeting state alongside capture state:
```rust
pub struct MeetingState {
    pub active: bool,
    pub meeting_id: Option<String>,
    pub start_time: Option<String>,
}
```
Managed via `Arc<Mutex<MeetingState>>` in Tauri state, same pattern as `SharedCaptureState`.

### Tray Integration

Add menu items to the existing system tray setup in `lib.rs`. Use Tauri's `SystemTray` menu update API to swap between "Start Meeting" and "End Meeting" labels. On click, invoke the corresponding Tauri command.

### Summary Prompt

```
Summarize the following meeting transcript. Structure your summary as:

## Key Topics
- List the main topics discussed

## Decisions
- List any decisions that were made

## Action Items
- List any action items or next steps mentioned

Transcript:
{concatenated_transcription_text}
```

### Testing

- Unit test: migration v5 creates meetings table and adds meeting_id columns.
- Unit test: insert_meeting and query round-trip.
- Unit test: summary prompt construction with known transcript text.
- Integration test: start meeting -> insert captures with meeting_id -> end meeting -> verify meeting record exists with summary.

## Out of Scope

- Speaker diarization or participant detection
- Calendar integration or meeting scheduling
- Auto-detection of meeting applications
- Video recording
- Real-time transcription overlay during meetings
- Meeting sharing, export, or collaboration
- Meeting title auto-generation from content
- Meeting templates or recurring meeting support

## Success Criteria

- "Start Meeting" in tray activates 2-second capture interval and audio recording.
- "End Meeting" restores normal capture interval and generates a summary via Ollama.
- All captures and transcriptions during a meeting share the same meeting_id.
- Meeting summary is stored in the meetings table and indexed in meetings_fts.
- Searching for a topic mentioned in a meeting returns the meeting in results.
- Meeting start/end is instant; summary generation is non-blocking.
