# Task Breakdown: Local AI Chat

## Overview
Total Tasks: 4 groups, 26 subtasks
Strategy: squad
Depth: standard
Assigned roles: backend-engineer (Ollama client + RAG pipeline), frontend-engineer (chat UI + streaming + citations), systems-engineer (integration + error handling), testing-engineer (pipeline tests + citation accuracy)

## Task List

### Proof of Life -- Vertical Slice

#### Task Group 1: Ollama Health Check + RAG Pipeline + Basic Chat UI + Citations
**Assigned implementer:** backend-engineer, frontend-engineer
**Dependencies:** Embedding & Semantic Search spec (Task Groups 1-3) must be complete -- `EmbeddingEngine`, `semantic_search_captures()`, `vec_captures` table, and fastembed pipeline exist and work.

This group delivers the Proof of Life scenario: user sends "what did I work on this morning?", the system embeds the query, retrieves relevant captures from sqlite-vec, builds a prompt with context and metadata, streams a response from Ollama, and displays it with citation badges. It is a vertical slice across Ollama integration, RAG retrieval, prompt construction, streaming, and citation rendering.

- [ ] 1.0 Complete minimal end-to-end RAG chat (query -> embed -> search -> prompt -> Ollama stream -> display with citations)
  - [ ] 1.1 Update `src-tauri/Cargo.toml` -- Add dependencies: `reqwest` (with `stream` and `json` features for Ollama HTTP calls), `futures-util` (for stream processing), `serde_json` (if not already present). These are the only new backend dependencies -- fastembed, rusqlite, sqlite-vec, and tokio are already in the project.
  - [ ] 1.2 Create `src-tauri/src/chat.rs` -- Implement core chat module:
    - `pub struct OllamaClient` with `base_url: String` (default `http://localhost:11434`).
    - `pub async fn check_health(&self) -> Result<OllamaStatus>` -- GET `/api/tags`, parse response to extract model names, return `OllamaStatus { available: bool, models: Vec<String> }`. Return `available: false` on connection error.
    - `pub async fn chat_stream(&self, model: &str, messages: Vec<ChatMessage>) -> Result<impl Stream<Item = Result<String>>>` -- POST `/api/chat` with `stream: true`. Return a stream of token strings parsed from Ollama's NDJSON response. Each line: `{"message":{"content":"token"},"done":false}`. Extract `message.content` from each line.
    - `pub struct ChatMessage { pub role: String, pub content: String }` -- Serializable message struct for Ollama API.
    - `pub struct OllamaStatus { pub available: bool, pub models: Vec<String> }` -- Serializable status struct.
  - [ ] 1.3 Implement RAG pipeline in `chat.rs`:
    - `pub struct RagPipeline` holding `Arc<EmbeddingEngine>` and `Arc<Database>`.
    - `pub fn retrieve_context(&self, query: &str, top_k: usize) -> Result<Vec<ChatContext>>` -- Embed query via `EmbeddingEngine::embed_text()`, call `Database::semantic_search_captures()` for top-K results, fetch full metadata (timestamp, app_name, window_title, OCR text) for each capture ID. Return `Vec<ChatContext>` where `ChatContext { capture_id, timestamp, app_name, window_title, ocr_text, distance }`.
    - `pub fn build_prompt(&self, contexts: &[ChatContext], user_message: &str) -> Vec<ChatMessage>` -- Construct system message with role description, citation instructions, and formatted context blocks. Append user message. Truncate context if total exceeds ~6000 tokens (rough estimate: 4 chars per token).
  - [ ] 1.4 Implement Tauri commands in `chat.rs` (or `lib.rs`):
    - `#[tauri::command] pub async fn chat_message(message: String, app_handle: AppHandle, ...)` -- Run RAG pipeline: retrieve context, build prompt, emit `chat:context` event with source metadata, stream from Ollama, emit `chat:token` for each token, emit `chat:done` on completion, emit `chat:error` on failure.
    - `#[tauri::command] pub async fn check_ollama_status(...)` -- Call `OllamaClient::check_health()`, return `OllamaStatus`.
  - [ ] 1.5 Register `chat` module and Tauri commands in `lib.rs` -- Add `mod chat;`, register `chat_message` and `check_ollama_status` in the Tauri command handler. Add `OllamaClient` and `RagPipeline` to Tauri managed state (initialized during app setup).
  - [ ] 1.6 Install frontend dependencies -- `npm install marked highlight.js` (or `svelte-markdown`) for markdown rendering in chat responses.
  - [ ] 1.7 Create `/chat` route -- Add `src/routes/chat/+page.svelte`. Basic layout: scrollable message area + fixed input bar at bottom. On mount, call `check_ollama_status` to determine initial state. If Ollama unavailable, show setup guidance. If available, show chat interface.
  - [ ] 1.8 Create `src/lib/components/ChatPanel.svelte` -- Container component:
    - `$state` for `messages` array (each: `{ role, content, sources?, streaming? }`), `isStreaming` flag, `ollamaStatus`.
    - Text input with send button. On send: push user message, invoke `chat_message` Tauri command, push empty assistant message with `streaming: true`.
    - Listen for Tauri events: `chat:context` (store sources on current message), `chat:token` (append to current assistant message content), `chat:done` (set `streaming: false`, trigger citation parsing), `chat:error` (display error).
    - Auto-scroll to bottom on new tokens.
  - [ ] 1.9 Create `src/lib/components/MessageBubble.svelte` -- Renders a single message:
    - User messages: right-aligned, accent-colored background, plain text.
    - Assistant messages: left-aligned, neutral background. During streaming: raw text with blinking cursor. After streaming: markdown-rendered content with citation badges.
  - [ ] 1.10 Create `src/lib/components/Citation.svelte` -- Citation badge component:
    - Accepts `timestamp`, `appName`, `captureId`, `windowTitle`, `snippet` props.
    - Default display: small pill showing `{time} - {appName}`.
    - Hover: expanded card with window title and OCR text snippet.
    - Click: navigate to `/timeline?capture={captureId}`.
  - [ ] 1.11 Implement citation parsing -- After stream completes, scan assistant message for `[Source: {timestamp} - {app_name}]` patterns via regex. Match each to a capture from the `chat:context` payload by timestamp and app name. Replace text markers with `Citation` component instances.
  - [ ] 1.12 Write 5 tests:
    - (a) `OllamaClient::check_health` returns `available: true` and model list when Ollama is running, `available: false` when connection refused.
    - (b) `RagPipeline::retrieve_context` returns captures ranked by cosine similarity with correct metadata fields.
    - (c) `build_prompt` constructs a well-formed prompt with system message, context blocks, and user message. Context blocks include timestamp, app name, and OCR text.
    - (d) `chat_stream` correctly parses Ollama NDJSON and yields individual tokens.
    - (e) Citation regex extracts `[Source: 10:45 AM - Chrome]` patterns and matches to known captures.

**Acceptance Criteria:**
- `cargo build --manifest-path src-tauri/Cargo.toml` compiles with reqwest and new chat module
- `check_ollama_status` returns correct availability and model list
- Sending "what did I work on this morning?" streams a grounded response with citation markers
- Citations render as hoverable badges linking to Timeline View
- Ollama not running shows setup guidance with installation steps
- All 5 tests pass

**Verification Steps:**
1. Start Ollama locally (`ollama serve`), pull llama3.1 (`ollama pull llama3.1`)
2. Run the app, navigate to `/chat`, send a message
3. Verify response streams token-by-token
4. Verify citation badges appear and are hoverable/clickable
5. Stop Ollama, refresh -- verify setup guidance appears

**Verification Commands:**
```bash
# Build
cargo build --manifest-path src-tauri/Cargo.toml

# Run backend tests
cargo test --manifest-path src-tauri/Cargo.toml --lib -- chat

# Verify Ollama is reachable
curl -s http://localhost:11434/api/tags | jq '.models[].name'

# Run the app
npm run tauri dev
```

---

### Chat UI Polish

#### Task Group 2: Message History, Markdown, Code Blocks, Citation Hover Cards, Loading States
**Assigned implementer:** frontend-engineer
**Dependencies:** Task Group 1 (basic chat UI, streaming, and citations work end-to-end)

- [ ] 2.0 Complete polished chat UI with markdown rendering, code blocks, citation hover cards, and loading states
  - [ ] 2.1 Implement markdown rendering in `MessageBubble.svelte` -- Use `marked` to parse assistant messages after streaming completes. Configure with GFM (GitHub Flavored Markdown) for tables and task lists. Sanitize HTML output to prevent XSS.
  - [ ] 2.2 Add syntax-highlighted code blocks -- Integrate `highlight.js` with `marked` renderer. Override the `code` renderer to wrap in `<pre><code class="hljs language-{lang}">`. Include language label above code blocks. Add a "copy" button on code blocks.
  - [ ] 2.3 Implement citation hover cards -- Expand `Citation.svelte` hover state: show a floating card (positioned above the badge) containing window title, full timestamp, OCR text snippet (first 200 chars), and a small screenshot thumbnail (loaded from `image_path` via Tauri asset protocol). Use `transition:fly` for smooth appearance.
  - [ ] 2.4 Add loading states -- Three states:
    - "Thinking..." with pulsing dots while waiting for first token (between send and first `chat:token` event).
    - Streaming indicator (blinking cursor at end of response) during token flow.
    - "Done" state: cursor disappears, markdown renders, citations become interactive.
  - [ ] 2.5 Implement session message history -- Messages persist within the current app session (in-memory array). Scrolling up shows previous messages. Clear chat button resets the message list. Messages are NOT persisted to database (out of scope).
  - [ ] 2.6 Add empty state -- When no messages exist, show a centered prompt with suggested questions: "What did I work on this morning?", "Summarize my last meeting", "What was I reading in Chrome yesterday?" Clicking a suggestion sends it as a message.
  - [ ] 2.7 Write 3 tests:
    - (a) Markdown renders headings, bold, code blocks, and lists correctly in assistant messages.
    - (b) Citation hover card appears on hover with correct metadata and disappears on mouse leave.
    - (c) Loading states transition correctly: empty -> thinking -> streaming -> done.

**Acceptance Criteria:**
- Assistant messages render full markdown with syntax-highlighted code blocks
- Code blocks have a working "copy" button
- Citation hover cards show window title, timestamp, snippet, and thumbnail
- Loading states provide clear feedback at each stage
- Empty state shows suggested questions
- Chat history scrollable within the session

**Verification Steps:**
1. Ask a coding question, verify code blocks render with syntax highlighting and copy button
2. Hover over a citation, verify the card appears with correct metadata
3. Verify loading states transition smoothly from thinking to streaming to done
4. Send multiple messages, scroll up to verify history is preserved

**Verification Commands:**
```bash
npm run tauri dev
# Manual testing in the /chat route
```

---

### Ollama Integration

#### Task Group 3: Model Detection, Setup Guidance UI, Error Handling, Model Switching
**Assigned implementer:** systems-engineer
**Dependencies:** Task Group 1 (OllamaClient and basic health check exist)

- [ ] 3.0 Complete robust Ollama integration with model management, setup guidance, and comprehensive error handling
  - [ ] 3.1 Create `src/lib/components/OllamaSetup.svelte` -- Full setup guidance component:
    - Icon and "Ollama is not running" heading.
    - Step-by-step instructions: (1) Install Ollama from ollama.ai, (2) Run `ollama serve` in terminal, (3) Run `ollama pull llama3.1`.
    - Copyable terminal commands (click to copy).
    - "Check Again" button that calls `check_ollama_status` and transitions to chat UI on success.
    - Animated status indicator: checking (spinner), not found (red), connected (green).
  - [ ] 3.2 Implement model detection and selection -- After health check returns available models, display the active model in the chat header. If `llama3.1` is not installed but other models are, show a dropdown to select an available model. Store selection in component state.
  - [ ] 3.3 Add error handling for Ollama mid-conversation failures -- Handle scenarios:
    - Ollama stops while streaming: catch stream error, show "Ollama connection lost" message in chat, offer "Retry" button.
    - Model not found: show error with suggestion to run `ollama pull {model}`.
    - Timeout (no first token in 30s): show timeout message with retry option.
    - Rate limiting / busy: show "Model is busy, retrying..." with exponential backoff (1s, 2s, 4s, max 3 retries).
  - [ ] 3.4 Add Ollama status indicator to chat header -- Small dot in the chat header: green when connected, yellow when checking, red when disconnected. Periodic health check every 60 seconds while on the `/chat` route.
  - [ ] 3.5 Write 3 tests:
    - (a) Setup guidance displays correct installation steps when Ollama is not available.
    - (b) Model selection dropdown populates with available models from health check response.
    - (c) Mid-stream error displays "connection lost" message and retry button.

**Acceptance Criteria:**
- Clear, actionable setup instructions when Ollama is not running
- Model selection works with any Ollama-installed model
- Mid-conversation Ollama failures show user-friendly error messages with retry options
- Status indicator accurately reflects Ollama connection state
- "Check Again" button works and transitions to chat UI on success

**Verification Steps:**
1. Start app without Ollama running -- verify setup guidance appears
2. Start Ollama, click "Check Again" -- verify transition to chat
3. Stop Ollama mid-response -- verify error message and retry button
4. Install multiple models, verify dropdown shows all models

**Verification Commands:**
```bash
# Test without Ollama
killall ollama 2>/dev/null; npm run tauri dev

# Test with Ollama
ollama serve & npm run tauri dev

# Test model listing
curl -s http://localhost:11434/api/tags | jq '.models[].name'

# Backend tests
cargo test --manifest-path src-tauri/Cargo.toml --lib -- chat::ollama
```

---

### Testing & Integration

#### Task Group 4: Pipeline Tests, Citation Accuracy, Streaming Reliability
**Assigned implementer:** testing-engineer
**Dependencies:** Task Groups 1, 2, 3

- [ ] 4.0 Complete test coverage for RAG pipeline, citation accuracy, and streaming reliability
  - [ ] 4.1 Review all tests from Groups 1-3 (11 total). Verify they compile and pass. Document any that are environment-dependent (e.g., tests requiring Ollama to be running, tests requiring captures in the database).
  - [ ] 4.2 Integration test: full RAG pipeline -- Insert 5 captures with known OCR text and pre-computed embeddings into the database. Send a chat message via `chat_message` Tauri command. Collect all emitted events. Assert: (a) `chat:context` contains the correct source captures, (b) `chat:token` events arrive in order, (c) `chat:done` fires, (d) assembled response contains citation markers matching the source captures.
  - [ ] 4.3 Integration test: citation accuracy -- Insert captures with distinct timestamps and app names. Send a query that should reference specific captures. Parse the response for `[Source: ...]` markers. Assert each citation matches a real capture by timestamp and app name. Assert no hallucinated citations (citations referencing captures that don't exist in the context).
  - [ ] 4.4 Integration test: streaming reliability -- Send 10 sequential chat messages. For each, verify: (a) `chat:token` events arrive without gaps, (b) `chat:done` fires exactly once, (c) no `chat:error` events, (d) total tokens received match the expected response length (within tolerance). Test with varying query lengths (short: 5 words, medium: 20 words, long: 100 words).
  - [ ] 4.5 Integration test: Ollama unavailable graceful degradation -- With Ollama stopped, call `chat_message`. Assert: (a) `chat:error` event fires with a descriptive message, (b) no crash or hang, (c) subsequent calls to `check_ollama_status` still work. Start Ollama, call `chat_message` again, assert it works.
  - [ ] 4.6 Integration test: context retrieval quality -- Insert 20 captures spanning different apps and topics. Query with "what was I doing in Chrome?" Assert: (a) top results are Chrome captures, (b) results are ordered by relevance, (c) non-Chrome captures only appear if semantically relevant to browsing. Query with "summarize my meetings" -- assert Zoom/Meet captures rank highest.
  - [ ] 4.7 Gap analysis -- Document untested paths: concurrent chat requests, very long responses (>4096 tokens), Ollama model swap mid-conversation, captures with empty OCR text in context, unicode/emoji in chat messages, network latency simulation, memory usage during extended chat sessions, prompt injection attempts. File as future test TODOs in a comment block.

**Acceptance Criteria:**
- All tests from Groups 1-3 pass
- 5 new integration tests added and passing
- Gap analysis identifies at least 5 untested edge cases
- Total test count: 16+ (11 from Groups 1-3 + 5 new)
- Citation accuracy: zero hallucinated citations in test scenarios
- Streaming reliability: zero dropped tokens or missed events across 10 sequential messages

**Verification Steps:**
1. Run full test suite -- expect all tests pass
2. Review test output for flaky tests or warnings
3. Manually test the Proof of Life scenario end-to-end

**Verification Commands:**
```bash
# Run all tests
cargo test --manifest-path src-tauri/Cargo.toml --lib

# Run chat-specific tests
cargo test --manifest-path src-tauri/Cargo.toml --lib -- chat

# Run tests with output for debugging
cargo test --manifest-path src-tauri/Cargo.toml --lib -- --nocapture

# List all tests
cargo test --manifest-path src-tauri/Cargo.toml --lib -- --list 2>&1 | tail -1

# Build release to verify no compile warnings
cargo build --manifest-path src-tauri/Cargo.toml --release 2>&1 | grep warning
```

---

## Execution Order

1. **Task Group 1: Proof of Life** (backend-engineer + frontend-engineer) -- Vertical slice. Must complete first. Delivers: "User sends message, gets streamed RAG response with citation badges."
2. **Task Group 2: Chat UI Polish** (frontend-engineer) -- Depends on Group 1 for basic chat working. Adds markdown, code blocks, hover cards, loading states.
3. **Task Group 3: Ollama Integration** (systems-engineer) -- Depends on Group 1 for OllamaClient. Can partially overlap with Group 2: setup guidance (3.1) only needs the health check from Group 1, while error handling (3.3) benefits from Group 2's UI polish.
4. **Task Group 4: Testing & Integration** (testing-engineer) -- Depends on all prior groups completing.

**Parallel execution possible:** Groups 2 and 3 can partially overlap after Group 1 completes -- setup guidance UI (3.1) and markdown rendering (2.1-2.2) have no dependencies on each other.
