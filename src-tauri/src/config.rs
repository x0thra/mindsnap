use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use crate::error::{MindsnapError, Result};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppConfig {
    /// Continuous focus duration before the first alert is triggered (minutes).
    pub initial_alert_minutes: u32,

    /// Interval for repeated alerts if the user stays in the target app (minutes).
    pub repeat_alert_minutes: u32,

    /// Executable names tracked for focus limit (e.g., "chrome.exe", "discord.exe").
    pub blacklisted_apps: Vec<String>,

    /// Global flag indicating whether notifications are active.
    pub notifications_enabled: bool,

    /// Sound playback alongside notifications.
    pub sound_enabled: bool,

    /// Minimize application to system tray on window close.
    pub minimize_to_tray_on_close: bool,

    /// User locale preference ("auto", "tr_tr", "en_us", etc.).
    #[serde(default = "default_language")]
    pub language: String,
}

fn default_language() -> String {
    "auto".to_string()
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            initial_alert_minutes: 5,
            repeat_alert_minutes: 2,
            blacklisted_apps: Vec::new(),
            notifications_enabled: true,
            sound_enabled: true,
            minimize_to_tray_on_close: true,
            language: default_language(),
        }
    }
}

impl AppConfig {
    /// Resolves the filesystem path for configuration storage.
    pub fn get_config_path() -> PathBuf {
        if let Some(config_dir) = dirs::config_dir() {
            let app_dir = config_dir.join("mindsnap");
            if let Err(err) = fs::create_dir_all(&app_dir) {
                eprintln!("Failed to create config directory: {}", err);
            } else {
                return app_dir.join("config.json");
            }
        }
        PathBuf::from("mindsnap_config.json")
    }

    pub fn sanitize(&mut self) {
        self.notifications_enabled = true;
        self.blacklisted_apps
            .retain(|app| !crate::tracker::is_system_or_ignored_app(app));
    }

    /// Loads configuration from disk, creating and persisting defaults on failure.
    pub fn load() -> Self {
        let path = Self::get_config_path();
        if path.exists() {
            match fs::read_to_string(&path) {
                Ok(content) => match serde_json::from_str::<AppConfig>(&content) {
                    Ok(mut config) => {
                        let original_len = config.blacklisted_apps.len();
                        config.sanitize();
                        if config.blacklisted_apps.len() != original_len {
                            let _ = config.save();
                        }
                        return config;
                    }
                    Err(err) => {
                        eprintln!("Failed to parse config file, falling back to default: {}", err);
                    }
                },
                Err(err) => {
                    eprintln!("Failed to read config file: {}", err);
                }
            }
        }

        let default_config = Self::default();
        if let Err(err) = default_config.save() {
            eprintln!("Failed to persist default configuration: {}", err);
        }
        default_config
    }

    /// Serializes configuration to disk as formatted JSON.
    pub fn save(&self) -> Result<()> {
        let path = Self::get_config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(MindsnapError::Io)?;
        }

        let json_data = serde_json::to_string_pretty(self).map_err(MindsnapError::Serialization)?;
        let mut file = File::create(&path).map_err(MindsnapError::Io)?;
        file.write_all(json_data.as_bytes()).map_err(MindsnapError::Io)?;
        file.flush().map_err(MindsnapError::Io)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.initial_alert_minutes, 5);
        assert_eq!(config.repeat_alert_minutes, 2);
        assert!(config.notifications_enabled);
        assert!(config.minimize_to_tray_on_close);
        assert!(config.blacklisted_apps.is_empty());
        assert_eq!(config.language, "auto");
    }

    #[test]
    fn test_config_serialization() {
        let config = AppConfig::default();
        let serialized = serde_json::to_string(&config);
        assert!(serialized.is_ok());

        let unwrapped_json = serialized.unwrap_or_default();
        let deserialized: std::result::Result<AppConfig, _> = serde_json::from_str(&unwrapped_json);
        assert!(deserialized.is_ok());
        if let Ok(deserialized_config) = deserialized {
            assert_eq!(config, deserialized_config);
        }
    }
}
