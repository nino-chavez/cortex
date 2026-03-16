use serde::{Deserialize, Serialize};
use std::path::PathBuf;

fn config_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".cortex")
        .join("config.toml")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CortexConfig {
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub retention: RetentionConfig,
    #[serde(default)]
    pub privacy: PrivacyConfig,
    #[serde(default)]
    pub audio: AudioConfig,
}

impl Default for CortexConfig {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            retention: RetentionConfig::default(),
            privacy: PrivacyConfig::default(),
            audio: AudioConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub capture_interval_secs: u64,
    pub hotkey: String,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            capture_interval_secs: 5,
            hotkey: "CommandOrControl+Shift+Space".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionConfig {
    pub screenshots_days: u32,
    pub audio_days: u32,
    pub keep_text_forever: bool,
}

impl Default for RetentionConfig {
    fn default() -> Self {
        Self {
            screenshots_days: 30,
            audio_days: 7,
            keep_text_forever: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyConfig {
    pub excluded_apps: Vec<String>,
}

impl Default for PrivacyConfig {
    fn default() -> Self {
        Self {
            excluded_apps: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioConfig {
    pub system_audio_enabled: bool,
    pub microphone_enabled: bool,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            system_audio_enabled: false,
            microphone_enabled: false,
        }
    }
}

impl CortexConfig {
    pub fn load() -> Self {
        let path = config_path();
        if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(contents) => toml::from_str(&contents).unwrap_or_default(),
                Err(_) => Self::default(),
            }
        } else {
            let config = Self::default();
            config.save().ok();
            config
        }
    }

    pub fn save(&self) -> Result<(), String> {
        let path = config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let toml_str = toml::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(&path, toml_str).map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct StorageStats {
    pub total_bytes: u64,
    pub screenshots_bytes: u64,
    pub audio_bytes: u64,
    pub database_bytes: u64,
    pub capture_count: i64,
}

pub fn get_storage_stats() -> StorageStats {
    let cortex_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".cortex");

    let db_path = cortex_dir.join("cortex.db");
    let screenshots_dir = cortex_dir.join("screenshots");
    let audio_dir = cortex_dir.join("audio");

    StorageStats {
        total_bytes: dir_size(&cortex_dir),
        screenshots_bytes: dir_size(&screenshots_dir),
        audio_bytes: dir_size(&audio_dir),
        database_bytes: std::fs::metadata(&db_path).map(|m| m.len()).unwrap_or(0),
        capture_count: 0, // Filled by caller from DB
    }
}

fn dir_size(path: &PathBuf) -> u64 {
    if !path.exists() {
        return 0;
    }
    walkdir(path)
}

fn walkdir(path: &PathBuf) -> u64 {
    let mut total = 0u64;
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let meta = entry.metadata();
            if let Ok(m) = meta {
                if m.is_file() {
                    total += m.len();
                } else if m.is_dir() {
                    total += walkdir(&entry.path());
                }
            }
        }
    }
    total
}

/// Run retention cleanup: delete files older than configured days.
pub fn run_cleanup(config: &CortexConfig) -> CleanupResult {
    let cortex_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".cortex");

    let mut deleted_screenshots = 0u64;
    let mut deleted_audio = 0u64;

    let now = chrono::Utc::now();

    // Clean screenshots
    let screenshots_dir = cortex_dir.join("screenshots");
    if screenshots_dir.exists() {
        let cutoff = now - chrono::Duration::days(config.retention.screenshots_days as i64);
        deleted_screenshots = cleanup_old_files(&screenshots_dir, cutoff);
    }

    // Clean audio
    let audio_dir = cortex_dir.join("audio");
    if audio_dir.exists() {
        let cutoff = now - chrono::Duration::days(config.retention.audio_days as i64);
        deleted_audio = cleanup_old_files(&audio_dir, cutoff);
    }

    CleanupResult {
        deleted_screenshots,
        deleted_audio,
    }
}

fn cleanup_old_files(dir: &PathBuf, cutoff: chrono::DateTime<chrono::Utc>) -> u64 {
    let mut count = 0;
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                count += cleanup_old_files(&path, cutoff);
            } else if let Ok(meta) = path.metadata() {
                if let Ok(modified) = meta.modified() {
                    let modified: chrono::DateTime<chrono::Utc> = modified.into();
                    if modified < cutoff {
                        std::fs::remove_file(&path).ok();
                        count += 1;
                    }
                }
            }
        }
    }
    count
}

#[derive(Debug, Clone, Serialize)]
pub struct CleanupResult {
    pub deleted_screenshots: u64,
    pub deleted_audio: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_roundtrips_through_toml() {
        let config = CortexConfig::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: CortexConfig = toml::from_str(&toml_str).unwrap();

        assert_eq!(parsed.general.capture_interval_secs, 5);
        assert_eq!(parsed.retention.screenshots_days, 30);
        assert_eq!(parsed.retention.audio_days, 7);
        assert!(parsed.retention.keep_text_forever);
        assert!(parsed.privacy.excluded_apps.is_empty());
    }

    #[test]
    fn config_with_excluded_apps() {
        let mut config = CortexConfig::default();
        config.privacy.excluded_apps = vec![
            "com.1password".to_string(),
            "com.apple.keychainaccess".to_string(),
        ];

        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: CortexConfig = toml::from_str(&toml_str).unwrap();

        assert_eq!(parsed.privacy.excluded_apps.len(), 2);
        assert!(parsed.privacy.excluded_apps.contains(&"com.1password".to_string()));
    }

    #[test]
    fn storage_stats_struct_serializes() {
        let stats = StorageStats {
            total_bytes: 1024000,
            screenshots_bytes: 800000,
            audio_bytes: 200000,
            database_bytes: 24000,
            capture_count: 150,
        };
        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("1024000"));
    }
}
