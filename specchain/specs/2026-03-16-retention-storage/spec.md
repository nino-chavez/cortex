# Specification: Retention & Storage Management

## Goal

Provide configurable retention policies per content type, automatic background cleanup, storage usage visibility, and data export -- all backed by a TOML config file at `~/.cortex/config.toml` with a new `config.rs` Rust module.

## Proof of Life

**Scenario:** User opens /settings/storage. Dashboard shows: cortex.db is 450 MB, screenshots/ is 2.1 GB (1,247 files), audio/ is 800 MB. User changes screenshot retention from 30 to 7 days and clicks "Run Cleanup Now." After cleanup completes, screenshots/ drops to 600 MB (412 files). User exports the last 7 days to ~/Desktop/cortex-export.db. Opening that file in DB Browser shows captures, transcriptions, and clipboard entries filtered to the last week, with all FTS5 tables intact.

**Validates:** Config persistence, cleanup logic, storage stats accuracy, and SQLite export with date filtering.

**Must work before:** Settings & Preferences UI (feature #12) which surfaces these controls.

## User Stories

- As a user, I want to set how long screenshots are kept so my disk doesn't fill up.
- As a user, I want to keep OCR text and transcriptions forever even if the source files are deleted.
- As a user, I want to see how much disk space Cortex is using, broken down by type.
- As a user, I want to export a subset of my data for backup or migration.
- As a user, I want cleanup to happen automatically without me thinking about it.

## Core Requirements

### Config Module (`config.rs`)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CortexConfig {
    pub retention: RetentionConfig,
    pub storage: StorageConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionConfig {
    pub screenshots_days: u32,  // 0 = forever, default 30
    pub audio_days: u32,        // 0 = forever, default 7
    pub text_days: u32,         // 0 = forever, default 0
    pub clipboard_days: u32,    // 0 = forever, default 30
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub data_dir: String,       // default "~/.cortex"
}
```

- `pub fn load_config() -> CortexConfig` -- Read from `~/.cortex/config.toml`. If missing or unparseable, return defaults and write the default file.
- `pub fn save_config(config: &CortexConfig) -> Result<()>` -- Write to `~/.cortex/config.toml`.
- Use `toml` crate for serialization. Use `serde` defaults for missing fields (forward-compatible).

### Cleanup Worker

- Background thread started in `lib.rs` setup, same pattern as OCR worker and clipboard watcher.
- Initial delay: 30 seconds after app launch.
- Interval: every 24 hours (86400 seconds).
- For each content type with `days > 0`:
  - Compute cutoff timestamp: `now - days`.
  - **Screenshots:** Query captures older than cutoff. Delete `image_path` files from disk. Delete rows from `captures`, `captures_fts`, `vec_captures`.
  - **Audio:** Query transcriptions older than cutoff where `audio_path != ''`. Delete audio files. If `text_days == 0`, keep the transcription row but clear `audio_path`. If `text_days > 0`, delete rows older than text cutoff.
  - **Clipboard:** Delete `clipboard_entries` and `clipboard_fts` rows older than cutoff.
- Log summary: "Cleanup complete: deleted X screenshots, Y audio files, Z clipboard entries."
- Return cleanup stats for manual trigger.

### Storage Stats

```rust
#[derive(Debug, Clone, Serialize)]
pub struct StorageStats {
    pub db_size_bytes: u64,
    pub screenshots_size_bytes: u64,
    pub screenshots_count: u64,
    pub audio_size_bytes: u64,
    pub audio_count: u64,
    pub capture_count: u64,
    pub transcription_count: u64,
    pub clipboard_count: u64,
    pub total_size_bytes: u64,
}
```

- `get_storage_stats`: Walk `~/.cortex/screenshots/` and `~/.cortex/audio/` directories, sum file sizes. Query DB for counts. Return `StorageStats`.

### Data Export

- `export_data(from, to, path)`:
  - Create new SQLite DB at `path`.
  - Create same schema (captures, captures_fts, transcriptions, transcriptions_fts, clipboard_entries, clipboard_fts, meetings).
  - `ATTACH DATABASE ? AS export`.
  - `INSERT INTO export.captures SELECT * FROM captures WHERE timestamp >= ? AND timestamp <= ?`.
  - Same for transcriptions (filter on `timestamp_start`), clipboard_entries (filter on `timestamp`), meetings (filter on `start_time`).
  - Rebuild FTS5 tables in export DB from the copied rows.
  - `DETACH DATABASE export`.
- Use Tauri save dialog for path selection on the frontend side.

### Testing

- Unit test: `load_config` returns defaults when no file exists.
- Unit test: `save_config` + `load_config` round-trip.
- Unit test: cleanup deletes rows older than cutoff, preserves newer rows.
- Unit test: `get_storage_stats` returns correct counts from test DB.
- Unit test: `export_data` creates a valid SQLite file with filtered rows.

## Out of Scope

- Cloud backup or sync
- Screenshot compression
- Moving data directory at runtime
- Per-app retention policies
- Embedding-specific retention
- Importing from exported files

## Success Criteria

- Config file created with defaults on first launch.
- Retention policy changes persist across app restarts.
- Background cleanup runs automatically and deletes expired content.
- Manual cleanup returns accurate deletion counts.
- Storage stats reflect actual disk usage within 5% accuracy.
- Exported SQLite files are valid and contain only filtered data.
