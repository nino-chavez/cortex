# Spec Requirements: Local AI Chat

## Initial Description
Local AI Chat -- RAG-powered conversational interface that lets users ask natural language questions about their captured digital history. Uses Ollama (localhost API) for LLM inference, the existing fastembed + sqlite-vec pipeline for context retrieval, and streams grounded answers with citations linking back to specific captures in the Timeline View.

## Requirements Discussion

### First Round Questions

**Q1: Proof of Life**
**Answer:** User asks "what did I work on this morning?" and gets a grounded answer citing specific captures with timestamps and app names. The response includes clickable citation badges like `[Source: 10:45 AM - Chrome]` that link to the corresponding capture in Timeline View.

**Q2: LLM Runtime**
**Answer:** Use Ollama localhost API (not a bundled LLM). Ollama runs as an external service at `localhost:11434`. Check for Ollama on startup -- if not running, show a setup guidance UI prompting the user to install Ollama and run `ollama pull llama3.1`. This avoids bundling multi-GB model weights and lets users manage their own models.

**Q3: RAG Pipeline**
**Answer:** Query flows through: user message -> fastembed (embed query) -> sqlite-vec (top-K nearest neighbors) -> build prompt with retrieved context + metadata -> Ollama `/api/chat` endpoint -> streaming response. Pass both OCR text AND metadata (app name, window title, timestamp) to the LLM so answers are grounded in specific context.

**Q4: Citation Format**
**Answer:** Every AI claim must be backed by citations. Format: `[Source: {timestamp} - {app_name}]`. Citations render as hoverable badges/footnotes in the chat UI. Clicking a citation navigates to that specific capture in Timeline View. The LLM prompt instructs the model to include source references in its response.

**Q5: Chat UI Location**
**Answer:** Standard chat UI in the main Cortex window at `/chat` route. Not a separate window or overlay. Components: ChatPanel (container), MessageBubble (user/assistant messages), Citation (hoverable badge), StreamingResponse (token-by-token display). Markdown rendering for responses, code block support.

**Q6: Streaming**
**Answer:** Stream tokens from Ollama as they arrive. Use Tauri event system to push tokens to the frontend. Display partial response in real-time with a typing indicator. User sees the answer forming word-by-word.

**Q7: Ollama Availability**
**Answer:** Check Ollama availability on startup via `GET http://localhost:11434/api/tags`. Graceful degradation if not running -- chat route shows setup instructions instead of the chat interface. Re-check on user action (button click). Do not block app startup or other features if Ollama is unavailable.

**Q8: Out of Scope**
**Answer:** Multi-turn memory/conversation persistence, function calling, agents, fine-tuning, model training, cloud LLM fallback, voice input to chat, image input to chat. Keep it simple: single-turn RAG with streaming.

### Existing Code to Reference
- **embedding.rs** -- `EmbeddingEngine` with `embed_text()` returning 384-dim vectors via fastembed (all-MiniLM-L6-v2). Reuse directly for query embedding.
- **storage.rs** -- `Database` struct with `semantic_search_captures()` for sqlite-vec cosine similarity queries. `get_recent_captures()` and capture metadata retrieval.
- **search.rs** -- `SearchResult` struct with `capture_id`, `timestamp`, `app_name`, `snippet`, `image_path`, `result_type`. Extend or mirror for chat context retrieval.
- **storage.rs migrations** -- Schema at v4 with `vec_captures` and `vec_transcriptions` virtual tables. No new schema changes needed for chat.

## Requirements Summary

### Functional Requirements
- Ollama health check on startup via `GET localhost:11434/api/tags`
- RAG pipeline: embed query via fastembed -> sqlite-vec top-10 -> build prompt with context + metadata -> stream from Ollama
- Pass OCR text, app name, window title, and timestamp as context to the LLM
- System prompt instructs model to cite sources using `[Source: {timestamp} - {app_name}]` format
- Stream tokens from Ollama `/api/chat` endpoint to frontend via Tauri events
- Parse citation markers from LLM output and render as hoverable badges
- Click citation to navigate to capture in Timeline View
- Markdown rendering for assistant responses (headings, lists, code blocks, inline code)
- Setup guidance UI when Ollama is not available
- Graceful degradation -- rest of app works without Ollama

### Tauri Commands
- `chat_message(message: String)` -- Embeds query, retrieves context, streams response from Ollama via Tauri events
- `check_ollama_status()` -- Returns `OllamaStatus { available: bool, models: Vec<String> }`

### Frontend Routes & Components
- Route: `/chat` in main Cortex SvelteKit app
- Components: `ChatPanel`, `MessageBubble`, `Citation`, `StreamingResponse`

### Technical Stack
- **Ollama REST API** at `localhost:11434` (`/api/chat` with `stream: true`, `/api/tags` for health)
- **fastembed** (existing) for query embedding
- **sqlite-vec** (existing) for context retrieval
- **reqwest** for HTTP calls to Ollama
- **Tauri event system** for streaming tokens to frontend
- **SvelteKit** for chat UI components

### Scope Boundaries
**In Scope:**
- Ollama health check and status reporting
- RAG pipeline (embed -> search -> prompt -> stream)
- Streaming chat responses via Tauri events
- Citation extraction and rendering
- Chat UI at `/chat` route
- Markdown rendering in responses
- Ollama setup guidance UI
- Model detection (list available models)

**Out of Scope:**
- Multi-turn memory / conversation persistence across sessions
- Function calling or tool use
- Agents or autonomous workflows
- Fine-tuning or model training
- Cloud LLM fallback
- Voice or image input to chat
- Embedding new models (uses existing fastembed pipeline)
- Chat history database storage
