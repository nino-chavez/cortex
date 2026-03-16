# Task Breakdown: Retention & Storage Management

## Overview
Total Tasks: 2 groups, 13 subtasks
Strategy: squad
Depth: standard

## Task List

### Proof of Life -- Config, Cleanup, Stats

#### Task Group 1: config.rs, retention policies, cleanup worker, storage stats
**Dependencies:** Capture Daemon (complete -- background thread pattern), Clipboard History (complete -- clipboard_entries table exists)

This group delivers the config module, cleanup worker, and storage stats command. No UI yet -- testable via Tauri commands.

- [ ] 1.0 Complete config module, cleanup worker, and storage stats
  - [ ] 1.1 Add `toml` crate to `src-tauri/Cargo.toml` dependencies.
  - [ ] 1.2 Create `src-tauri/src/config.rs`:
    - Define `CortexConfig`, `RetentionConfig`, `StorageConfig` structs with `Serialize`/`Deserialize`.
    - `impl Default` for each with: screenshots_days=30, audio_days=7, text_days=0, clipboard_days=30, data_dir="~/.cortex".
    - `pub fn config_path() -> PathBuf` -- returns `~/.cortex/config.toml`.
    - `pub fn load_config() -> CortexConfig` -- read file with `std::fs::read_to_string`, parse with `toml::from_str`. On any error, return defaults and write default file.
    - `pub fn save_config(config: &CortexConfig) -> Result<()>` -- serialize with `toml::to_string_pretty`, write to config_path.
  - [ ] 1.3 Create cleanup module in `src-tauri/src/cleanup.rs`:
    - `pub fn run_cleanup(db: &Database, config: &CortexConfig) -> CleanupStats` where `CleanupStats` has `screenshots_deleted`, `audio_deleted`, `clipboard_deleted` fields.
    - Screenshot cleanup: if `screenshots_days > 0`, compute cutoff. Query `SELECT id, image_path FROM captures WHERE timestamp < cutoff`. Delete each file with `std::fs::remove_file`. Delete from `captures`, `captures_fts`, `vec_captures`.
    - Audio cleanup: if `audio_days > 0`, compute cutoff. Query transcriptions with `audio_path != ''` older than cutoff. Delete audio files. If `text_days == 0`, keep row but set `audio_path = ''`. If `text_days > 0` and row is older than text cutoff, delete row entirely.
    - Clipboard cleanup: if `clipboard_days > 0`, delete `clipboard_entries` and `clipboard_fts` rows older than cutoff.
    - `pub fn start_cleanup_worker(db: Arc<Database>, stop_flag: Arc<AtomicBool>)` -- background thread: sleep 30s, run cleanup, then sleep 24h in a loop. Check stop_flag each cycle.
  - [ ] 1.4 Add storage stats methods to `Database` in `storage.rs`:
    - `get_transcription_count() -> Result<i64>` -- `SELECT COUNT(*) FROM transcriptions`.
    - `get_clipboard_count() -> Result<i64>` -- `SELECT COUNT(*) FROM clipboard_entries`.
    - `delete_captures_before(cutoff: &str) -> Result<Vec<String>>` -- returns deleted image_paths.
    - `delete_audio_before(cutoff: &str, preserve_text: bool) -> Result<Vec<String>>` -- returns deleted audio_paths.
    - `delete_clipboard_before(cutoff: &str) -> Result<i64>` -- returns count deleted.
  - [ ] 1.5 Create `get_storage_stats` function:
    - Walk `~/.cortex/screenshots/` with `std::fs::read_dir` recursively, sum file sizes and count.
    - Walk `~/.cortex/audio/` similarly.
    - Get `cortex.db` file size via `std::fs::metadata`.
    - Query DB for capture_count, transcription_count, clipboard_count.
    - Return `StorageStats` struct.
  - [ ] 1.6 Register Tauri commands in `lib.rs`:
    - `#[tauri::command] get_storage_stats()` -- calls stats function, returns StorageStats.
    - `#[tauri::command] get_retention_policies()` -- calls `config::load_config().retention`.
    - `#[tauri::command] set_retention_policy(content_type: String, days: u32)` -- load config, update field, save.
    - `#[tauri::command] run_cleanup()` -- calls `cleanup::run_cleanup`, returns CleanupStats.
    - Add `mod config; mod cleanup;` in lib.rs. Start cleanup worker in setup block.
  - [ ] 1.7 Write 5 tests:
    - (a) `load_config` returns defaults when config.toml doesn't exist.
    - (b) `save_config` + `load_config` round-trip preserves all fields.
    - (c) `run_cleanup` deletes captures older than cutoff, preserves newer ones.
    - (d) `get_storage_stats` returns correct capture_count from test DB.
    - (e) `delete_clipboard_before` removes correct entries.

**Acceptance Criteria:**
- config.toml created with defaults on first load
- Retention policy changes persist after save_config + load_config
- Cleanup worker starts in background and runs on schedule
- Manual cleanup returns accurate deletion counts
- Storage stats return correct sizes and counts
- All 5 tests pass

**Verification Commands:**
```bash
cargo build --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml --lib -- config
cargo test --manifest-path src-tauri/Cargo.toml --lib -- cleanup
cargo test --manifest-path src-tauri/Cargo.toml --lib -- storage::tests
```

---

### Export + Storage Dashboard UI

#### Task Group 2: Data export and /settings/storage dashboard
**Dependencies:** Task Group 1 (config, cleanup, and stats commands work)

- [ ] 2.0 Complete data export and storage dashboard UI
  - [ ] 2.1 Add `export_data` function in `cleanup.rs` (or a new `export.rs`):
    - Accept `from: &str, to: &str, export_path: &str`.
    - Create new SQLite DB at export_path.
    - Create schema tables (captures, transcriptions, clipboard_entries, meetings -- no FTS/vec).
    - Use `ATTACH DATABASE ? AS export_db`.
    - Insert filtered rows from each table by date range.
    - Detach and return export stats (row counts per table).
  - [ ] 2.2 Register `export_data` Tauri command in `lib.rs`:
    - `#[tauri::command] export_data(from: String, to: String)` -- use Tauri save dialog for path. Call export function. Return export stats.
  - [ ] 2.3 Create `/settings/storage` route -- `src/routes/settings/storage/+page.svelte`:
    - Call `get_storage_stats()` on mount. Display: total size (formatted), DB size, screenshots size + count, audio size + count, clipboard count.
    - Use a simple bar or grid layout for the breakdown.
  - [ ] 2.4 Add retention policy controls to the storage page:
    - For each content type (screenshots, audio, text, clipboard): show current retention days and a number input or dropdown to change it.
    - On change, call `set_retention_policy(type, days)`.
    - Show "forever" label when days = 0.
  - [ ] 2.5 Add "Run Cleanup Now" button:
    - Calls `run_cleanup()`. Shows spinner during execution.
    - On completion, display results ("Deleted 142 screenshots, 38 audio files, 0 clipboard entries").
    - Refresh storage stats after cleanup.
  - [ ] 2.6 Add export section to storage page:
    - Date range picker (from/to).
    - "Export" button calls `export_data(from, to)`.
    - Show success message with file path and row counts.

**Acceptance Criteria:**
- /settings/storage displays accurate storage breakdown
- Retention policy changes update config.toml immediately
- "Run Cleanup Now" deletes expired content and refreshes stats
- Export creates a valid SQLite file with filtered data
- Date range picker works for export filtering

**Verification Commands:**
```bash
npm run tauri dev
# Navigate to /settings/storage
# Verify stats display, change retention, run cleanup, export data
```

---

## Execution Order

1. **Task Group 1: Backend** -- Config module, cleanup worker, storage stats, Tauri commands.
2. **Task Group 2: UI + Export** -- Depends on Group 1 for working backend. Delivers storage dashboard and export.
