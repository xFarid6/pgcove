//! App settings: theme preference and query defaults.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Light,
    #[default]
    Dark,
    System,
}

/// App settings persisted to disk (theme, query defaults).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    #[serde(default)]
    pub theme: Theme,
    /// Default row limit for table browse/query results.
    #[serde(default = "default_row_limit")]
    pub default_row_limit: u32,
    /// Statement timeout in seconds.
    #[serde(default = "default_statement_timeout")]
    pub default_statement_timeout: u32,
}

fn default_row_limit() -> u32 {
    50
}

fn default_statement_timeout() -> u32 {
    30
}

impl Default for AppSettings {
    fn default() -> Self {
        AppSettings {
            theme: Theme::default(),
            default_row_limit: default_row_limit(),
            default_statement_timeout: default_statement_timeout(),
        }
    }
}

fn settings_file(dir: &Path) -> PathBuf {
    dir.join("settings.json")
}

/// Load settings from disk; returns defaults if the file doesn't exist.
pub fn load(dir: &Path) -> Result<AppSettings, String> {
    let path = settings_file(dir);
    if !path.exists() {
        return Ok(AppSettings::default());
    }
    let json = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

/// Save settings to disk, creating the parent directory if needed.
pub fn save(dir: &Path, settings: &AppSettings) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let path = settings_file(dir);
    let json = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn default_settings_serialize() {
        let settings = AppSettings::default();
        let json = serde_json::to_string(&settings).unwrap();
        assert!(json.contains("\"theme\":\"dark\""));
        assert!(json.contains("\"defaultRowLimit\":50"));
        assert!(json.contains("\"defaultStatementTimeout\":30"));
    }

    #[test]
    fn settings_roundtrip() {
        let dir = TempDir::new().unwrap();
        let settings = AppSettings {
            theme: Theme::Light,
            default_row_limit: 100,
            default_statement_timeout: 60,
        };

        save(dir.path(), &settings).unwrap();
        let loaded = load(dir.path()).unwrap();

        assert_eq!(loaded.theme, Theme::Light);
        assert_eq!(loaded.default_row_limit, 100);
        assert_eq!(loaded.default_statement_timeout, 60);
    }

    #[test]
    fn missing_settings_returns_defaults() {
        let dir = TempDir::new().unwrap();
        let settings = load(dir.path()).unwrap();
        assert_eq!(settings, AppSettings::default());
    }
}
