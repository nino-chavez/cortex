# Specification: Local AI Chat

## Goal

Provide a RAG-powered conversational interface that lets users ask natural language questions about their captured digital history and receive grounded, citation-backed answers streamed in real-time. Uses Ollama (localhost API) for LLM inference and the existing fastembed + sqlite-vec pipeline for context retrieval, keeping the entire pipeline local and private.

## Proof of Life

**Scenario:** User opens the `/chat` route and types "what did I work on this morning?" The system embeds the query, retrieves the top-10 most relevant captures from sqlite-vec, builds a prompt with the retrieved OCR text and metadata, sends it to Ollama, and streams a response like: "This morning you were working on the Cortex capture daemon in Cursor [Source: 9:15 AM - Cursor], then reviewed a pull request on GitHub in Chrome [Source: 10:22 AM - Chrome], and had a meeting on Zoom [Source: 11:00 AM - Zoom]." Each citation badge is hoverable and clickable, linking to the specific capture in Timeline View.

**Validates:** The full RAG pipeline works end-to-end -- query embedding, vector retrieval, prompt construction with metadata, Ollama streaming, citation extraction, and interactive citation rendering.

**Must work before:** Smart Summaries, Meeting Memory AI features, or any advanced chat capabilities.

## User Stories

- As a user, I want to ask natural language questions about my digital history and get answers grounded in my actual captures, so I can recall what I was doing without manually searching.
- As a user, I want every AI claim to include a citation linking to the specific capture, so I can verify the answer and see the original context.
- As a user, I want to see the response stream in real-time token-by-token, so I don't have to wait for the full response before reading.
- As a user, I want to click a citation badge to jump to that capture in the Timeline View, so I can see the full screenshot and surrounding context.
- As a user, I want clear instructions if Ollama is not installed or running, so I can set it up and start chatting.
- As a user, I want the chat to render markdown formatting including code blocks, so technical answers are readable.

## Core Requirements

### Functional Requirements

- **Ollama health check:** On app startup, send `GET http://localhost:11434/api/tags` to detect Ollama availability and list installed models. Cache the result. Expose via `check_ollama_status()` Tauri command returning `OllamaStatus { available: bool, models: Vec<String> }`. Re-check on user action.
- **RAG pipeline:** When the user sends a message:
  1. Embed the query using `EmbeddingEngine::embed_text()` (existing fastembed, 384-dim).
  2. Query `vec_captures` via `Database::semantic_search_captures()` for top-10 nearest neighbors by cosine distance.
  3. For each result, fetch full capture metadata: `timestamp`, `app_name`, `window_title`, and OCR text from `captures_fts`.
  4. Build a prompt with a system message instructing the model to answer based only on the provided context and cite sources, followed by the retrieved context blocks, followed by the user's question.
  5. Send to Ollama `POST http://localhost:11434/api/chat` with `stream: true`.
  6. Read the NDJSON stream, extract each token, emit via Tauri event `chat:token` to the frontend.
  7. On stream completion, emit `chat:done` event.
- **Prompt construction:** The system prompt includes:
  - Role: "You are Cortex, a local AI assistant that answers questions based on the user's screen captures and audio transcriptions."
  - Instruction: "Answer based ONLY on the provided context. For every claim, cite the source using the format [Source: {timestamp} - {app_name}]. If the context does not contain enough information, say so."
  - Context block: Each retrieved capture formatted as `[{timestamp}] [{app_name} - {window_title}]\n{ocr_text}\n---`
  - User message appended last.
- **Streaming:** Use `reqwest` with streaming response. Parse each line of Ollama's NDJSON response (`{"message":{"content":"token"},"done":false}`). Emit each content token as a Tauri event. Frontend appends tokens to the displayed response in real-time.
- **Citation extraction:** Parse `[Source: ...]` patterns from the LLM response. Match each citation back to a capture by timestamp and app name. Render as hoverable badges showing the capture's window title and a thumbnail. Click navigates to `/timeline?capture={capture_id}`.
- **Ollama setup guidance:** When `check_ollama_status()` returns `available: false`, the `/chat` route displays:
  1. "Ollama is not running" message.
  2. Link to `https://ollama.ai` for installation.
  3. Terminal command to copy: `ollama pull llama3.1`.
  4. "Check again" button that re-runs the health check.
- **Markdown rendering:** Assistant messages render full markdown: headings, bold, italic, lists, inline code, fenced code blocks with syntax highlighting. Use a Svelte markdown component (e.g., `svelte-markdown` or `marked` + `highlight.js`).
- **Model selection:** Default to `llama3.1`. If not available, show available models from `/api/tags` and let the user pick. Store selection in app state (not persisted across restarts for now).

### Non-Functional Requirements

- Chat response should begin streaming within 2 seconds of sending a message (excluding model cold-start).
- RAG context retrieval (embed + search) should complete in under 500ms.
- Chat UI must remain responsive during streaming -- no frozen frames.
- Ollama health check must not block app startup -- run asynchronously.
- Chat should work with any Ollama-compatible model, not just llama3.1.
- The chat feature must not affect capture daemon or OCR pipeline performance.

## Visual Design

### Chat Route (`/chat`)

The chat interface occupies the main content area of the Cortex window, consistent with the existing app layout (sidebar navigation + main content).

**Layout:**
- Full-height scrollable message area with alternating user/assistant message bubbles.
- User messages: right-aligned, styled with the app's accent color.
- Assistant messages: left-aligned, neutral background, full markdown rendering.
- Citations inline within assistant messages as small pill-shaped badges.
- Fixed input bar at the bottom: text input + send button.
- Loading state: pulsing dot indicator while waiting for first token.
- Streaming state: cursor/caret at the end of the partially rendered response.

**Citation Badge:**
- Inline pill: `10:45 AM - Chrome` with a subtle background.
- Hover: expanded card showing window title, OCR text snippet, and screenshot thumbnail.
- Click: navigates to `/timeline?capture={id}`.

**Ollama Not Available State:**
- Centered card with icon, "Ollama is not running" heading, installation steps, and "Check Again" button.

## Conversion Design

Not applicable -- this is a new feature route, not a conversion funnel.

## Reusable Components

### Existing Code to Leverage

- **`embedding.rs`** -- `EmbeddingEngine` struct with `embed_text(&str) -> Option<Vec<f32>>`. Used directly to embed the user's query. No changes needed.
- **`storage.rs`** -- `Database::semantic_search_captures(&[f32], limit) -> Vec<(i64, f64)>` for vector similarity search. `get_recent_captures()` for metadata retrieval. Connection pool with `Mutex<Connection>`.
- **`search.rs`** -- `SearchResult` struct for result shape. `search_captures()` for FTS5 fallback if semantic search returns too few results.
- **Tauri event system** -- `app_handle.emit()` for pushing streaming tokens to the frontend. Already used in other parts of the app.

### New Components Required

- **`src-tauri/src/chat.rs`** -- Rust module containing:
  - `OllamaClient` struct: HTTP client for Ollama API. Methods: `check_health()`, `chat_stream()`.
  - `RagPipeline` struct: Orchestrates embed -> search -> prompt -> stream. Holds references to `EmbeddingEngine` and `Database`.
  - `build_prompt()` function: Constructs the system + context + user prompt.
  - `parse_citation()` function: Extracts `[Source: ...]` markers from response text.
- **`src/routes/chat/+page.svelte`** -- SvelteKit chat route.
- **`src/lib/components/ChatPanel.svelte`** -- Container component managing message list, scroll behavior, and input bar.
- **`src/lib/components/MessageBubble.svelte`** -- Individual message component. Renders user messages as plain text, assistant messages as markdown with citations.
- **`src/lib/components/Citation.svelte`** -- Citation badge component. Pill display, hover card with metadata, click-to-navigate.
- **`src/lib/components/StreamingResponse.svelte`** -- Handles token-by-token rendering during streaming. Shows cursor/caret, transitions to static markdown when stream completes.
- **`src/lib/components/OllamaSetup.svelte`** -- Setup guidance displayed when Ollama is not available.

## Technical Approach

### Ollama Integration

- **Health check:** `GET http://localhost:11434/api/tags` returns `{"models":[{"name":"llama3.1:latest",...}]}`. Parse to extract model names. If request fails (connection refused), Ollama is not running.
- **Chat endpoint:** `POST http://localhost:11434/api/chat` with body:
  ```json
  {
    "model": "llama3.1",
    "messages": [
      {"role": "system", "content": "...system prompt with context..."},
      {"role": "user", "content": "...user question..."}
    ],
    "stream": true
  }
  ```
- **Stream parsing:** Ollama returns NDJSON. Each line: `{"message":{"role":"assistant","content":"token"},"done":false}`. Final line: `{"message":{"role":"assistant","content":""},"done":true}`. Parse each line, extract `message.content`, emit as Tauri event.
- **HTTP client:** Use `reqwest` with `Response::bytes_stream()` for streaming. Process chunks as they arrive. Handle connection errors, timeouts (30s for first token, 120s total), and model not found errors.

### RAG Pipeline

- **Query embedding:** Call `EmbeddingEngine::embed_text(user_message)` to get a 384-dim vector.
- **Context retrieval:** Call `Database::semantic_search_captures(query_vec, 10)` to get top-10 capture IDs ranked by cosine similarity.
- **Metadata enrichment:** For each capture ID, query the `captures` table for `timestamp`, `app_name`, `window_title`, and join to `captures_fts` for the OCR text. Query `transcriptions` table for any transcriptions linked to the capture.
- **Context formatting:** Each context block:
  ```
  [2026-03-16 09:15:23] [Cursor - main.rs -- cortex]
  fn start_capture_daemon() {
      let config = load_config();
      ...
  }
  ---
  ```
- **Prompt assembly:** System prompt + formatted context blocks + user message. Total prompt should stay under the model's context window (8192 tokens for llama3.1 default). If context exceeds ~6000 tokens, truncate the lowest-ranked results.

### Tauri Commands

- **`chat_message(message: String)`** -- Async Tauri command. Runs the RAG pipeline, streams tokens via events. Does not return the full response (frontend builds it from events). Returns `Result<(), String>` for error handling.
  - Events emitted:
    - `chat:token` with payload `{ "token": "..." }` for each token.
    - `chat:context` with payload `{ "sources": [...] }` at the start, containing the retrieved captures used as context.
    - `chat:done` with empty payload when stream completes.
    - `chat:error` with payload `{ "error": "..." }` on failure.
- **`check_ollama_status()`** -- Sync Tauri command. Returns `OllamaStatus { available: bool, models: Vec<String> }`.

### Frontend Architecture

- **State management:** Svelte 5 runes. `$state` for message list, streaming status, Ollama status. No external state library.
- **Event listeners:** On mount, listen for `chat:token`, `chat:context`, `chat:done`, `chat:error` Tauri events. Append tokens to the current assistant message. On done, finalize the message and parse citations.
- **Scroll behavior:** Auto-scroll to bottom on new tokens. Stop auto-scrolling if user scrolls up manually. Resume on new user message.
- **Markdown rendering:** Use `marked` for markdown parsing + `highlight.js` for code syntax highlighting. Render after stream completes (during streaming, show raw text with a cursor to avoid re-rendering markdown on every token).
- **Citation parsing:** After stream completes, scan the response for `[Source: {timestamp} - {app_name}]` patterns. Replace with `<Citation>` components. Match to captures from the `chat:context` event payload.

### Testing

- **Unit tests for Ollama client:** Mock HTTP responses. Test health check parsing, stream parsing, error handling (connection refused, timeout, invalid JSON).
- **Unit tests for RAG pipeline:** Mock embedding engine and database. Test prompt construction with known context. Verify context truncation when exceeding token limit.
- **Unit tests for citation parsing:** Test extraction of `[Source: ...]` patterns from various response formats. Test matching citations to captures by timestamp and app name.
- **Integration test for full pipeline:** Insert captures with known OCR text and embeddings. Send a chat message. Verify the response references the correct captures. Verify citation markers are present.
- **Frontend tests:** Verify streaming token display, citation rendering, Ollama setup UI visibility, markdown rendering.

## Out of Scope

- Multi-turn memory / conversation persistence across sessions
- Function calling or tool use
- Agents or autonomous workflows
- Fine-tuning or model training
- Cloud LLM fallback (OpenAI, Anthropic, etc.)
- Voice input to chat
- Image/screenshot input to chat (multi-modal)
- Chat history database storage
- Model downloading or management (Ollama handles this)
- Hybrid search (RRF) for context retrieval
- Custom system prompts or prompt editing UI

## Success Criteria

- User asks "what did I work on this morning?" and receives a grounded answer citing 2+ specific captures with correct timestamps and app names.
- Citation badges are hoverable (showing metadata) and clickable (navigating to Timeline View).
- Response streams token-by-token with no frozen UI frames.
- RAG context retrieval completes in under 500ms.
- First token appears within 2 seconds of sending (excluding model cold-start).
- When Ollama is not running, chat route shows clear setup instructions and a working "Check Again" button.
- Markdown formatting renders correctly: headings, code blocks, lists, inline code.
- Chat does not degrade capture daemon or OCR pipeline performance.
