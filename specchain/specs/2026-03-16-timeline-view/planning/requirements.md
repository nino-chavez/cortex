# Spec Requirements: Timeline View

## Initial Description
Timeline View -- full-window visual history browser in the main Cortex window. A filmstrip/horizontal scrollbar at bottom (video editor style) with a large center "Stage" showing the active capture. Smart downsampling for thumbnails, transcription subtitle track, filters, and day-based navigation.

## Requirements Discussion

### First Round Questions

**Q1: Proof of Life**
**Answer:** User opens the History/Timeline tab in the main Cortex window. A filmstrip of the day's captures appears at the bottom. They scrub through it visually, click any thumbnail, and the full screenshot loads in the center Stage with OCR text and transcription metadata displayed alongside it.

**Q2: Value Signal**
**Answer:** "Visual Time Machine" -- user stops digging through folders or search results and instead scrubs their visual history the way they'd scrub a video timeline. The spatial/temporal layout triggers recall faster than text search alone.

**Q3: Layout**
**Answer:** Full-window experience in the main Cortex window, NOT the search overlay. Three zones: large center Stage (active capture + metadata), filmstrip/horizontal scrollbar at bottom (video editor style), and a toolbar/filter bar at top.

**Q4: Filmstrip Behavior**
**Answer:** Smart downsampling: 1 thumbnail per minute at default zoom. On zoom or hover, expand to 5-second granularity. This avoids loading 1,000+ thumbnails for a full day while still allowing precision scrubbing. Horizontal scroll with momentum/inertia.

**Q5: Transcription Display**
**Answer:** Transcription segments as a "subtitle track" below the screenshot thumbnails in the filmstrip. Each segment aligns to its time range. Clicking a transcription segment loads the corresponding capture in the Stage.

**Q6: Filters**
**Answer:** App name filter (dropdown of distinct apps), content type filter (screenshots, transcriptions, or both), "Jump to Now" button that scrolls the filmstrip to the current time, and a calendar picker for navigating to previous days.

**Q7: Stage Actions**
**Answer:** "Copy Image" action is in scope -- low effort, high utility. Copies the full screenshot to the system clipboard. Also show OCR text and any transcription text associated with the capture's time range.

**Q8: Keyboard Navigation**
**Answer:** Left/right arrow keys step through captures one at a time. Holding arrow key should rapid-advance. This gives keyboard-only users a way to scrub without the mouse.

**Q9: Design Direction**
**Answer:** Same Signal X Dark design tokens as the search UI. Deep charcoal/black background, subtle borders, consistent with the rest of Cortex.

**Q10: Out of Scope**
**Answer:** No audio playback, no editing captures, no export functionality. Those are separate specs.

### Existing Code to Reference
- **storage.rs** -- `CaptureRow` struct, `get_recent_captures()`, `get_captures_by_app()`, existing DB schema with `captures`, `captures_fts`, `transcriptions` tables
- **search.rs** -- `SearchResult` struct, `search_captures()` with FTS5 + transcription union, `insert_fts()`, OCR status queries
- **ResultCard.svelte** -- Existing search result card with Signal X Dark styling, `relativeTime()` helper, source badges
- **tauri.conf.json** -- Main window configuration
- **lib.rs** -- Existing Tauri commands

## Requirements Summary

### Functional Requirements
- Full-window timeline route at `/timeline` in the main Cortex window
- Large center Stage displaying the active capture at full resolution with metadata
- Horizontal filmstrip at bottom with thumbnail strip (video editor style)
- Smart downsampling: 1 thumbnail per minute default, 5-second granularity on zoom/hover
- Transcription subtitle track below filmstrip thumbnails
- Calendar picker for date selection and "Jump to Now" button
- App name and content type filters
- "Copy Image" action on the Stage capture
- OCR text and transcription text displayed on Stage
- Left/right arrow keyboard navigation through captures

### UI Components (SvelteKit + Tailwind)
- TimelinePage.svelte -- main route container with layout zones
- Stage.svelte -- center panel: full-res screenshot + metadata + actions
- Filmstrip.svelte -- bottom panel: horizontal scrolling thumbnail strip
- FilmstripThumbnail.svelte -- individual thumbnail in the filmstrip
- SubtitleTrack.svelte -- transcription segments below filmstrip
- TimelineToolbar.svelte -- top bar with filters, calendar picker, Jump to Now

### Tauri Integration
- New Tauri command `get_captures_for_day(date: String)` -- returns all captures for a given date (ISO date string), ordered by timestamp ascending
- New Tauri command `get_capture_by_id(id: i64)` -- returns full capture row + associated OCR text + transcriptions for the time range
- Reuse existing `get_distinct_apps()` command for app filter dropdown
- Use Tauri asset protocol for thumbnail and full-res image loading
- Use Tauri clipboard plugin or `invoke` command for "Copy Image"

### Design Tokens
- Same Signal X Dark tokens as search UI
- Background: #0A0A0A
- Surface: #141414
- Elevated: #1C1C1C
- Border: #262626
- Text primary: #FAFAFA
- Text secondary: #A3A3A3
- Accent: #3B82F6
- Font: system San Francisco via -apple-system

### Scope Boundaries
**In Scope:**
- Timeline route in main window
- Stage with full screenshot + metadata
- Filmstrip with smart downsampling
- Subtitle track for transcriptions
- Calendar picker and Jump to Now
- App and content type filters
- Copy Image action
- Keyboard navigation (left/right arrows)

**Out of Scope:**
- Audio playback (separate spec)
- Editing or deleting captures
- Export functionality
- Multi-day view (single day at a time)
- Timeline in the search overlay (this is main window only)
