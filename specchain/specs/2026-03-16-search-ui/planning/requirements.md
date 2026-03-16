# Spec Requirements: Search UI

## Initial Description
Search UI — SvelteKit interface for searching captured history. Combined keyword + semantic search with filters (date range, application, content type). Results display the matched text, source screenshot thumbnail, timestamp, and app context. Global hotkey opens a floating search overlay from any application.

## Requirements Discussion

### First Round Questions

**Q1: Proof of Life**
**Answer:** Cmd+Shift+Space → Type "invoice" → Results appear instantly. Seeing the specific screenshot of the invoice from 4 days ago with the total amount highlighted, alongside the app icon (Chrome) and "4d ago" timestamp.

**Q2: Value Signal**
**Answer:** "Muscle Memory" — user stops using Cmd+F in individual apps and reflexively searches Cortex because it has context of everything they saw.

**Q3: Layout**
**Answer:** Raycast-style floating panel. Centered, ~700-800px wide, disappears on focus loss. Not a full-window experience.

**Q4: Search Mode**
**Answer:** Hybrid by default. Run both FTS5 and semantic in backend. Add "sparkle/brain" icon to semantic-only results so user knows why it appeared. No manual toggle needed.

**Q5: Result Cards**
**Answer:** Left: small screenshot thumbnail (click to enlarge / hover-zoom). Center: app icon + name + timestamp (top), text snippet with bolded keywords (bottom). Right: source badge (OCR or Audio).

**Q6: Global Hotkey**
**Answer:** Cmd+Shift+Space default. Configurable in future. Use Tauri global shortcut plugin.

**Q7: Filters**
**Answer:** Collapsible filter bar, appears on click of Filter icon or typing modifiers (/app: /date:). Prioritize app name and date range. Minimal by default.

**Q8: Click Action**
**Answer:** Primary: detail overlay within search window showing full-resolution screenshot + full extracted text. Quick actions: "Copy Text to Clipboard", "Open in Finder". For transcriptions: "Play Audio" button for that 30-second chunk.

**Q9: Design Direction**
**Answer:** "Signal X Dark" / Raycast-inspired. Deep charcoal/black (#0A0A0A) with subtle borders (#262626). Vibrant accent for highlights. San Francisco typography. Tailwind CSS.

**Q10: Out of Scope**
**Answer:** No chat/RAG, no timeline scrubbing, no settings management.

### Existing Code to Reference
- **search.rs** — Backend search with keyword + semantic modes, unified results
- **storage.rs** — All DB queries already implemented
- **embedding.rs** — EmbeddingEngine for semantic search queries
- **lib.rs** — Tauri commands already registered (search_captures, get_ocr_status, etc.)
- **tauri.conf.json** — Window configuration

## Requirements Summary

### Functional Requirements
- Floating search overlay window (Raycast-style, ~700-800px wide)
- Global hotkey Cmd+Shift+Space to toggle overlay
- Hide on blur (window disappears when focus lost)
- Always-on-top, with shadow
- Hybrid search: run both FTS5 + semantic, merge results
- Semantic-only results marked with sparkle icon
- Result cards: thumbnail, app icon+name, timestamp, snippet with bold highlights, source badge
- Detail overlay on click: full screenshot, full text, copy/finder actions
- Collapsible filter bar: app name, date range
- Instant feel: results appear as user types (debounced)

### UI Components (SvelteKit + Tailwind)
- SearchOverlay.svelte — main floating window
- SearchInput.svelte — input with debounce, filter toggle
- ResultCard.svelte — thumbnail + metadata + snippet
- ResultDetail.svelte — full screenshot + text + actions
- FilterBar.svelte — app dropdown, date range

### Tauri Integration
- New "search" window in tauri.conf.json (decorations: false, transparent: true, always_on_top: true)
- Global shortcut plugin for Cmd+Shift+Space
- window.set_shadow(true), hide on blur
- Invoke existing search_captures command from frontend

### Design Tokens
- Background: #0A0A0A
- Surface: #141414
- Border: #262626
- Text primary: #FAFAFA
- Text secondary: #A3A3A3
- Accent: vibrant highlight color (blue or cyan)
- Font: system San Francisco via -apple-system

### Scope Boundaries
**In Scope:**
- Search overlay window
- Global hotkey
- Hybrid search results
- Result cards with thumbnails
- Detail view with screenshot + text
- Copy to clipboard, open in Finder
- App and date filters

**Out of Scope:**
- Chat/RAG interface (spec #7)
- Timeline scrubbing view (spec #6)
- Settings UI (spec #12)
- Audio playback (deferred)
- Configurable hotkey (future)
