# Spec Requirements: Smart Summaries

## Initial Description
Smart Summaries -- Tauri commands that generate on-demand AI summaries of captured digital history. Three summary types: time period ("summarize my morning"), application ("what did I do in VS Code today"), and topic ("everything about the auth refactor"). Reuses the existing chat.rs Ollama client and search infrastructure. Returns markdown summaries with citations.

## Requirements Discussion

### First Round Questions

**Q1: Proof of Life**
**Answer:** User invokes `summarize_period("2026-03-16T08:00:00", "2026-03-16T12:00:00")`. The system queries captures and transcriptions in that time range, builds a prompt with the collected OCR text and metadata, calls Ollama, and returns a markdown summary like: "This morning you worked on the Cortex capture daemon in Cursor [Source: 9:15 AM - Cursor], reviewed a PR in Chrome [Source: 10:22 AM - Chrome]..."

**Q2: Summary Types**
**Answer:** Three distinct Tauri commands, each with a different query strategy:
- `summarize_period(from, to)` -- queries captures/transcriptions by timestamp range.
- `summarize_app(app_name, date)` -- queries captures WHERE app_name = ? AND date = ?.
- `summarize_topic(topic)` -- uses semantic search (embed topic -> sqlite-vec) to find relevant content.
All three follow the same pipeline: query -> collect context -> build prompt -> Ollama -> return markdown.

**Q3: Prompt Design**
**Answer:** Each summary type has a tailored system prompt:
- Period: "Summarize the user's activity during this time period. Group by application and topic."
- App: "Summarize what the user did in {app_name} on {date}. Focus on tasks, documents, and key actions."
- Topic: "Summarize everything related to '{topic}' from the user's captured history. Include timeline context."
All prompts instruct the model to include `[Source: {timestamp} - {app_name}]` citations.

**Q4: Relationship to Chat**
**Answer:** Smart Summaries reuse the Ollama client and citation format from chat.rs but are NOT chat messages. They are standalone Tauri commands that return a complete response (not streamed). The summary UI is a separate panel, not part of the chat conversation.

**Q5: Out of Scope**
**Answer:** Streaming summaries, scheduled/automatic summaries, summary persistence/history, summary sharing, custom prompt editing, summary comparison across days.

### Existing Code to Reference
- **chat.rs** -- `OLLAMA_BASE_URL`, `DEFAULT_MODEL`, `ChatResponse`, `Citation`, `build_rag_prompt`, reqwest client pattern. Reuse Ollama call pattern; adapt prompt for summarization.
- **storage.rs** -- `get_captures_for_day`, `get_captures_by_app`, `get_recent_captures`, `get_capture_ocr_text`. Query methods for period and app summaries.
- **embedding.rs** -- `EmbeddingEngine::embed_text` for topic summary (semantic search).
- **search.rs** -- `search_captures` with time range filters. Alternative to direct DB queries.

## Requirements Summary

### Functional Requirements
- `summarize_period(from: String, to: String)` -- Collects captures + transcriptions in time range, summarizes via Ollama
- `summarize_app(app_name: String, date: String)` -- Collects captures for app on date, summarizes via Ollama
- `summarize_topic(topic: String)` -- Semantic search for topic, summarizes top results via Ollama
- Each returns markdown with `[Source: timestamp - app_name]` citations
- Non-streaming (returns complete response)
- Graceful error when Ollama unavailable or no captures found

### Tauri Commands
- `summarize_period(from: String, to: String)` -- Returns `SummaryResponse { markdown: String, citations: Vec<Citation> }`
- `summarize_app(app_name: String, date: String)` -- Returns `SummaryResponse`
- `summarize_topic(topic: String)` -- Returns `SummaryResponse`

### Frontend
- Summary panel accessible from Timeline view and main navigation
- Displays markdown-rendered summary with citation badges
- Preset buttons: "Summarize this morning", "Summarize today", "Summarize this app"
- Topic input field for free-form topic summaries

### Scope Boundaries
**In Scope:**
- Three summary Tauri commands
- Tailored prompts per summary type
- Markdown response with citations
- Summary panel UI
- Preset summary shortcuts

**Out of Scope:**
- Streaming summaries
- Summary persistence or history
- Scheduled/automatic summaries
- Custom prompt editing
- Summary comparison or diffing
- Export or sharing
