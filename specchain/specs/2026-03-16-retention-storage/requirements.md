# Spec Requirements: Retention & Storage Management

## Initial Description
Configurable retention policies per content type with automatic and manual cleanup, a storage usage dashboard, and data export. Config lives in `~/.cortex/config.toml` (not in the DB), with a new `config.rs` module for TOML read/write. Background cleanup worker deletes expired content on startup and daily.

## Requirements Discussion

### First Round Questions

**Q1: Proof of Life**
**Answer:** User opens /settings/storage and sees total DB size, screenshot folder size, audio folder size, and capture count. User sets screenshot retention to 7 days. On next cleanup cycle (or manual "Run Cleanup" button), screenshots older than 7 days are deleted from disk and their rows removed from the captures table. OCR text in FTS5 is preserved (text retention defaults to forever). User exports last 30 days as a SQLite file and opens it in DB Browser to verify filtered data.

**Q2: Config Format**
**Answer:** TOML file at `~/.cortex/config.toml`. Parsed with the `toml` crate. Structure:

```toml
[retention]
screenshots_days = 30
audio_days = 7
text_days = 0          # 0 = keep forever
clipboard_days = 30

[storage]
data_dir = "~/.cortex"
```

If the file doesn't exist, create it with defaults on first read. If a key is missing, use the default value (forward-compatible).

**Q3: Cleanup Worker**
**Answer:** Background Rust thread that runs once on startup (after a 30-second delay to not block launch) and then every 24 hours. For each content type with a non-zero retention policy: query rows older than the cutoff date, delete associated files from disk, then delete rows from the database. Log the number of items cleaned per type. Cleanup is idempotent and skips items that are already gone.

**Q4: Storage Stats**
**Answer:** `get_storage_stats` Tauri command returns: total DB file size (bytes), screenshots directory size (walk and sum), audio directory size, capture count, transcription count, clipboard entry count, and a breakdown by content type showing count + disk size. Uses `std::fs::metadata` and directory walking.

**Q5: Export**
**Answer:** Export creates a new SQLite file at a user-chosen path (via Tauri save dialog). Copy the schema, then `INSERT INTO ... SELECT` with a date filter from the main DB. Include captures, captures_fts, transcriptions, transcriptions_fts, clipboard_entries, clipboard_fts, and meetings tables. Do NOT copy screenshot/audio files (too large) -- only metadata and text content. This makes exports small and portable.

**Q6: Out of Scope**
**Answer:** Cloud backup/sync, compression of existing screenshots, moving the data directory at runtime, per-app retention rules, retention policies for embeddings (they follow their source content), import from exported SQLite.

### Existing Code to Reference
- **storage.rs** -- Database struct, all table schemas, migration pattern.
- **lib.rs** -- `cortex_data_dir()` function for data directory path. Tauri command registration pattern.
- **capture.rs** -- Background thread pattern with stop flag for cleanup worker.

## Requirements Summary

### Functional Requirements
- New `config.rs` module: read/write `~/.cortex/config.toml` with `toml` crate
- Default retention policies: screenshots 30 days, audio 7 days, text forever (0), clipboard 30 days
- Background cleanup worker: runs on startup (30s delay) and every 24 hours
- Cleanup deletes expired files from disk and rows from DB
- `get_storage_stats` returns size breakdown by type and counts
- `export_data` creates filtered SQLite subset at user-chosen path
- `set_retention_policy` updates config.toml
- `run_cleanup` triggers manual cleanup

### Tauri Commands
- `get_storage_stats()` -- Returns StorageStats struct with sizes and counts
- `set_retention_policy(content_type: String, days: u32)` -- Updates config.toml
- `get_retention_policies()` -- Returns current policies from config.toml
- `run_cleanup()` -- Triggers immediate cleanup, returns count of deleted items
- `export_data(from: String, to: String, path: String)` -- Export date-filtered SQLite subset

### New Files
- `src-tauri/src/config.rs` -- TOML config read/write, RetentionConfig struct
- No schema migration needed (config is file-based, not in DB)

### Scope Boundaries
**In Scope:**
- TOML config file management
- Per-content-type retention policies
- Background cleanup worker
- Manual cleanup trigger
- Storage usage statistics
- SQLite subset export with date filtering
- Storage dashboard UI at /settings/storage

**Out of Scope:**
- Cloud backup or sync
- Screenshot compression optimization
- Moving data directory at runtime
- Per-app retention rules
- Embedding retention (follows source content)
- Import from exported SQLite
- File-level export (screenshots/audio files)
