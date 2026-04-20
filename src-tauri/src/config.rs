use crate::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// App configuration — loaded from ~/.config/claude-usage/config.toml
/// Falls back to defaults if file is missing or malformed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// How often to poll the API in seconds (default: 60)
    pub refresh_interval_seconds: u64,

    /// 5-hour window notification thresholds in percent (default: [75, 90])
    pub notify_thresholds_5h: Vec<u8>,

    /// 7-day window notification thresholds in percent (default: [90])
    pub notify_thresholds_7d: Vec<u8>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            refresh_interval_seconds: 60,
            notify_thresholds_5h: vec![75, 90],
            notify_thresholds_7d: vec![90],
        }
    }
}

impl Config {
    /// Load config from disk, falling back to defaults on any error
    pub fn load() -> Self {
        match Self::try_load() {
            Ok(config) => config,
            Err(e) => {
                tracing::warn!("Failed to load config (using defaults): {}", e);
                Config::default()
            }
        }
    }

    fn try_load() -> Result<Self> {
        let path = config_path()?;

        if !path.exists() {
            return Ok(Config::default());
        }

        let content = std::fs::read_to_string(&path)?;
        let config: Config = toml::from_str(&content)
            .map_err(|e| crate::AppError::Parse(e.to_string()))?;

        Ok(config)
    }

    /// Write default config to disk if no config file exists yet
    pub fn write_defaults_if_missing() -> Result<()> {
        let path = config_path()?;
        if path.exists() {
            return Ok(());
        }

        let defaults = Config::default();
        let content = toml::to_string_pretty(&defaults)
            .map_err(|e| crate::AppError::Parse(e.to_string()))?;

        let with_comments = format!(
            "# Claude Usage Tracker — Configuration\n\
             # All values are optional; defaults shown below.\n\n\
             {}",
            content
        );

        std::fs::write(&path, with_comments)?;
        Ok(())
    }
}

/// Path to ~/.config/claude-usage/config.toml
fn config_path() -> Result<PathBuf> {
    let base = dirs::config_dir().ok_or_else(|| {
        crate::AppError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Could not determine config directory",
        ))
    })?;

    let dir = base.join("claude-usage");
    std::fs::create_dir_all(&dir)?;
    Ok(dir.join("config.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.refresh_interval_seconds, 60);
        assert_eq!(config.notify_thresholds_5h, vec![75, 90]);
        assert_eq!(config.notify_thresholds_7d, vec![90]);
    }

    #[test]
    fn test_config_from_toml() {
        let toml_str = r#"
            refresh_interval_seconds = 30
            notify_thresholds_5h = [80, 95]
            notify_thresholds_7d = [85]
        "#;

        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.refresh_interval_seconds, 30);
        assert_eq!(config.notify_thresholds_5h, vec![80, 95]);
        assert_eq!(config.notify_thresholds_7d, vec![85]);
    }

    #[test]
    fn test_partial_config_uses_defaults() {
        // Only override one field — others should stay default
        let toml_str = r#"
            refresh_interval_seconds = 120
        "#;

        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.refresh_interval_seconds, 120);
        assert_eq!(config.notify_thresholds_5h, vec![75, 90]); // default
        assert_eq!(config.notify_thresholds_7d, vec![90]); // default
    }

    #[test]
    fn test_empty_toml_uses_all_defaults() {
        let config: Config = toml::from_str("").unwrap();
        assert_eq!(config.refresh_interval_seconds, 60);
        assert_eq!(config.notify_thresholds_5h, vec![75, 90]);
    }
}
