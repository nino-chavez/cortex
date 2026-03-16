# Cortex

Local-first Mac app for continuous screen/audio capture with on-device AI search and chat.

## Stack

- **Desktop:** Tauri v2 (Rust backend, native WebView)
- **Frontend:** SvelteKit (SPA mode) / Svelte 5 (runes) / TypeScript / Tailwind CSS v4
- **AI/ML:** MLX (chat, embeddings, whisper) — Ollama as dev fallback
- **Storage:** SQLite + sqlite-vec + FTS5 (via rusqlite)
- **macOS APIs:** ScreenCaptureKit, Apple Vision, Accessibility

## Commands

```bash
npm install          # Frontend dependencies
npm run dev          # SvelteKit dev server (port 5173)
npm run build        # Build frontend to build/
cargo tauri dev      # Run full Tauri app in dev mode
cargo tauri build    # Production build
```

## Project Structure

```
src/                 # SvelteKit frontend
src-tauri/           # Rust backend (Tauri)
  src/lib.rs         # Tauri app entry point
specchain/           # Spec-driven development workflow
  product/           # Mission, roadmap, tech stack
  specs/             # Feature specifications
```

## Conventions

- npm (not pnpm)
- Svelte 5 runes (`$state`, `$derived`, `$effect`)
- No emoji in code or commits
- Descriptive, conventional-ish commit messages
