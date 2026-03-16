# Task Breakdown: Timeline View

## Overview
Total Tasks: 4 groups, 30 subtasks
Strategy: squad
Depth: standard
Assigned roles: ui-engineer (Svelte components + styling), api-engineer (Tauri commands + DB queries), testing-engineer (gap analysis)

## Task List

### Proof of Life -- Vertical Slice

#### Task Group 1: Timeline Route, Stage, Filmstrip with Basic Thumbnails
**Assigned implementer:** api-engineer, ui-engineer
**Dependencies:** Capture Daemon spec (complete), OCR Pipeline spec (complete), Search UI spec (design tokens)

This group delivers the Proof of Life scenario: user opens `/timeline`, sees a filmstrip of today's captures at 1 thumbnail/minute, clicks one, and the full screenshot + OCR + transcription metadata loads in the center Stage. Requires new Tauri commands for day-based queries and the core layout components.

- [ ] 1.0 Complete timeline route with Stage and Filmstrip showing today's captures
  - [ ] 1.1 Add `get_captures_for_day` Tauri command -- In `src-tauri/src/storage.rs`, add method `get_captures_for_day(&self, date: &str) -> Result<Vec<CaptureRow>>` that queries `SELECT * FROM captures WHERE date(timestamp) = ?1 ORDER BY timestamp ASC`. Register as Tauri command in `src-tauri/src/lib.rs`.
  - [ ] 1.2 Add `get_capture_by_id` Tauri command -- Create a `CaptureDetail` struct in `src-tauri/src/storage.rs` containing `capture: CaptureRow`, `ocr_text: Option<String>`, `transcriptions: Vec<TranscriptionRow>`. Add method `get_capture_by_id(&self, id: i64)` that joins `captures` with `captures_fts` and `transcriptions` (where `timestamp_start` is within +/- 30s of the capture timestamp). Register as Tauri command.
  - [ ] 1.3 Create `src/routes/timeline/+page.svelte` -- Timeline route. Three-zone layout: `TimelineToolbar` (top, 48px), `Stage` (center, flex-1), `Filmstrip` (bottom, fixed 176px). Uses `$state` for `captures`, `selectedCaptureId`, `selectedDate` (defaults to today). On mount, calls `get_captures_for_day` for today.
  - [ ] 1.4 Create `src/lib/components/timeline/Stage.svelte` -- Props: `captureDetail: CaptureDetail | null`. Displays full-res screenshot via asset protocol (`asset://localhost/` + `image_path`), `object-contain` to fill available space. Below/beside the image: app name (16px, bold), window title (14px, secondary), timestamp (13px, secondary). OCR text in a scrollable mono-font container. Transcription text if present. Loading skeleton when `captureDetail` is null.
  - [ ] 1.5 Create `src/lib/components/timeline/Filmstrip.svelte` -- Props: `captures: CaptureRow[]`, `selectedId: number | null`, `onselect: (id: number) => void`. Implements basic 1-per-minute downsampling: bucket captures by minute, pick first in each bucket. Renders a horizontal scrollable `<div>` with `overflow-x: auto` and `scroll-behavior: smooth`. Time axis with hour markers at top. Each thumbnail rendered via `FilmstripThumbnail`.
  - [ ] 1.6 Create `src/lib/components/timeline/FilmstripThumbnail.svelte` -- Props: `capture: CaptureRow`, `selected: boolean`, `onclick: () => void`. Renders 96x120px thumbnail via asset protocol with `loading="lazy"` and `decoding="async"`. Selected state: 2px `--accent` border ring. Hover state: `--bg-elevated` background. Time label below thumbnail (HH:MM format).
  - [ ] 1.7 Create `src/lib/components/timeline/TimelineToolbar.svelte` -- Minimal version for Proof of Life: display current date as text, placeholder for calendar and filters. 48px height, `--bg-surface` background, `--border-default` bottom border.
  - [ ] 1.8 Wire selection flow -- In `+page.svelte`, when `Filmstrip` fires `onselect(id)`, set `selectedCaptureId`, then call `get_capture_by_id(id)` and pass result to `Stage`. Default to selecting the most recent capture on page load.
  - [ ] 1.9 Write tests -- (a) `get_captures_for_day` returns captures ordered by timestamp ascending for a given date. (b) `get_capture_by_id` returns capture with OCR text and transcriptions. (c) Filmstrip downsampling produces ~1 thumbnail per minute from a dense capture list.

**Acceptance Criteria:**
- `/timeline` route loads in the main window and displays today's captures
- Filmstrip shows ~1 thumbnail per minute with time labels
- Clicking a thumbnail loads the full screenshot + metadata in the Stage
- OCR text and transcription segments display in the Stage
- Most recent capture is selected by default on page load

**Verification Steps:**
1. Navigate to `/timeline` -- expect filmstrip populates with today's captures
2. Click a thumbnail -- expect Stage shows full screenshot with app name, timestamp, OCR text
3. Verify filmstrip has ~1 thumbnail per minute (not one per capture)
4. Verify time axis shows hour markers

**Verification Commands:**
```bash
# Build frontend
npm run build

# Run full app
npx tauri dev

# Verify new Tauri commands compile
cd src-tauri && cargo check
```

---

### Smart Downsampling & Zoom

#### Task Group 2: Zoom Levels, Hover Expansion, Virtualized Rendering
**Assigned implementer:** ui-engineer
**Dependencies:** Task Group 1

This group adds the zoom interaction model to the filmstrip: scroll-wheel zoom between 1-minute, 30-second, and 5-second granularity, hover-to-expand lens effect, and virtualized DOM rendering for performance.

- [ ] 2.0 Complete smart downsampling with zoom and virtualized rendering
  - [ ] 2.1 Add zoom level state -- In `Filmstrip.svelte`, add `$state` for `zoomLevel: '1min' | '30s' | '5s'` defaulting to `'1min'`. Listen for `wheel` events (with Ctrl/Meta held) or pinch gestures on the filmstrip. Ctrl+scroll-up increases granularity (1min -> 30s -> 5s), Ctrl+scroll-down decreases.
  - [ ] 2.2 Implement multi-level downsampling -- Create `$derived` `downsampledCaptures` that buckets the full capture list by the current zoom interval (60s, 30s, or 5s) and picks the first capture per bucket. Each bucket also stores the count of captures it represents (for density indication).
  - [ ] 2.3 Implement hover expansion lens -- Track mouse X position over the filmstrip. When hovering, compute the time range under the cursor (+/- 2.5 minutes). Within that range, override downsampling to use 5-second granularity regardless of zoom level. This creates a "magnifying" effect. Smoothly transition the expanded region width over 150ms.
  - [ ] 2.4 Implement virtualized rendering -- Replace the naive "render all thumbnails" approach with a virtual scroll. Calculate which thumbnails are visible based on `scrollLeft` and container width. Only render visible thumbnails + 10-item buffer on each side. Use `$effect` to listen to `scroll` events and update the visible range. Maintain a spacer element at the end to keep the scrollbar accurate.
  - [ ] 2.5 Add zoom level indicator -- Small pill in the bottom-right of the filmstrip showing the current granularity ("1 min", "30s", "5s"). Fades in on zoom change, fades out after 2 seconds.
  - [ ] 2.6 Smooth scroll-to-time utility -- Create `scrollToTime(time: Date)` method on the filmstrip that calculates the pixel offset for a given time and smoothly scrolls to it. Used by Jump to Now and keyboard navigation.
  - [ ] 2.7 Write tests -- (a) Downsampling at 1min produces correct bucket count for a known capture set. (b) Zoom level changes correctly on Ctrl+scroll events. (c) Virtual scroll only renders items within the visible range + buffer.

**Acceptance Criteria:**
- Ctrl+scroll wheel changes zoom level between 1min, 30s, and 5s granularity
- Filmstrip re-renders with appropriate number of thumbnails at each zoom level
- Hovering over a region expands it to 5-second granularity
- Filmstrip scrolls at 60fps with 600+ thumbnails via virtualization
- Zoom level indicator shows current granularity

**Verification Steps:**
1. Scroll filmstrip horizontally -- expect smooth 60fps scrolling
2. Ctrl+scroll up -- expect more thumbnails appear (finer granularity)
3. Ctrl+scroll down -- expect fewer thumbnails (coarser granularity)
4. Hover over a section -- expect it expands to show more thumbnails
5. Check DOM inspector -- expect only ~30-50 thumbnail nodes in DOM regardless of total count

**Verification Commands:**
```bash
npm run build

npx tauri dev

# Performance check: open DevTools, check frame rate during filmstrip scroll
```

---

### Transcription Track & Metadata

#### Task Group 3: Subtitle Track, OCR Overlay, Copy Image Action
**Assigned implementer:** ui-engineer, api-engineer
**Dependencies:** Task Group 1

This group adds the transcription subtitle track below the filmstrip, enriches the Stage with OCR text display, and implements the Copy Image clipboard action.

- [ ] 3.0 Complete subtitle track, Stage metadata, and Copy Image
  - [ ] 3.1 Add `get_transcriptions_for_day` Tauri command -- In `src-tauri/src/storage.rs`, add method `get_transcriptions_for_day(&self, date: &str) -> Result<Vec<TranscriptionRow>>` that queries transcriptions where `date(timestamp_start) = ?1` ordered by `timestamp_start ASC`. Create `TranscriptionRow` struct with `id`, `capture_id`, `timestamp_start`, `timestamp_end`, `text`, `source`. Register as Tauri command.
  - [ ] 3.2 Create `src/lib/components/timeline/SubtitleTrack.svelte` -- Props: `transcriptions: TranscriptionRow[]`, `dayStart: Date`, `dayEnd: Date`, `onselect: (transcription: TranscriptionRow) => void`. Renders a horizontal bar below the filmstrip, same width and scroll-synced. Each transcription is a pill positioned by time: `left` = time-to-pixel(timestamp_start), `width` = time-to-pixel(duration). Pill styled with `--accent-transcript` at 20% opacity, truncated text, 11px font. Clicking a pill fires `onselect`.
  - [ ] 3.3 Sync subtitle track scroll with filmstrip -- The subtitle track and filmstrip must scroll together horizontally. Wrap both in a shared scroll container, or sync their `scrollLeft` values via `$effect` when either scrolls.
  - [ ] 3.4 Enhance Stage OCR display -- In `Stage.svelte`, display the OCR text in a collapsible panel below the screenshot. Styled: `--bg-surface` background, `--border-default` border, mono font (SF Mono / JetBrains Mono), 13px, `--text-secondary`. Expandable with a "Show OCR Text" toggle. If no OCR text, show "No text detected" in muted text.
  - [ ] 3.5 Enhance Stage transcription display -- In `Stage.svelte`, below the OCR panel, show transcription segments associated with the capture. Each segment: speaker source label ("System Audio" / "Microphone"), text content, start-end time range. Styled with `--accent-transcript` left border, `--bg-surface` background.
  - [ ] 3.6 Add `copy_image_to_clipboard` Tauri command -- In `src-tauri/src/lib.rs`, create command that reads the image file at a given path, converts to PNG if needed (using the `image` crate), and writes to the system clipboard using the `arboard` crate (`Clipboard::new()?.set_image()`). Add `arboard` and `image` to `Cargo.toml`.
  - [ ] 3.7 Add Copy Image button to Stage -- In `Stage.svelte`, add a "Copy Image" button in the action bar. On click, calls `invoke('copy_image_to_clipboard', { path })`. Show "Copied!" confirmation for 1.5 seconds. Styled: `--bg-elevated` background, `--text-primary` text, clipboard icon.
  - [ ] 3.8 Write tests -- (a) `get_transcriptions_for_day` returns transcriptions ordered by start time for a given date. (b) `copy_image_to_clipboard` handles valid PNG/WebP paths without error. (c) Subtitle track pills are positioned proportionally to their time range.

**Acceptance Criteria:**
- Transcription pills appear in the subtitle track, aligned to their time range
- Subtitle track scrolls in sync with the filmstrip
- Stage shows OCR text in a collapsible mono-font panel
- Stage shows transcription segments with source labels
- Copy Image button copies the screenshot to the system clipboard
- Clicking a subtitle pill loads the associated capture in the Stage

**Verification Steps:**
1. View a day with transcriptions -- expect pills in the subtitle track
2. Scroll the filmstrip -- expect subtitle track scrolls with it
3. Click a transcription pill -- expect Stage loads the nearest capture with transcription highlighted
4. Click "Copy Image" -- paste into Preview or Messages, verify the screenshot appears
5. Toggle OCR text panel -- expect it expands/collapses showing extracted text

**Verification Commands:**
```bash
npm run build

cd src-tauri && cargo check

npx tauri dev

# Verify arboard crate compiles
cd src-tauri && cargo build
```

---

### Filters & Navigation

#### Task Group 4: App Filter, Content Type, Calendar Picker, Jump to Now, Keyboard Navigation
**Assigned implementer:** ui-engineer
**Dependencies:** Task Groups 1, 2, 3

This group completes the toolbar with all filter and navigation controls, adds keyboard navigation, and applies final polish.

- [ ] 4.0 Complete filters, calendar, Jump to Now, and keyboard navigation
  - [ ] 4.1 Add calendar date picker to toolbar -- In `TimelineToolbar.svelte`, add a date input (styled as a calendar button that opens a native date picker or custom dropdown). When the date changes, fire an `ondatechange(date: string)` event. In `+page.svelte`, re-fetch captures and transcriptions for the new date, reset filmstrip scroll to start, clear selection.
  - [ ] 4.2 Add app name filter dropdown -- In `TimelineToolbar.svelte`, call `get_distinct_apps()` on mount to populate a `<select>` dropdown. When an app is selected, filter the captures list client-side (since all day captures are already loaded). Update the filmstrip and subtitle track to show only matching captures. "All Apps" option to clear the filter.
  - [ ] 4.3 Add content type filter -- In `TimelineToolbar.svelte`, add a segmented control or toggle group: "All" | "Screenshots" | "Audio". "Screenshots" filters to captures with `ocr_status = 'completed'`. "Audio" filters to time ranges that have transcription segments. Default: "All".
  - [ ] 4.4 Implement Jump to Now -- In `TimelineToolbar.svelte`, add a "Jump to Now" button (clock icon). On click: scroll the filmstrip to the current time position using `scrollToTime(new Date())`, select the most recent capture. Disable the button if viewing a past day (not today). Styled: `--accent` text when active, `--text-muted` when disabled.
  - [ ] 4.5 Implement keyboard navigation -- In `+page.svelte`, listen for `keydown` on the document. Left arrow: select previous capture in the (downsampled) list. Right arrow: select next capture. Hold to repeat at ~5/second (use `keydown` repeat events). Home: select first capture of day. End: select last capture. Ensure the filmstrip scrolls to keep the selected thumbnail visible.
  - [ ] 4.6 Add navigation between main view and timeline -- Add a tab or sidebar link in the main Cortex window layout that switches between the default view and `/timeline`. Use SvelteKit routing. Active tab indicator styled with `--accent`.
  - [ ] 4.7 Add empty states -- No captures for selected day: show centered message "No captures for [date]" with calendar icon. No captures matching filter: "No [app name] captures today". Loading state: skeleton placeholders in filmstrip and Stage.
  - [ ] 4.8 Add filmstrip time axis polish -- Hour markers as vertical lines with labels ("9 AM", "10 AM", etc.). Half-hour marks as shorter lines without labels. Current time indicator as a thin `--accent` vertical line (today only). Time range adapts to the actual capture span (not always midnight-to-midnight).
  - [ ] 4.9 Write tests -- (a) Calendar date change triggers re-fetch of captures for the new day. (b) App filter correctly reduces the filmstrip to matching captures only. (c) Keyboard left/right arrow changes selection. (d) Jump to Now scrolls to the most recent capture.

**Acceptance Criteria:**
- Calendar picker loads captures for a different day
- App filter narrows filmstrip to captures from a specific app
- Content type filter toggles between screenshots, audio, and all
- Jump to Now scrolls to the current time and selects the latest capture
- Left/right arrow keys step through captures in order
- Navigation tab switches between main view and timeline
- Empty and loading states display appropriate messages
- Time axis shows hour markers with current-time indicator

**Verification Steps:**
1. Click calendar, select yesterday -- expect filmstrip loads yesterday's captures
2. Select an app from the filter dropdown -- expect only that app's captures remain
3. Toggle content type to "Audio" -- expect only time ranges with transcriptions
4. Click "Jump to Now" -- expect filmstrip scrolls to current time
5. Press right arrow repeatedly -- expect captures advance one by one
6. Press Home -- expect first capture of day selected
7. Navigate to a day with no captures -- expect empty state message

**Verification Commands:**
```bash
# Full frontend build
npm run build

# Full app with hot reload
npx tauri dev

# Run any component tests
npm test
```

---

## Execution Order

1. **Task Group 1: Timeline Route + Stage + Filmstrip** (api-engineer + ui-engineer) -- Proof of Life vertical slice. Must complete first. Delivers the route, Tauri commands, basic filmstrip, and Stage.
2. **Task Group 2: Smart Downsampling & Zoom** (ui-engineer) -- Depends on Group 1 for filmstrip. Adds zoom levels, hover expansion, and virtualization.
3. **Task Group 3: Transcription Track & Metadata** (ui-engineer + api-engineer) -- Depends on Group 1 for layout and Stage. Can run in parallel with Group 2.
4. **Task Group 4: Filters & Navigation** (ui-engineer) -- Depends on Groups 1, 2, and 3. Final integration and polish pass.

**Parallel execution possible:** Groups 2 and 3 can run concurrently after Group 1 completes.
