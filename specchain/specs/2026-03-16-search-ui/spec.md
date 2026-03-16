# Specification: Search UI

## Goal

Build a Raycast-style floating search overlay that lets the user instantly search their entire captured history via a global hotkey (Cmd+Shift+Space). The overlay runs hybrid search (FTS5 keyword + semantic vector) against the existing Rust backend, displays result cards with screenshot thumbnails and metadata, and provides a detail view with full screenshot, extracted text, and quick actions -- all styled in a "Signal X Dark" aesthetic using SvelteKit + Tailwind CSS within a Tauri window.

## Proof of Life

**Scenario:** User is working in any application. They press Cmd+Shift+Space. A floating search panel appears centered on screen. They type "invoice" and within 300ms of pausing, result cards appear showing a Chrome screenshot from 4 days ago with the invoice total highlighted in bold, a "4d ago" relative timestamp, and a Chrome app icon. They click a result and see the full screenshot with extracted text and a "Copy Text" button.
**Validates:** Global hotkey triggers overlay, search invokes the existing `search_captures` Tauri command, results render with thumbnails and metadata, detail view displays full context with actions.
**Must work before:** Chat/RAG interface, timeline view, settings UI, or any feature that consumes search results visually.

## User Stories

- As a knowledge worker, I want a global hotkey to instantly open a search overlay so I can recall anything I saw without leaving my current app.
- As a user searching my history, I want hybrid keyword + semantic results so I find things even when I don't remember the exact words.
- As a user reviewing results, I want to see a screenshot thumbnail, app name, relative timestamp, and text snippet so I can quickly identify the right result.
- As a user, I want to click a result and see the full screenshot with extracted text so I can read the complete context.
- As a user, I want to copy extracted text and open source files in Finder so I can act on what I found.
- As a user, I want to filter by application and date range so I can narrow my search when I remember the context.
- As a user, I want the overlay to disappear when I click away so it stays out of my way.

## Core Requirements

### Functional Requirements

1. **Floating overlay window** -- Raycast-style panel, ~700-800px wide, vertically centered on screen. Decorations disabled, transparent background, always-on-top, with window shadow.
2. **Global hotkey** -- Cmd+Shift+Space toggles the overlay (show/hide) via `tauri-plugin-global-shortcut`. Registered on app startup in the Tauri `setup` hook.
3. **Hide on blur** -- When the overlay loses focus, it hides automatically. Pressing the hotkey re-shows it with the previous query intact.
4. **Search input** -- Single text input with 300ms debounce. On each debounced keystroke, invokes the existing `search_captures` Tauri command.
5. **Hybrid search** -- The backend already runs FTS5 keyword search. Semantic search via `EmbeddingEngine` (all-MiniLM-L6-v2, 384-dim) queries sqlite-vec for vector similarity. Results from both sources are merged and deduplicated by `capture_id`. Semantic-only results (those not in the FTS5 set) are marked with a sparkle icon.
6. **Result cards** -- Each result displays: screenshot thumbnail (left, loaded via Tauri asset protocol `asset://localhost/`), app name + relative timestamp + text snippet with bold keyword highlights (center), source badge "OCR" or "Audio" (right).
7. **Detail overlay** -- Clicking a result card opens a detail view within the same window showing: full-resolution screenshot, complete extracted text, "Copy Text to Clipboard" button, "Open in Finder" button.
8. **Collapsible filter bar** -- Toggle via filter icon or `/app:` `/date:` modifiers. Provides app name dropdown (populated from distinct app names in captures table) and date range picker.
9. **Keyboard navigation** -- Arrow keys move selection through results, Enter opens detail view, Escape closes detail (or hides overlay if no detail is open).

### Non-Functional Requirements

1. **Perceived performance** -- Results must appear within 500ms of the debounce firing for queries up to 50 results.
2. **Visual consistency** -- All components follow Signal X Dark design tokens (see below).
3. **Memory** -- The overlay window should be lightweight; images load lazily and are not kept in memory after scrolling past.
4. **Accessibility** -- Focus management: overlay traps focus when visible, returns focus to previous app on hide.

## Visual Design

### Design Tokens (Signal X Dark)

| Token | Value | Usage |
|---|---|---|
| `--bg-primary` | `#0A0A0A` | Overlay background |
| `--bg-surface` | `#141414` | Result cards, detail panel |
| `--bg-elevated` | `#1C1C1C` | Hovered cards, input field |
| `--border-default` | `#262626` | Card borders, dividers |
| `--border-focus` | `#3B82F6` | Focused input, selected card |
| `--text-primary` | `#FAFAFA` | Headings, primary text |
| `--text-secondary` | `#A3A3A3` | Timestamps, metadata |
| `--text-muted` | `#737373` | Placeholder text |
| `--accent` | `#3B82F6` | Focus rings, active states |
| `--accent-semantic` | `#A78BFA` | Sparkle icon for semantic results |
| `--badge-ocr` | `#22D3EE` | OCR source badge |
| `--badge-audio` | `#F472B6` | Audio/transcription source badge |

### Typography

- Font stack: `-apple-system, BlinkMacSystemFont, "SF Pro Text", "SF Pro Display", system-ui, sans-serif`
- Search input: 18px, font-weight 400
- Result card title (app name): 13px, font-weight 600
- Result card snippet: 14px, font-weight 400
- Timestamp: 12px, font-weight 400, `--text-secondary`

### Layout

- Overlay: 720px wide, max-height 520px, 12px border-radius, 1px `--border-default` border
- Search input: full width, 48px height, 16px horizontal padding
- Result list: scrollable, max ~6 visible results
- Result card: 72px height, 12px padding, thumbnail 48x48px rounded-md
- Detail view: replaces result list, scrollable, full-width screenshot with text below

## Conversion Design

N/A -- this is a search interface, not a conversion flow.

## Reusable Components

### Existing Code to Leverage

- **`search_captures` Tauri command** (`src-tauri/src/lib.rs` line 47) -- Already registered, accepts `query`, `app_filter`, `time_from`, `time_to`. Returns `Vec<SearchResult>` with `capture_id`, `timestamp`, `app_name`, `snippet`, `image_path`, `result_type`.
- **`SearchResult` struct** (`src-tauri/src/search.rs`) -- Serialized to JSON for the frontend. Includes HTML bold tags in `snippet` from FTS5.
- **`EmbeddingEngine`** (`src-tauri/src/embedding.rs`) -- `embed_text()` returns 384-dim vector for semantic queries.
- **`Database`** (`src-tauri/src/storage.rs`) -- All query methods available. Has `search_captures()` with FTS5 + transcription union.
- **`+layout.svelte`** -- Already imports `app.css` and uses Svelte 5 `$props()`.
- **`app.css`** -- Already imports Tailwind CSS v4.

### New Components Required

- **`SearchOverlay.svelte`** -- Main overlay container. Manages visibility, input state, results, and detail view.
- **`SearchInput.svelte`** -- Debounced text input with filter toggle button.
- **`ResultCard.svelte`** -- Single result row: thumbnail, metadata, snippet, source badge.
- **`ResultDetail.svelte`** -- Full screenshot + text + action buttons.
- **`FilterBar.svelte`** -- Collapsible filters: app dropdown, date range.
- **Search window in `tauri.conf.json`** -- New window entry with `decorations: false`, `transparent: true`, `alwaysOnTop: true`, `visible: false`.
- **Global shortcut registration** -- In Tauri `setup` hook, register Cmd+Shift+Space to toggle search window visibility.

## Technical Approach

- **Window management:** Add a second window entry `"search"` in `tauri.conf.json` pointing to the same SvelteKit frontend (route `/search`). Configure with `decorations: false`, `transparent: true`, `alwaysOnTop: true`, `visible: false`, `width: 720`, `height: 520`, `center: true`. Use `@tauri-apps/api/window` to manage show/hide/focus.
- **Global shortcut:** Use `tauri-plugin-global-shortcut` (add to `Cargo.toml` and `tauri.conf.json` plugins). Register `CommandOrControl+Shift+Space` in the `setup` closure. On trigger, toggle the search window's visibility.
- **Hide on blur:** Listen for the window `blur` event via `@tauri-apps/api/window` `onFocusChanged`. When the search window loses focus, call `window.hide()`.
- **Search invocation:** Frontend calls `invoke('search_captures', { query, appFilter, timeFrom, timeTo })` from `@tauri-apps/api/core`. The 300ms debounce is handled in the Svelte component using a `$effect` with `setTimeout`/`clearTimeout`.
- **Thumbnail loading:** Screenshots are stored at `~/.cortex/screenshots/...`. Use Tauri's asset protocol (`asset://localhost/` + absolute path) to load images in `<img>` tags without needing a separate HTTP server. Requires `"asset"` protocol scope in `tauri.conf.json`.
- **Semantic result marking:** The backend merges FTS5 and semantic results. Add a `is_semantic: bool` field to `SearchResult` so the frontend can show the sparkle icon on semantic-only hits.
- **Svelte 5 patterns:** All components use runes: `$state` for reactive local state, `$derived` for computed values, `$effect` for side effects (debounce, event listeners). Props via `$props()`.
- **Styling:** Tailwind CSS v4 utility classes with custom CSS variables for design tokens defined in `app.css`. Dark mode is the only mode.

## Out of Scope

- Chat/RAG conversational interface (spec #7)
- Timeline scrubbing view (spec #6)
- Settings UI / configurable hotkey (spec #12)
- Audio playback in detail view (deferred)
- Multi-window search (single overlay only)
- Result ranking/scoring tuning
- Onboarding or first-run experience for search

## Success Criteria

1. Cmd+Shift+Space opens the search overlay from any application within 200ms.
2. Typing a query shows results within 500ms of the 300ms debounce (800ms total from last keystroke).
3. Result cards display screenshot thumbnails, app names, relative timestamps, and highlighted text snippets.
4. Semantic-only results are visually distinguished with a sparkle icon.
5. Clicking a result shows full screenshot, complete extracted text, and working Copy/Finder actions.
6. Overlay hides when clicking outside or pressing Escape.
7. Keyboard navigation (arrows, Enter, Escape) works throughout the result list and detail view.
8. Filter bar filters results by app name and date range.
9. All components render correctly with Signal X Dark design tokens.
