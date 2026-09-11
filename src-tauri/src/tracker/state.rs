use std::time::Instant;
use serde::{Deserialize, Serialize};

/// Basic information regarding the active window and process.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActiveAppInfo {
    /// Process executable name (e.g. "chrome.exe", "Discord.exe").
    pub process_name: String,
    /// Window title text (e.g. "YouTube - Google Chrome").
    pub window_title: String,
    /// Operating system process ID (PID).
    pub process_id: u32,
}

/// Rich application metadata for UI rendering (friendly name and icon).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppItemInfo {
    /// Executable file name (e.g. "chrome.exe").
    pub exe_name: String,
    /// Human-friendly application name (e.g. "Google Chrome").
    pub display_name: String,
    /// Window title or description.
    pub window_title: String,
    /// Base64 encoded PNG icon (data:image/png;base64,...).
    pub icon_base64: Option<String>,
    /// Whether this application is currently blacklisted.
    pub is_tracked: bool,
}

/// Active focus session state for a tracked application.
#[derive(Debug, Clone)]
pub struct ActiveSession {
    /// Target application executable name.
    pub app_name: String,
    /// User-friendly application display name (e.g. "Discord").
    pub display_name: String,
    /// Session initialization timestamp.
    pub start_time: Instant,
    /// Accumulated active focus time in seconds (frozen when unfocused).
    pub continuous_seconds: u64,
    /// Number of alerts delivered during this session.
    pub alert_count: u32,
    /// Active second count at which the last alert fired.
    pub last_alert_seconds: u64,
}

impl ActiveSession {
    pub fn new(app_name: String, display_name: String) -> Self {
        Self {
            app_name,
            display_name,
            start_time: Instant::now(),
            continuous_seconds: 0,
            alert_count: 0,
            last_alert_seconds: 0,
        }
    }
}

/// Real-time tracking status broadcast to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackerStatus {
    /// Indicates whether the tracking engine loop is active.
    pub is_tracking: bool,
    /// Foreground application executable name (e.g. "discord.exe").
    pub current_app: Option<String>,
    /// User-friendly application display name (e.g. "Discord").
    pub current_app_display_name: Option<String>,
    /// Foreground window title.
    pub current_window_title: Option<String>,
    /// Accumulated focus duration on current application (seconds).
    pub continuous_seconds: u64,
    /// Indicates whether foreground application is blacklisted.
    pub is_blacklisted: bool,
    /// Initial alert threshold in seconds.
    pub initial_threshold_seconds: u64,
    /// Repeat alert interval threshold in seconds.
    pub repeat_threshold_seconds: u64,
    /// Number of alerts fired in the current session.
    pub alert_count: u32,
}

impl Default for TrackerStatus {
    fn default() -> Self {
        Self {
            is_tracking: true,
            current_app: None,
            current_app_display_name: None,
            current_window_title: None,
            continuous_seconds: 0,
            is_blacklisted: false,
            initial_threshold_seconds: 300,
            repeat_threshold_seconds: 120,
            alert_count: 0,
        }
    }
}
