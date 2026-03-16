# Task Breakdown: Search UI

## Overview
Total Tasks: 4 groups, 28 subtasks
Strategy: squad
Depth: standard
Assigned roles: ui-engineer (Svelte components + styling), api-engineer (Tauri window + shortcut + IPC), testing-engineer (gap analysis)

## Task List

### Proof of Life — Vertical Slice

#### Task Group 1: Search Overlay + Results
**Assigned implementer:** api-engineer, ui-engineer
**Dependencies:** Capture Daemon spec (complete), OCR Pipeline spec (complete), Embedding Search spec (complete)

This group delivers the Proof of Life scenario: Cmd+Shift+Space opens a floating overlay, user types a query, results appear from the existing `search_captures` backend. It is a vertical slice across Tauri window config, global shortcut, SvelteKit search route, and basic result rendering.

- [ ] 1.0 Complete search overlay with working query-to-results pipeline
  - [ ] 1.1 Add `"search"` window to `src-tauri/tauri.conf.json` — Configure: `label: "search"`, `url: "/search"`, `width: 720`, `height: 520`, `decorations: false`, `transparent: true`, `alwaysOnTop: true`, `visible: false`, `center: true`, `resizable: false`. Add `"asset"` protocol scope for `~/.cortex/screenshots/` to enable thumbnail loading.
  - [ ] 1.2 Add `tauri-plugin-global-shortcut` — Add crate to `src-tauri/Cargo.toml`. Add plugin to the `tauri.conf.json` plugins list. In `src-tauri/src/lib.rs` `setup` closure, register `CommandOrControl+Shift+Space` to toggle the `"search"` window visibility (show + focus if hidden, hide if visible).
  - [ ] 1.3 Create `src/routes/search/+page.svelte` — The search route. Renders `SearchOverlay` component. This is the entry point for the search window.
  - [ ] 1.4 Create `src/lib/components/search/SearchOverlay.svelte` — Main overlay container. Uses `$state` for `query`, `results`, `selectedIndex`, `showDetail`. Renders `SearchInput` at top, scrollable results list below. Transparent background with `#0A0A0A` rounded panel, `#262626` border, window shadow via Tauri `set_shadow(true)`.
  - [ ] 1.5 Create `src/lib/components/search/SearchInput.svelte` — Text input with 300ms debounce. Props: `oninput` callback. Uses `$effect` with `setTimeout`/`clearTimeout` for debounce. Styled: 48px height, 18px font, `#1C1C1C` background, placeholder "Search your history...", magnifying glass icon left, filter toggle button right.
  - [ ] 1.6 Wire search invocation — In `SearchOverlay.svelte`, use `$effect` to call `invoke('search_captures', { query, appFilter: null, timeFrom: null, timeTo: null })` from `@tauri-apps/api/core` when the debounced query changes. Store results in `$state`. Handle empty query (clear results) and loading state.
  - [ ] 1.7 Create `src/lib/components/search/ResultCard.svelte` — Single result row. Props: `result: SearchResult`, `selected: boolean`. Layout: thumbnail (48x48 rounded-md, left), app name + relative timestamp top-right of center, snippet with `{@html}` for bold tags below, source badge ("OCR" or "Audio") on far right. Hover state with `#1C1C1C` background.
  - [ ] 1.8 Implement hide-on-blur — In `SearchOverlay.svelte`, use `getCurrentWindow()` from `@tauri-apps/api/window` and listen for the `blur` event via `onFocusChanged`. On blur, call `window.hide()`. On show (via hotkey), call `window.show()` and `window.setFocus()`.
  - [ ] 1.9 Write tests — (a) Verify `tauri.conf.json` search window has correct properties (unit test or manual check). (b) Verify debounce fires only once for rapid input (Svelte component test or manual). (c) Verify results render with correct structure after invoke returns mock data.

**Acceptance Criteria:**
- Cmd+Shift+Space opens a floating, borderless, always-on-top search panel centered on screen
- Typing a query invokes `search_captures` after 300ms debounce and displays result cards
- Result cards show app name, relative timestamp, text snippet, and source badge
- Clicking outside the overlay hides it
- Pressing Cmd+Shift+Space again re-shows the overlay with the previous query

**Verification Steps:**
1. Run app, press Cmd+Shift+Space — expect overlay appears centered on screen
2. Type "error" — expect results from captured data appear within ~800ms
3. Click outside overlay — expect it disappears
4. Press Cmd+Shift+Space again — expect overlay reappears with "error" still in input

**Verification Commands:**
```bash
# Build frontend
npm run build

# Run full app
npx tauri dev

# Check search window config
cat src-tauri/tauri.conf.json | grep -A 15 '"search"'
```

---

### Result Cards & Thumbnails

#### Task Group 2: Rich Result Cards with Thumbnails and Semantic Markers
**Assigned implementer:** ui-engineer
**Dependencies:** Task Group 1

- [ ] 2.0 Complete rich result cards with thumbnails, highlights, and semantic indicators
  - [ ] 2.1 Implement thumbnail loading — Use Tauri's asset protocol (`asset://localhost/` + absolute path from `image_path`) to load screenshot thumbnails in `<img>` tags. Add lazy loading (`loading="lazy"`). Show a neutral placeholder `#141414` div while loading. Handle missing images gracefully (show generic icon).
  - [ ] 2.2 Add relative timestamp formatting — Create `src/lib/utils/time.ts` with a `relativeTime(isoString: string): string` function. Returns "just now", "2m ago", "3h ago", "4d ago", "2w ago", etc. No external dependency — use simple math against `Date.now()`.
  - [ ] 2.3 Add keyword highlight rendering — The backend returns snippets with `<b>` tags from FTS5. Render via `{@html snippet}` in `ResultCard.svelte`. Style `<b>` tags with `color: var(--accent)` and `font-weight: 600`. Sanitize the snippet to prevent XSS (strip all tags except `<b>` and `</b>`).
  - [ ] 2.4 Add source badges — Render a small pill badge on the right side of each `ResultCard`. "OCR" badge in `#22D3EE` (cyan), "Audio" badge in `#F472B6` (pink). Use `result.result_type` to determine which badge to show. Styled: 10px font, uppercase, rounded-full, px-2 py-0.5.
  - [ ] 2.5 Add semantic result sparkle icon — Add `is_semantic` field support. When a result is semantic-only (not from FTS5), show a small sparkle/brain SVG icon next to the app name in `#A78BFA` (purple). Add a tooltip "Found via meaning, not exact words".
  - [ ] 2.6 Update `SearchResult` Rust struct — Add `is_semantic: bool` field to `SearchResult` in `src-tauri/src/search.rs`. Default to `false` for FTS5 results. Set to `true` for results that came only from vector similarity search. This requires the backend search to track which results came from which source.
  - [ ] 2.7 Write tests — (a) `relativeTime` returns correct strings for various time differences. (b) Snippet sanitization strips non-`<b>` tags. (c) Source badge renders correct color for "ocr" vs "transcription" result types.

**Acceptance Criteria:**
- Screenshot thumbnails load from disk via asset protocol and display at 48x48
- Timestamps show relative time ("4d ago") instead of ISO strings
- Keywords in snippets are highlighted with the accent color
- Source badges clearly distinguish OCR vs Audio results
- Semantic-only results show a sparkle icon

**Verification Steps:**
1. Search for a term that exists in captured OCR text — expect thumbnail, bold highlights, "OCR" badge
2. Search for a term that exists in transcriptions — expect "Audio" badge in pink
3. Check relative timestamps are correct for captures from various dates
4. Verify semantic-only results show sparkle icon (requires embedding data in DB)

**Verification Commands:**
```bash
npm run build

npx tauri dev

# Verify asset protocol works by checking search window can load images
# (manual: type a query, inspect result thumbnails)
```

---

### Detail View & Actions

#### Task Group 3: Result Detail Overlay with Screenshot and Actions
**Assigned implementer:** ui-engineer
**Dependencies:** Task Group 1

- [ ] 3.0 Complete detail view with full screenshot, text, and action buttons
  - [ ] 3.1 Create `src/lib/components/search/ResultDetail.svelte` — Full detail overlay that replaces the result list within the search panel. Props: `result: SearchResult`. Shows back arrow button at top to return to results.
  - [ ] 3.2 Implement full screenshot display — Load the full-resolution screenshot via asset protocol. Display at full width within the overlay (max-width constrained to panel width). Add rounded corners and `#262626` border. Handle loading state with skeleton placeholder.
  - [ ] 3.3 Implement full text display — Below the screenshot, show the complete OCR text for the capture. Create a new Tauri command `get_capture_text(capture_id: i64)` in `src-tauri/src/lib.rs` that returns the full OCR text from the `captures_fts` table (not the snippet). Styled: 14px mono font (JetBrains Mono or SF Mono), `#FAFAFA` text, `#141414` background, scrollable container with max-height.
  - [ ] 3.4 Add "Copy Text to Clipboard" button — Use `navigator.clipboard.writeText()` or Tauri's clipboard plugin. Styled: `#262626` background, `#FAFAFA` text, hover `#333333`, with clipboard icon. Show brief "Copied!" confirmation state (1.5s).
  - [ ] 3.5 Add "Open in Finder" button — Use `invoke('open_in_finder', { path: result.image_path })`. Create new Tauri command `open_in_finder(path: String)` in `src-tauri/src/lib.rs` that calls `Command::new("open").arg("-R").arg(path)` to reveal the file in Finder. Styled same as Copy button, with folder icon.
  - [ ] 3.6 Wire detail view to overlay — In `SearchOverlay.svelte`, when a `ResultCard` is clicked (or Enter is pressed on selected card), set `selectedResult` state and render `ResultDetail` instead of the result list. Back button or Escape returns to results. Click-outside still hides the entire overlay.
  - [ ] 3.7 Write tests — (a) `get_capture_text` command returns full text for a valid capture ID. (b) `open_in_finder` command does not panic on valid/invalid paths. (c) Detail view renders screenshot and text when given a result prop.

**Acceptance Criteria:**
- Clicking a result card transitions to detail view showing full screenshot
- Full OCR/transcription text is displayed below the screenshot
- "Copy Text" button copies text to clipboard with confirmation feedback
- "Open in Finder" button reveals the screenshot file in Finder
- Back button and Escape return to the result list
- Click-outside still hides the entire overlay from detail view

**Verification Steps:**
1. Search, click a result — expect detail view with full screenshot and text
2. Click "Copy Text" — paste into another app, verify text matches
3. Click "Open in Finder" — expect Finder opens with screenshot file selected
4. Press Escape — expect return to result list
5. Click outside overlay while in detail view — expect overlay hides

**Verification Commands:**
```bash
npm run build

npx tauri dev

# Test get_capture_text command exists
# (manual: click a result, verify full text loads)

# Test open_in_finder
# (manual: click "Open in Finder", verify Finder opens)
```

---

### Filters & Polish

#### Task Group 4: Filter Bar, Keyboard Navigation, and Design Polish
**Assigned implementer:** ui-engineer
**Dependencies:** Task Groups 1, 2, 3

- [ ] 4.0 Complete filter bar, keyboard navigation, and visual polish
  - [ ] 4.1 Create `src/lib/components/search/FilterBar.svelte` — Collapsible filter panel between search input and results. Toggle via filter icon button in `SearchInput` or by typing `/app:` or `/date:` in the search box. Uses `$state` for `expanded: boolean`, `selectedApp: string | null`, `dateFrom: string | null`, `dateTo: string | null`.
  - [ ] 4.2 Add app name dropdown — Create a new Tauri command `get_distinct_apps()` in `src-tauri/src/lib.rs` that queries `SELECT DISTINCT app_name FROM captures ORDER BY app_name`. Populate a `<select>` dropdown. Styled: `#1C1C1C` background, `#262626` border, `#FAFAFA` text.
  - [ ] 4.3 Add date range filter — Two date input fields (From / To) or preset buttons ("Today", "Past Week", "Past Month"). Pass `timeFrom` and `timeTo` as ISO strings to the `search_captures` invoke call.
  - [ ] 4.4 Wire filters to search — When filter values change, re-invoke `search_captures` with the filter parameters via `$effect`. Clear filters button resets all filters and re-runs search.
  - [ ] 4.5 Implement keyboard navigation — In `SearchOverlay.svelte`, listen for `keydown` events. Arrow Up/Down moves `selectedIndex` through results (clamped to bounds, scrolls into view). Enter opens detail view for selected result. Escape closes detail view first, then hides overlay on second press. Tab cycles between input and filter fields.
  - [ ] 4.6 Add Signal X Dark design tokens to `src/app.css` — Define CSS custom properties: `--bg-primary: #0A0A0A`, `--bg-surface: #141414`, `--bg-elevated: #1C1C1C`, `--border-default: #262626`, `--border-focus: #3B82F6`, `--text-primary: #FAFAFA`, `--text-secondary: #A3A3A3`, `--text-muted: #737373`, `--accent: #3B82F6`, `--accent-semantic: #A78BFA`. Apply to `:root`.
  - [ ] 4.7 Add transitions and animations — Overlay: fade-in on show (150ms opacity + slight scale from 0.98). Result cards: subtle hover transition (100ms background-color). Detail view: slide-in from right (200ms transform). Filter bar: slide-down expand (150ms height).
  - [ ] 4.8 Polish scroll behavior — Result list: custom scrollbar styling (thin, `#262626` track, `#404040` thumb). Scroll selected result into view on keyboard navigation. Prevent body scroll when overlay is open.
  - [ ] 4.9 Add empty states — No results: show "No results for [query]" centered with muted text. No query: show "Start typing to search your history" with a subtle search icon. Error state: show "Search failed — try again" with retry action.
  - [ ] 4.10 Write tests — (a) `get_distinct_apps` returns correct list from DB. (b) Keyboard navigation updates selectedIndex correctly at bounds. (c) Filter bar toggles visibility. (d) Date filter formats produce valid ISO strings for the backend.

**Acceptance Criteria:**
- Filter bar expands/collapses and shows app dropdown + date range
- Selecting an app or date range re-runs search with filters applied
- Arrow keys navigate through results, Enter opens detail, Escape closes
- All components use Signal X Dark design tokens
- Transitions are smooth (fade, slide, hover)
- Empty and error states display appropriate messages
- Scrollbar is custom-styled and keyboard nav scrolls into view

**Verification Steps:**
1. Click filter icon — expect filter bar slides down with app dropdown and date fields
2. Select an app — expect results filter to that app only
3. Set date range — expect results filter to that range
4. Use arrow keys to navigate results — expect selection highlight moves
5. Press Enter — expect detail view opens for selected result
6. Press Escape — expect detail closes, then overlay hides on second Escape
7. Search for nonsense — expect "No results" empty state
8. Verify all colors match Signal X Dark tokens

**Verification Commands:**
```bash
# Full frontend build (catches compile errors)
npm run build

# Full app with hot reload
npx tauri dev

# Verify design tokens in CSS
grep -c "bg-primary" src/app.css

# Run any component tests
npm test
```

---

## Execution Order

1. **Task Group 1: Search Overlay + Results** (api-engineer + ui-engineer) — Proof of Life vertical slice. Must complete first. Delivers working hotkey, overlay, search, and basic results.
2. **Task Group 2: Rich Result Cards** (ui-engineer) — Depends on Group 1 for basic ResultCard. Adds thumbnails, highlights, badges, semantic markers.
3. **Task Group 3: Detail View & Actions** (ui-engineer) — Depends on Group 1 for overlay structure. Can run in parallel with Group 2.
4. **Task Group 4: Filters & Polish** (ui-engineer) — Depends on Groups 1, 2, and 3. Final polish pass.

**Parallel execution possible:** Groups 2 and 3 can run concurrently after Group 1 completes.
