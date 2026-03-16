# Task Breakdown: Smart Summaries

## Overview
Total Tasks: 2 groups, 13 subtasks
Strategy: squad
Depth: standard

## Task List

### Proof of Life -- Backend

#### Task Group 1: Three Summary Tauri Commands
**Dependencies:** Local AI Chat Task Group 1 (Ollama client pattern in chat.rs), Embedding & Semantic Search (EmbeddingEngine, semantic_search_captures)

This group delivers three working Tauri commands that generate summaries by querying the database, building tailored prompts, and calling Ollama. No UI yet -- commands can be tested via Tauri invoke or Rust tests.

- [ ] 1.0 Complete three summary commands reusing chat.rs patterns
  - [ ] 1.1 Create `src-tauri/src/summary.rs` with shared types and helpers:
    - `pub struct SummaryResponse { pub markdown: String, pub citations: Vec<Citation> }` (reuse `Citation` from `chat.rs`).
    - `struct ContextBlock { capture_id: i64, timestamp: String, app_name: String, text: String }` -- internal type for collected context.
    - `fn format_context_blocks(blocks: &[ContextBlock]) -> String` -- formats blocks as numbered entries with timestamp and app metadata.
    - `fn call_ollama_summary(prompt: &str) -> Result<String, String>` -- calls Ollama `/api/generate` with non-streaming request, 120s timeout. Reuses reqwest blocking client pattern from `chat.rs`.
    - `fn parse_citations(text: &str, blocks: &[ContextBlock]) -> Vec<Citation>` -- extracts `[Source: ...]` patterns and maps to Citation structs.
  - [ ] 1.2 Implement `summarize_period`:
    - `fn collect_period_context(db: &Database, from: &str, to: &str) -> Vec<ContextBlock>` -- query captures with OCR text in time range + transcriptions in time range. Cap at ~50 entries.
    - Build prompt with period template.
    - Call Ollama, parse citations, return `SummaryResponse`.
    - Handle empty results: return `SummaryResponse { markdown: "No captures found...", citations: vec![] }`.
  - [ ] 1.3 Implement `summarize_app`:
    - `fn collect_app_context(db: &Database, app_name: &str, date: &str) -> Vec<ContextBlock>` -- query captures WHERE app_name = ? and timestamp within the date.
    - Build prompt with app template.
    - Call Ollama, parse citations, return `SummaryResponse`.
  - [ ] 1.4 Implement `summarize_topic`:
    - `fn collect_topic_context(db: &Database, engine: &EmbeddingEngine, topic: &str) -> Vec<ContextBlock>` -- embed topic, semantic search top-20, fetch metadata and OCR text.
    - Build prompt with topic template.
    - Call Ollama, parse citations, return `SummaryResponse`.
  - [ ] 1.5 Register Tauri commands -- Add `mod summary;` in `lib.rs`. Register three commands: `summarize_period`, `summarize_app`, `summarize_topic`. Each takes appropriate parameters and returns `SummaryResponse`.
  - [ ] 1.6 Add helper query to `storage.rs` -- `get_captures_in_range(from: &str, to: &str) -> Result<Vec<CaptureRow>>` if not already covered by existing methods. This queries captures with OCR text joined from captures_fts.
  - [ ] 1.7 Write 4 tests:
    - (a) `collect_period_context` returns captures within the specified time range and excludes captures outside it.
    - (b) `collect_app_context` filters by app_name and date correctly.
    - (c) `build_summary_prompt` for each type includes the correct instruction and all context blocks.
    - (d) Empty context returns friendly message without calling Ollama.

**Acceptance Criteria:**
- `cargo build` compiles with new summary module
- `summarize_period` returns markdown with citations for a time range
- `summarize_app` returns markdown filtered to a specific application
- `summarize_topic` uses semantic search and returns topically relevant summary
- Empty queries handled gracefully
- All 4 tests pass

**Verification Commands:**
```bash
cargo build --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml --lib -- summary
```

---

### Summary UI

#### Task Group 2: Summary Panel + Integration with Timeline
**Dependencies:** Task Group 1 (summary commands work)

- [ ] 2.0 Complete summary UI panel with preset buttons, topic input, and citation rendering
  - [ ] 2.1 Create `/summary` route -- `src/routes/summary/+page.svelte`. Layout: header with title, preset buttons row, topic input, and summary display area.
  - [ ] 2.2 Create `src/lib/components/SummaryPanel.svelte`:
    - Preset buttons: "This Morning" (8am-12pm), "This Afternoon" (12pm-5pm), "Today" (full day). Each invokes `summarize_period` with computed timestamps.
    - App summary dropdown: populated from `get_distinct_apps()`, selecting an app invokes `summarize_app(app, today)`.
    - Topic input: text field + "Summarize" button, invokes `summarize_topic(input)`.
    - Display area: renders returned markdown with citation badges (reuse `Citation.svelte` from chat).
    - Loading state: spinner while waiting for Ollama response.
    - Error state: message when Ollama is unavailable.
  - [ ] 2.3 Add "Summarize this day" button to Timeline view -- On the Timeline page, add a button that invokes `summarize_period` for the currently viewed day and displays the result in a slide-over or modal panel.
  - [ ] 2.4 Add "Summaries" to sidebar navigation.
  - [ ] 2.5 Wire citation clicks to Timeline -- Clicking a citation in a summary navigates to `/timeline?capture={capture_id}`, reusing the same citation navigation from chat.
  - [ ] 2.6 Write 2 tests:
    - (a) Preset buttons compute correct time ranges for "This Morning" and "Today".
    - (b) Summary panel renders markdown and citation badges from a mock SummaryResponse.

**Acceptance Criteria:**
- Summary panel accessible from sidebar navigation
- Preset buttons generate time-based summaries
- App dropdown generates app-specific summaries
- Topic input generates semantic summaries
- Citations are clickable and navigate to Timeline
- "Summarize this day" button works on Timeline view

**Verification Commands:**
```bash
npm run tauri dev
# Navigate to /summary, test preset buttons and topic input
```

---

## Execution Order

1. **Task Group 1: Backend Commands** -- Must complete first. Delivers the three summary commands.
2. **Task Group 2: Summary UI** -- Depends on Group 1 for working commands. Delivers the frontend panel and Timeline integration.
