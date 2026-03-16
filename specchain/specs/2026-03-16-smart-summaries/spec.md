# Specification: Smart Summaries

## Goal

Provide three on-demand AI summary commands that let users generate natural language summaries of their captured history by time period, application, or topic. Composes existing Ollama client, database queries, and embedding/search infrastructure -- no new ML pipelines required.

## Proof of Life

**Scenario:** User opens the summary panel, clicks "Summarize my morning". The system queries all captures and transcriptions from 8:00 AM to 12:00 PM today, builds a summarization prompt with the collected OCR text and metadata, calls Ollama, and displays a markdown response: "This morning you spent most of your time in Cursor working on the meeting memory feature [Source: 9:15 AM - Cursor], reviewed two PRs in Chrome [Source: 10:22 AM - Chrome], and had a 30-minute standup on Zoom [Source: 11:00 AM - Zoom]."

**Validates:** The period summary command correctly queries the database, builds a usable prompt, calls Ollama, and returns a citation-backed markdown summary.

**Must work before:** Any automated or scheduled summary features.

## User Stories

- As a user, I want to summarize a time period so I can quickly recall what I did during my morning, afternoon, or full day.
- As a user, I want to summarize my activity in a specific application so I can see what I accomplished in VS Code or Chrome today.
- As a user, I want to summarize everything related to a topic so I can get a comprehensive view of a project or theme across all apps and time.
- As a user, I want citations in summaries so I can jump to the original captures for full context.

## Core Requirements

### Functional Requirements

- **`summarize_period(from, to)`:**
  1. Query `captures` WHERE timestamp BETWEEN from AND to. Fetch OCR text from `captures_fts`.
  2. Query `transcriptions` WHERE timestamp_start BETWEEN from AND to.
  3. Concatenate context (OCR text + transcription text) with metadata (timestamp, app_name).
  4. Build prompt: "Summarize the user's activity during this time period. Group by application and major task. Cite sources."
  5. Call Ollama `/api/generate` (non-streaming, 120s timeout).
  6. Parse citations from response, return `SummaryResponse`.

- **`summarize_app(app_name, date)`:**
  1. Query `captures` WHERE app_name = ? AND date(timestamp) = ?. Fetch OCR text.
  2. Build prompt: "Summarize what the user did in {app_name} on {date}. Focus on tasks, documents viewed, and key actions. Cite sources."
  3. Call Ollama, return `SummaryResponse`.

- **`summarize_topic(topic)`:**
  1. Embed the topic string via `EmbeddingEngine::embed_text()`.
  2. Semantic search via `Database::semantic_search_captures()` for top-20 results.
  3. Fetch metadata and OCR text for each result.
  4. Build prompt: "Summarize everything related to '{topic}' from the user's history. Include chronological context. Cite sources."
  5. Call Ollama, return `SummaryResponse`.

- **Response format:** `SummaryResponse { markdown: String, citations: Vec<Citation> }`. Reuse the `Citation` struct from `chat.rs`. The markdown includes `[Source: timestamp - app_name]` markers that map to the citations vector.

- **Empty results:** If no captures/transcriptions found for the query, return a friendly message ("No captures found for this time period") without calling Ollama.

- **Context limits:** Cap context at ~6000 tokens (~24000 chars). If the collected content exceeds this, truncate oldest entries for period/app summaries, or lowest-ranked entries for topic summaries.

### Non-Functional Requirements

- Summary generation should complete within 30 seconds for typical queries (excluding Ollama model cold-start).
- Database queries for context collection should complete in under 1 second.
- Summary commands must not block the UI -- run async.

## Reusable Components

### Existing Code to Leverage

- **`chat.rs`** -- `OLLAMA_BASE_URL`, `DEFAULT_MODEL`, `Citation` struct, `ChatResponse` struct (adapt to `SummaryResponse`), reqwest blocking client with `/api/generate`, `build_rag_prompt` (adapt for summarization prompts).
- **`storage.rs`** -- `get_captures_for_day`, `get_captures_by_app`, `get_capture_ocr_text`, `semantic_search_captures`. Direct database access for period and app queries.
- **`embedding.rs`** -- `EmbeddingEngine::embed_text` for topic summaries.
- **`search.rs`** -- `SearchResult` struct as reference for result shape.

### New Code Required

- **`src-tauri/src/summary.rs`** -- Three public functions + Tauri command wrappers. `SummaryResponse` struct. Prompt builders for each summary type.
- **`src/routes/summary/+page.svelte`** -- Summary panel UI.
- **`src/lib/components/SummaryPanel.svelte`** -- Summary display with markdown rendering and citations.

## Technical Approach

### Module Structure

`summary.rs` contains:
- `SummaryResponse { markdown: String, citations: Vec<Citation> }` (reuses `Citation` from chat.rs)
- `fn collect_period_context(db, from, to) -> Vec<ContextBlock>`
- `fn collect_app_context(db, app_name, date) -> Vec<ContextBlock>`
- `fn collect_topic_context(db, engine, topic) -> Vec<ContextBlock>`
- `fn build_summary_prompt(context_type, contexts) -> String`
- `fn call_ollama_summary(prompt) -> Result<String>`
- Three `#[tauri::command]` functions wrapping the above.

### Prompt Templates

Period:
```
Summarize the user's computer activity from {from} to {to}.
Group the summary by application and major task.
For every claim, cite the source using [Source: {timestamp} - {app_name}].

Context:
{formatted_context_blocks}
```

App:
```
Summarize what the user did in {app_name} on {date}.
Focus on specific tasks, documents, code, and key actions.
Cite sources using [Source: {timestamp} - {app_name}].

Context:
{formatted_context_blocks}
```

Topic:
```
Summarize everything related to "{topic}" from the user's captured history.
Provide chronological context showing how this topic developed over time.
Cite sources using [Source: {timestamp} - {app_name}].

Context:
{formatted_context_blocks}
```

### Testing

- Unit test: `collect_period_context` returns captures in the specified time range.
- Unit test: `collect_app_context` filters by app name and date.
- Unit test: `build_summary_prompt` includes all context blocks and the correct instruction.
- Unit test: Empty context returns friendly message without calling Ollama.
- Integration test: Full pipeline with known captures produces a summary containing expected citations.

## Out of Scope

- Streaming summaries (returns complete response)
- Summary persistence or history tracking
- Scheduled or automatic summaries
- Custom prompt editing by users
- Summary comparison across time periods
- Export or sharing of summaries

## Success Criteria

- `summarize_period` returns a markdown summary covering the specified time range with correct citations.
- `summarize_app` returns a focused summary for the specified app and date.
- `summarize_topic` uses semantic search and returns a topically coherent summary.
- All summaries include `[Source: timestamp - app_name]` citation markers.
- Empty queries return a friendly message without calling Ollama.
- Summary UI renders markdown with clickable citation badges.
