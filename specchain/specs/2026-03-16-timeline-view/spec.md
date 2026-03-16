# Specification: Timeline View

## Goal

Build a full-window visual timeline browser at `/timeline` in the main Cortex window that lets the user scrub through their captured history like a video editor. A large center Stage displays the active capture at full resolution with OCR and transcription metadata, while a horizontal filmstrip at the bottom provides spatial/temporal navigation with smart downsampling (1 thumbnail/minute default, 5-second granularity on zoom). Transcription segments appear as a subtitle track below the filmstrip. Filters, a calendar picker, and keyboard navigation complete the experience -- all styled with Signal X Dark design tokens using SvelteKit + Tailwind CSS.

## Proof of Life

**Scenario:** User clicks the Timeline/History tab in the main Cortex window. The filmstrip populates with thumbnails from today's captures (downsampled to 1 per minute). They scrub horizontally, click a thumbnail at 2:35 PM, and the Stage loads the full-resolution screenshot showing their Figma window. Below the screenshot, the OCR text reads "Component Library v2 -- Button variants" and a transcription segment shows "...let's finalize the button states before the review." The user presses the right arrow key to step to the next capture.
**Validates:** Day-based capture loading via new Tauri commands, filmstrip rendering with smart downsampling, Stage display with full metadata, transcription subtitle alignment, and keyboard navigation.
**Must work before:** Smart summaries, meeting memory view, or any feature that builds on temporal browsing.

## User Stories

- As a knowledge worker, I want to visually scrub through my day's captures so I can recall context faster than text search alone.
- As a user viewing my timeline, I want a large Stage showing the full screenshot and metadata so I can see exactly what was on screen.
- As a user navigating the filmstrip, I want smart downsampling so the UI stays fast even on days with thousands of captures.
- As a user zooming into a time range, I want to see 5-second granularity so I can pinpoint the exact moment I need.
- As a user, I want to see transcription segments aligned to the timeline so I know what was being said at any point.
- As a user, I want to filter by app and content type so I can focus on specific contexts.
- As a user, I want to copy the current screenshot to my clipboard so I can paste it into a document or message.
- As a user, I want a calendar picker to navigate to previous days and a "Jump to Now" button to return to the current time.
- As a user, I want to use arrow keys to step through captures so I can navigate without the mouse.

## Core Requirements

### Functional Requirements

1. **Timeline route** -- `/timeline` route in the main Cortex window (not the search overlay). Full-window layout with three zones: toolbar (top), Stage (center), filmstrip (bottom).
2. **Stage component** -- Center of the window, takes the majority of vertical space. Displays the active capture's full-resolution screenshot (loaded via Tauri asset protocol), app name, window title, timestamp, OCR text (from `captures_fts`), and any transcription segments that overlap the capture's time range. Includes a "Copy Image" button.
3. **Filmstrip component** -- Fixed-height horizontal strip at the bottom of the window. Contains a scrollable row of thumbnail images representing the day's captures. Supports momentum/inertia scrolling. A thin time axis with hour markers runs along the top of the filmstrip. Clicking a thumbnail selects it and loads it in the Stage.
4. **Smart downsampling** -- At default zoom, the filmstrip shows 1 thumbnail per minute (selecting the first capture in each 1-minute bucket). On zoom-in (scroll wheel or pinch), granularity increases to 1 per 30 seconds, then 1 per 5 seconds. On hover over a filmstrip region, temporarily expand that region to show 5-second granularity without changing overall zoom. This keeps the filmstrip performant: a 10-hour workday at 1/min = ~600 thumbnails vs ~7,200 at 5-second capture intervals.
5. **Subtitle track** -- Below the filmstrip thumbnails, a secondary row displays transcription segments. Each segment is a small pill showing truncated text, positioned to align with its `timestamp_start`/`timestamp_end` time range. Clicking a transcription pill loads the nearest capture in the Stage and highlights the transcription text.
6. **Toolbar/filter bar** -- Top bar with: calendar date picker (defaults to today), "Jump to Now" button, app name dropdown filter (populated from `get_distinct_apps()`), content type toggle (screenshots / transcriptions / both).
7. **Copy Image action** -- Button on the Stage panel that copies the current full-resolution screenshot to the system clipboard. Uses Tauri command to read the image file and write to clipboard as PNG data.
8. **Calendar picker** -- Date selector that loads captures for a different day. When the date changes, invokes `get_captures_for_day(date)` and resets the filmstrip and Stage.
9. **Jump to Now** -- Button that scrolls the filmstrip to the current time position and selects the most recent capture. Only active when viewing today's date.
10. **Keyboard navigation** -- Left arrow selects the previous capture in the filmstrip, right arrow selects the next. Holding an arrow key rapid-advances at ~5 captures/second. Home key jumps to the first capture of the day, End key jumps to the last.

### Non-Functional Requirements

1. **Performance** -- Filmstrip must render smoothly with up to 600 visible thumbnails (1/min for 10 hours). Thumbnails load lazily and are virtualized (only DOM nodes for visible thumbnails + buffer). Target: filmstrip scrolling at 60fps.
2. **Visual consistency** -- All components follow Signal X Dark design tokens, consistent with the search UI.
3. **Memory** -- Full-resolution images are only loaded for the active Stage capture. Filmstrip thumbnails use smaller/compressed versions. Thumbnails outside the viewport are unloaded.
4. **Responsiveness** -- Stage image scales to fit available space while maintaining aspect ratio. Filmstrip height is fixed (~120px thumbnails + ~32px subtitle track + ~24px time axis).

## Visual Design

### Design Tokens (Signal X Dark)

| Token | Value | Usage |
|---|---|---|
| `--bg-primary` | `#0A0A0A` | Page background |
| `--bg-surface` | `#141414` | Stage panel, filmstrip background |
| `--bg-elevated` | `#1C1C1C` | Active thumbnail, hover states |
| `--border-default` | `#262626` | Panel borders, time axis lines |
| `--border-focus` | `#3B82F6` | Selected thumbnail border |
| `--text-primary` | `#FAFAFA` | Stage metadata, toolbar text |
| `--text-secondary` | `#A3A3A3` | Timestamps, OCR text |
| `--text-muted` | `#737373` | Placeholder, time axis labels |
| `--accent` | `#3B82F6` | Selected thumbnail ring, active states |
| `--accent-transcript` | `#A78BFA` | Transcription pills in subtitle track |
| `--badge-ocr` | `#22D3EE` | OCR indicator on Stage |
| `--badge-audio` | `#F472B6` | Transcription indicator on Stage |

### Typography

- Font stack: `-apple-system, BlinkMacSystemFont, "SF Pro Text", "SF Pro Display", system-ui, sans-serif`
- Stage app name: 16px, font-weight 600
- Stage window title: 14px, font-weight 400, `--text-secondary`
- Stage timestamp: 13px, font-weight 400, `--text-secondary`
- Stage OCR text: 13px, `font-family: "SF Mono", "JetBrains Mono", monospace`, `--text-secondary`
- Filmstrip time axis: 11px, font-weight 400, `--text-muted`
- Subtitle track pills: 11px, font-weight 500

### Layout

- Toolbar: full width, 48px height, `--bg-surface` background, `--border-default` bottom border
- Stage: flexible height (fills space between toolbar and filmstrip), 24px padding, screenshot centered with `object-contain`
- Filmstrip: fixed at bottom, total height ~176px (120px thumbnails + 32px subtitle track + 24px time axis)
- Filmstrip thumbnails: 96px wide x 120px tall (16:10ish aspect ratio matching typical Mac screens), 4px gap, 2px border-radius
- Selected thumbnail: 2px `--accent` border ring
- Subtitle track pills: 20px height, variable width based on duration, `--accent-transcript` background at 20% opacity, `--accent-transcript` text

## Conversion Design

N/A -- this is a browsing/recall interface, not a conversion flow.

## Reusable Components

### Existing Code to Leverage

- **`CaptureRow` struct** (`src-tauri/src/storage.rs`) -- Contains `id`, `timestamp`, `app_name`, `bundle_id`, `window_title`, `display_id`, `image_path`, `image_hash`, `is_private`. Serializes to JSON for the frontend.
- **`captures` table** -- Has `timestamp` (ISO string), `app_name`, `image_path` columns. Indexed on `timestamp` and `app_name`.
- **`captures_fts` table** -- FTS5 virtual table with `capture_id` and `ocr_text`. Join on `capture_id` to get OCR text for a capture.
- **`transcriptions` table** -- Has `capture_id`, `timestamp_start`, `timestamp_end`, `text`, `source`. Indexed on `timestamp_start`.
- **`get_distinct_apps()` Tauri command** -- Already exists (or will from search UI spec). Returns distinct app names.
- **Signal X Dark tokens** -- Defined in `src/app.css` from search UI spec.
- **`relativeTime()` utility** -- In `src/lib/utils/time.ts` from search UI spec.
- **Asset protocol** -- Configured in `tauri.conf.json` for loading screenshots from `~/.cortex/screenshots/`.
- **`ResultCard.svelte`** -- Existing component with Signal X Dark styling patterns to reference.

### New Components Required

- **`src/routes/timeline/+page.svelte`** -- Timeline route. Renders the three-zone layout (toolbar, stage, filmstrip).
- **`src/lib/components/timeline/Stage.svelte`** -- Active capture display: full-res screenshot, app metadata, OCR text, transcriptions, Copy Image button.
- **`src/lib/components/timeline/Filmstrip.svelte`** -- Horizontal scrolling thumbnail strip with time axis, smart downsampling, and virtualized rendering.
- **`src/lib/components/timeline/FilmstripThumbnail.svelte`** -- Individual thumbnail in the filmstrip with selection state.
- **`src/lib/components/timeline/SubtitleTrack.svelte`** -- Transcription pills aligned to the filmstrip's time axis.
- **`src/lib/components/timeline/TimelineToolbar.svelte`** -- Top bar with calendar picker, Jump to Now, app filter, content type filter.

### New Tauri Commands Required

- **`get_captures_for_day(date: String) -> Vec<CaptureRow>`** -- Query: `SELECT * FROM captures WHERE date(timestamp) = ?1 ORDER BY timestamp ASC`. Returns all captures for the given date, ordered chronologically. The frontend handles downsampling.
- **`get_capture_by_id(id: i64) -> CaptureDetail`** -- Returns the full `CaptureRow` plus joined OCR text from `captures_fts` and any transcription segments from `transcriptions` where `timestamp_start` falls within a reasonable window (e.g., +/- 30 seconds of the capture's timestamp). Returns a new `CaptureDetail` struct that combines capture metadata, OCR text, and related transcriptions.
- **`copy_image_to_clipboard(path: String)`** -- Reads the image file at the given path and writes it to the system clipboard as PNG data. Uses `NSPasteboard` via Swift bridge or the `arboard` Rust crate.

## Technical Approach

- **Route structure:** Add `/timeline` route in SvelteKit. This renders in the main Cortex window (not the search overlay window). Add navigation between the main view and timeline (tab or sidebar).
- **Day loading:** On mount and date change, call `get_captures_for_day(date)` to fetch all captures for the day. Store the full list in `$state`. The filmstrip component handles downsampling from this full list based on current zoom level.
- **Smart downsampling algorithm:** Given the full capture list and a zoom level, bucket captures by time interval (60s, 30s, or 5s). Pick the first capture in each bucket as the representative thumbnail. Zoom level is controlled by scroll wheel (Ctrl+scroll or pinch) on the filmstrip. Store zoom level in `$state`. Recompute downsampled list via `$derived`.
- **Hover expansion:** When the mouse hovers over a region of the filmstrip, temporarily expand a ~5-minute window around the cursor to 5-second granularity. This creates a "lens" effect where the hovered area shows more detail. Implement by tracking mouse position relative to the filmstrip's time range and injecting additional thumbnails into that region.
- **Virtualized filmstrip:** Only render DOM nodes for thumbnails visible in the viewport plus a buffer (e.g., 10 thumbnails on each side). Use `IntersectionObserver` or manual scroll position calculation. As the user scrolls, recycle/create thumbnail nodes. This is critical for performance -- even at 1/min, a 10-hour day is 600 thumbnails.
- **Thumbnail images:** Use the same full screenshots but load them as small thumbnails. The `<img>` element's CSS constrains display size; the browser handles downscaling. For future optimization, generate actual thumbnail files (e.g., 200px wide WebP) during capture. For now, lazy-load the full image via asset protocol with `loading="lazy"` and `decoding="async"`.
- **Stage rendering:** When a capture is selected (via filmstrip click, keyboard nav, or subtitle track click), call `get_capture_by_id(id)` to fetch full metadata + OCR + transcriptions. Display the full-res image in the Stage with `object-contain` to fit the available space. Show metadata in a sidebar or overlay panel within the Stage area.
- **Subtitle track positioning:** The subtitle track is a horizontal bar below the filmstrip. Each transcription segment is positioned absolutely based on its `timestamp_start` and `timestamp_end` relative to the day's time range. Width is proportional to duration. This requires the same time-to-pixel mapping as the filmstrip.
- **Copy Image:** The `copy_image_to_clipboard` Tauri command reads the PNG/WebP file, decodes it if needed, and writes PNG data to the system clipboard. On macOS, use `NSPasteboard.general` with `NSImage` via the `arboard` crate or a Swift bridge.
- **Svelte 5 patterns:** All components use runes: `$state` for captures list, selected ID, zoom level, date, filters. `$derived` for downsampled captures, filtered captures, time-to-pixel mapping. `$effect` for data fetching on date/filter change and keyboard event listeners. Props via `$props()`.
- **Styling:** Tailwind CSS v4 utility classes with Signal X Dark CSS custom properties. Dark mode only.

## Out of Scope

- Audio playback (separate spec)
- Editing or deleting captures from the timeline
- Export functionality (screenshots, OCR text, transcriptions)
- Multi-day continuous view (single day at a time, switch via calendar)
- Timeline view in the search overlay (main window only)
- Meeting-specific grouped view (separate meeting memory spec)
- Thumbnail generation pipeline (use full images for now)
- Drag-to-select time ranges

## Success Criteria

1. `/timeline` route loads in the main Cortex window and displays today's captures.
2. Filmstrip shows ~1 thumbnail per minute and scrolls smoothly at 60fps.
3. Clicking a filmstrip thumbnail loads the full screenshot and metadata in the Stage within 200ms.
4. Zooming into the filmstrip increases granularity to 5-second intervals.
5. Hovering over a filmstrip region temporarily expands it to show finer detail.
6. Transcription segments appear as pills in the subtitle track, aligned to the correct time positions.
7. "Copy Image" copies the current Stage screenshot to the system clipboard.
8. Calendar picker loads captures for a different day.
9. "Jump to Now" scrolls to the current time and selects the most recent capture.
10. App filter and content type filter narrow the displayed captures.
11. Left/right arrow keys step through captures in chronological order.
12. All components render correctly with Signal X Dark design tokens.
