pub mod platform;
pub mod state;

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

use crate::config::AppConfig;
use crate::tracker::state::{ActiveSession, TrackerStatus};

/// Checks if an application belongs to system utilities, background services, or Mindsnap itself.
pub fn is_system_or_ignored_app(exe_name: &str) -> bool {
    platform::is_platform_ignored_app(exe_name)
}

/// Checks if a reverse-DNS domain segment is generic (e.g. TLD, organization type, or role suffix).
pub fn is_generic_reverse_dns_segment(s: &str) -> bool {
    matches!(
        s.to_ascii_lowercase().as_str(),
        "com"
            | "org"
            | "net"
            | "io"
            | "dev"
            | "im"
            | "app"
            | "apps"
            | "client"
            | "browser"
            | "desktop"
            | "application"
            | "launcher"
            | "ui"
            | "gui"
            | "main"
            | "bin"
            | "standalone"
            | "linux"
    )
}

/// Checks if a candidate process string and a rule string match across reverse-DNS naming conventions.
/// For example: "dev.geopjr.Tuba" <=> "tuba", "com.spotify.Client" <=> "spotify", "org.gnome.Nautilus" <=> "nautilus".
pub fn matches_reverse_dns(a: &str, b: &str) -> bool {
    let a_clean = a.trim().to_lowercase();
    let b_clean = b.trim().to_lowercase();

    if a_clean == b_clean {
        return true;
    }

    let (rdns, other) = if a_clean.contains('.') && !b_clean.contains('.') {
        (a_clean.as_str(), b_clean.as_str())
    } else if b_clean.contains('.') && !a_clean.contains('.') {
        (b_clean.as_str(), a_clean.as_str())
    } else {
        return false;
    };

    let segments: Vec<&str> = rdns.split('.').collect();
    if segments.is_empty() {
        return false;
    }

    for seg in segments.iter().rev() {
        if !is_generic_reverse_dns_segment(seg) {
            let norm_seg = seg.replace('_', "-");
            let norm_other = other.replace('_', "-");
            if norm_seg == norm_other
                || norm_seg.starts_with(&format!("{norm_other}-"))
                || norm_other.starts_with(&format!("{norm_seg}-"))
            {
                return true;
            }
            break;
        }
    }

    for seg in &segments {
        if !is_generic_reverse_dns_segment(seg) && seg.len() >= 3 {
            let norm_seg = seg.replace('_', "-");
            let norm_other = other.replace('_', "-");
            if norm_seg == norm_other {
                return true;
            }
        }
    }

    false
}

/// Checks whether an application is blacklisted (cross-platform matching).
pub fn is_app_blacklisted(process_name: &str, window_title: &str, blacklist: &[String]) -> bool {
    // Internal system tools and Mindsnap itself are never blacklisted
    if is_system_or_ignored_app(process_name) {
        return false;
    }

    let proc_lower = process_name.trim().to_lowercase();
    let title_lower = window_title.trim().to_lowercase();
    let proc_base = proc_lower.strip_suffix(".exe").unwrap_or(&proc_lower);
    let proc_base = proc_base.strip_suffix(".desktop").unwrap_or(proc_base);

    for rule in blacklist {
        let rule_clean = rule.trim().to_lowercase();
        if rule_clean.is_empty() {
            continue;
        }

        let rule_base = rule_clean.strip_suffix(".exe").unwrap_or(&rule_clean);
        let rule_base = rule_base.strip_suffix(".desktop").unwrap_or(rule_base);

        // 1. Direct process name match (e.g. "chrome.exe" == "chrome.exe" or "discord" == "discord")
        if proc_lower == rule_clean {
            return true;
        }

        // 2. Base name match without platform extension (e.g. "discord.exe" <=> "discord")
        if proc_base == rule_base {
            return true;
        }

        // 3. Common packaging prefix/suffix and separator match (e.g. "brave-browser" <=> "brave", "google_chrome" <=> "google-chrome")
        let norm_proc = proc_base.replace('_', "-");
        let norm_rule = rule_base.replace('_', "-");
        if norm_proc == norm_rule
            || norm_proc.starts_with(&format!("{norm_rule}-"))
            || norm_rule.starts_with(&format!("{norm_proc}-"))
        {
            return true;
        }

        // 4. Reverse-DNS application ID match (e.g. "dev.geopjr.Tuba" <=> "tuba", "com.spotify.Client" <=> "spotify")
        if matches_reverse_dns(proc_base, rule_base) {
            return true;
        }

        // 5. Substring match inside window title (only for meaningful terms of at least 3 characters)
        if rule_base.len() >= 3 && title_lower.contains(rule_base) {
            return true;
        }
    }

    false
}

/// Determines whether an active session key matches any process currently known to be running.
fn is_any_running_alias(key: &str, running_set: &HashSet<String>) -> bool {
    let key_clean = key.trim().to_lowercase();
    let key_base = key_clean.strip_suffix(".exe").unwrap_or(&key_clean);
    let key_base = key_base.strip_suffix(".desktop").unwrap_or(key_base);

    if running_set.contains(key_clean.as_str()) || running_set.contains(key_base) {
        return true;
    }

    for running in running_set {
        let running_base = running.strip_suffix(".exe").unwrap_or(running);
        let running_base = running_base.strip_suffix(".desktop").unwrap_or(running_base);

        if key_base == running_base {
            return true;
        }

        let norm_key = key_base.replace('_', "-");
        let norm_run = running_base.replace('_', "-");
        if norm_key == norm_run
            || norm_key.starts_with(&format!("{norm_run}-"))
            || norm_run.starts_with(&format!("{norm_key}-"))
        {
            return true;
        }

        if matches_reverse_dns(key_base, running_base) {
            return true;
        }
    }

    false
}

/// Background loop monitoring foreground windows and managing active focus sessions.
pub async fn run_tracker_loop(
    app_handle: AppHandle,
    config_store: Arc<RwLock<AppConfig>>,
    status_store: Arc<RwLock<TrackerStatus>>,
) {
    let mut sessions: HashMap<String, ActiveSession> = HashMap::new();
    let mut last_tracked_app: Option<state::ActiveAppInfo> = None;
    let mut interval = tokio::time::interval(Duration::from_secs(1));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut tick_counter: u64 = 0;

    loop {
        interval.tick().await;
        tick_counter = tick_counter.wrapping_add(1);

        let config = {
            let guard = config_store.read().await;
            guard.clone()
        };

        // Prune terminated process sessions every 2 seconds
        if tick_counter.is_multiple_of(2) {
            if let Ok(running_windows) = platform::list_running_windows() {
                let running_set: HashSet<String> = running_windows
                    .into_iter()
                    .map(|w| {
                        let proc = w.process_name.trim().to_lowercase();
                        proc.strip_suffix(".exe").unwrap_or(&proc).to_string()
                    })
                    .collect();

                sessions.retain(|k, v| {
                    let is_still_blacklisted = is_app_blacklisted(&v.app_name, "", &config.blacklisted_apps);
                    if !is_still_blacklisted {
                        return false;
                    }

                    // Never prune the currently focused tracked session while it is actively in foreground
                    if let Some(ref active) = last_tracked_app {
                        let active_key = active.process_name.trim().to_lowercase();
                        if k.eq_ignore_ascii_case(&active_key) || matches_reverse_dns(k, &active_key) {
                            return true;
                        }
                    }

                    is_any_running_alias(k, &running_set)
                });

                if let Some(ref tracked) = last_tracked_app {
                    let is_still_blacklisted = is_app_blacklisted(&tracked.process_name, "", &config.blacklisted_apps);
                    if !is_still_blacklisted {
                        last_tracked_app = None;
                    }
                }
            }
        }

        let active_window = match platform::get_active_window() {
            Ok(info) => info,
            Err(err) => {
                eprintln!("Failed to retrieve active window info: {}", err);
                None
            }
        };

        if let Some(app_info) = active_window {
            // When foreground window is Mindsnap or a system utility, do not track Mindsnap itself.
            // Preserve and report the tracked application's status in paused state.
            if is_system_or_ignored_app(&app_info.process_name) {
                let mut status = status_store.write().await;
                status.is_tracking = true;

                if let Some(ref tracked) = last_tracked_app {
                    let key = tracked.process_name.trim().to_lowercase();
                    if let Some(session) = sessions.get(&key) {
                        status.current_app = Some(tracked.process_name.clone());
                        status.current_app_display_name = Some(session.display_name.clone());
                        status.current_window_title = Some(tracked.window_title.clone());
                        status.continuous_seconds = session.continuous_seconds;
                        status.is_blacklisted = true;
                        status.initial_threshold_seconds = (config.initial_alert_minutes as u64) * 60;
                        status.repeat_threshold_seconds = (config.repeat_alert_minutes as u64) * 60;
                        status.alert_count = session.alert_count;
                    } else {
                        status.current_app = None;
                        status.current_app_display_name = None;
                        status.current_window_title = None;
                        status.continuous_seconds = 0;
                        status.is_blacklisted = false;
                        status.initial_threshold_seconds = (config.initial_alert_minutes as u64) * 60;
                        status.repeat_threshold_seconds = (config.repeat_alert_minutes as u64) * 60;
                        status.alert_count = 0;
                    }
                } else {
                    status.current_app = None;
                    status.current_app_display_name = None;
                    status.current_window_title = None;
                    status.continuous_seconds = 0;
                    status.is_blacklisted = false;
                    status.initial_threshold_seconds = (config.initial_alert_minutes as u64) * 60;
                    status.repeat_threshold_seconds = (config.repeat_alert_minutes as u64) * 60;
                    status.alert_count = 0;
                }
            } else {
                let display_name = platform::get_friendly_app_name(&app_info.process_name);
                let blacklisted = is_app_blacklisted(
                    &app_info.process_name,
                    &app_info.window_title,
                    &config.blacklisted_apps,
                );

                let mut active_session_seconds = 0;
                let mut active_alert_count = 0;

                if blacklisted {
                    let app_key = app_info.process_name.trim().to_lowercase();
                    let session = sessions
                        .entry(app_key)
                        .or_insert_with(|| ActiveSession::new(app_info.process_name.clone(), display_name.clone()));

                    session.continuous_seconds += 1;
                    active_session_seconds = session.continuous_seconds;

                    last_tracked_app = Some(app_info.clone());

                    let initial_threshold = (config.initial_alert_minutes as u64) * 60;
                    let repeat_threshold = (config.repeat_alert_minutes as u64) * 60;
                    let lang = &config.language;

                    if session.continuous_seconds >= initial_threshold && session.alert_count == 0 {
                        if config.notifications_enabled {
                            let title = crate::i18n::first_alert_title(lang);
                            let body = crate::i18n::first_alert_body(
                                lang,
                                &session.display_name,
                                config.initial_alert_minutes,
                            );

                            send_notification(&app_handle, &title, &body, config.sound_enabled);
                        }
                        session.alert_count += 1;
                        session.last_alert_seconds = session.continuous_seconds;
                    } else if session.alert_count > 0 {
                        let seconds_since_last = session
                            .continuous_seconds
                            .saturating_sub(session.last_alert_seconds);

                        if seconds_since_last >= repeat_threshold {
                            if config.notifications_enabled {
                                let total_minutes = session.continuous_seconds / 60;
                                let title = crate::i18n::repeat_alert_title(lang);
                                let body = crate::i18n::repeat_alert_body(
                                    lang,
                                    &session.display_name,
                                    total_minutes,
                                );

                                send_notification(&app_handle, &title, &body, config.sound_enabled);
                            }
                            session.alert_count += 1;
                            session.last_alert_seconds = session.continuous_seconds;
                        }
                    }

                    active_alert_count = session.alert_count;
                }

                // Synchronize current status for frontend inspection
                let mut status = status_store.write().await;
                status.is_tracking = true;
                status.current_app = Some(app_info.process_name);
                status.current_app_display_name = Some(display_name);
                status.current_window_title = Some(app_info.window_title);
                status.continuous_seconds = active_session_seconds;
                status.is_blacklisted = blacklisted;
                status.initial_threshold_seconds = (config.initial_alert_minutes as u64) * 60;
                status.repeat_threshold_seconds = (config.repeat_alert_minutes as u64) * 60;
                status.alert_count = active_alert_count;
            }
        } else {
            // Preserve session metrics when no window is focused (e.g. lock screen)
            let mut status = status_store.write().await;
            if let Some(ref tracked) = last_tracked_app {
                let key = tracked.process_name.trim().to_lowercase();
                if let Some(session) = sessions.get(&key) {
                    status.current_app = Some(tracked.process_name.clone());
                    status.current_app_display_name = Some(session.display_name.clone());
                    status.current_window_title = Some(tracked.window_title.clone());
                    status.continuous_seconds = session.continuous_seconds;
                    status.is_blacklisted = true;
                    status.alert_count = session.alert_count;
                } else {
                    status.current_app = None;
                    status.current_app_display_name = None;
                    status.current_window_title = None;
                    status.continuous_seconds = 0;
                    status.is_blacklisted = false;
                }
            } else {
                status.current_app = None;
                status.current_app_display_name = None;
                status.current_window_title = None;
                status.continuous_seconds = 0;
                status.is_blacklisted = false;
            }
        }
    }
}

/// Delivers a native desktop notification via Tauri plugin and optional platform chime.
pub fn send_notification(app_handle: &AppHandle, title: &str, body: &str, sound_enabled: bool) {
    let mut builder = app_handle.notification().builder();
    builder = builder.title(title).body(body);
    if let Some(icon_path) = platform::get_notification_icon_path() {
        builder = builder.icon(icon_path);
    }
    if let Err(err) = builder.show() {
        eprintln!("Failed to display system notification: {:?}", err);
    }
    if sound_enabled {
        platform::play_notification_sound();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_app_blacklisted() {
        let blacklist = vec![
            "chrome.exe".to_string(),
            "discord".to_string(),
            "twitter".to_string(),
        ];

        // Exact match
        assert!(is_app_blacklisted("chrome.exe", "Google Chrome", &blacklist));
        assert!(is_app_blacklisted("CHROME.EXE", "New Tab", &blacklist));

        // Extension-stripped match
        assert!(is_app_blacklisted("discord.exe", "Discord", &blacklist));
        assert!(is_app_blacklisted("Discord.exe", "#general", &blacklist));

        // Window title substring match
        assert!(is_app_blacklisted("firefox.exe", "Twitter / X - Home", &blacklist));

        // Non-blacklisted application
        assert!(!is_app_blacklisted("code.exe", "Visual Studio Code", &blacklist));
        assert!(!is_app_blacklisted("notepad.exe", "Untitled - Notepad", &blacklist));

        // Cross-platform (.exe vs Linux binary match)
        assert!(is_app_blacklisted("discord", "Discord", &["discord.exe".to_string()]));
        assert!(is_app_blacklisted("discord.exe", "Discord", &["discord".to_string()]));
        assert!(is_app_blacklisted("spotify", "Spotify", &["spotify.desktop".to_string()]));

        // Reverse-DNS flatpak matching
        assert!(is_app_blacklisted("dev.geopjr.Tuba", "Tuba", &["tuba".to_string()]));
        assert!(is_app_blacklisted("tuba", "Tuba", &["dev.geopjr.Tuba".to_string()]));
        assert!(is_app_blacklisted("com.spotify.Client", "Spotify", &["spotify".to_string()]));
        assert!(is_app_blacklisted("org.gnome.Nautilus", "Files", &["nautilus".to_string()]));
        assert!(is_app_blacklisted("org.telegram.desktop", "Telegram", &["telegram".to_string()]));

        // System applications and Mindsnap itself are never blocked even if blacklisted
        let dangerous_blacklist = vec![
            "mindsnap.exe".to_string(),
            "explorer.exe".to_string(),
            "msedgewebview2.exe".to_string(),
        ];
        assert!(!is_app_blacklisted("mindsnap.exe", "Mindsnap", &dangerous_blacklist));
        assert!(!is_app_blacklisted("explorer.exe", "Windows Explorer", &dangerous_blacklist));
        assert!(!is_app_blacklisted("msedgewebview2.exe", "WebView2", &dangerous_blacklist));
    }

    #[test]
    fn test_matches_reverse_dns() {
        assert!(matches_reverse_dns("dev.geopjr.Tuba", "tuba"));
        assert!(matches_reverse_dns("tuba", "dev.geopjr.Tuba"));
        assert!(matches_reverse_dns("com.spotify.Client", "spotify"));
        assert!(matches_reverse_dns("spotify", "com.spotify.Client"));
        assert!(matches_reverse_dns("org.gnome.Nautilus", "nautilus"));
        assert!(matches_reverse_dns("nautilus", "org.gnome.Nautilus"));
        assert!(matches_reverse_dns("org.telegram.desktop", "telegram"));
        assert!(matches_reverse_dns("org.telegram.desktop", "telegram-desktop"));
        assert!(matches_reverse_dns("com.valvesoftware.Steam", "steam"));

        // Negative assertions
        assert!(!matches_reverse_dns("google-chrome", "dev.geopjr.Tuba"));
        assert!(!matches_reverse_dns("firefox", "com.spotify.Client"));
    }

    #[test]
    fn test_is_any_running_alias() {
        let running_set: HashSet<String> = [
            "tuba".to_string(),
            "spotify".to_string(),
            "google-chrome".to_string(),
        ]
        .into_iter()
        .collect();

        assert!(is_any_running_alias("dev.geopjr.Tuba", &running_set));
        assert!(is_any_running_alias("com.spotify.Client", &running_set));
        assert!(is_any_running_alias("google-chrome", &running_set));
        assert!(is_any_running_alias("google_chrome", &running_set));
        assert!(!is_any_running_alias("firefox", &running_set));
        assert!(!is_any_running_alias("discord", &running_set));
    }

    #[test]
    fn test_is_system_or_ignored_app() {
        assert!(is_system_or_ignored_app("mindsnap.exe"));
        assert!(is_system_or_ignored_app("MindSnap"));
        assert!(is_system_or_ignored_app("msedgewebview2.exe"));
        assert!(is_system_or_ignored_app("explorer.exe"));
        assert!(is_system_or_ignored_app("shellexperiencehost.exe"));
        assert!(is_system_or_ignored_app("TextInputHost.exe"));
        assert!(is_system_or_ignored_app("conhost.exe"));

        // User applications must not be ignored
        assert!(!is_system_or_ignored_app("chrome.exe"));
        assert!(!is_system_or_ignored_app("discord.exe"));
        assert!(!is_system_or_ignored_app("spotify.exe"));
        assert!(!is_system_or_ignored_app("code.exe"));
    }

    #[test]
    fn test_session_pause_and_resume() {
        let mut sessions: HashMap<String, ActiveSession> = HashMap::new();

        // 1. Focused on Discord for 10 seconds
        let discord_key = "discord.exe".to_string();
        let session = sessions
            .entry(discord_key.clone())
            .or_insert_with(|| ActiveSession::new("discord.exe".to_string(), "Discord".to_string()));
        session.continuous_seconds += 10;
        assert_eq!(session.continuous_seconds, 10);

        // 2. Switched away from Discord (duration frozen)
        assert_eq!(
            sessions.get(&discord_key).map(|s| s.continuous_seconds),
            Some(10)
        );

        // 3. Returned to Discord for an additional 5 seconds
        let session = sessions
            .entry(discord_key.clone())
            .or_insert_with(|| ActiveSession::new("discord.exe".to_string(), "Discord".to_string()));
        session.continuous_seconds += 5;
        assert_eq!(session.continuous_seconds, 15);

        // 4. Closed window (dropped from running set) cleans up session
        let running_set: HashSet<String> = ["code.exe".to_string()].into_iter().collect();
        sessions.retain(|k, _| running_set.contains(k));
        assert!(!sessions.contains_key(&discord_key));
    }

    #[test]
    fn test_mindsnap_self_focus_preserves_tracked_app() {
        let mut sessions: HashMap<String, ActiveSession> = HashMap::new();
        let mut last_tracked_app: Option<state::ActiveAppInfo> = None;
        let mut status = state::TrackerStatus::default();

        let config_blacklist = vec!["discord.exe".to_string()];

        // 1. User focuses Discord (tracked/blacklisted app)
        let discord_app = state::ActiveAppInfo {
            process_name: "discord.exe".to_string(),
            window_title: "#general - Discord".to_string(),
            process_id: 1234,
        };

        if is_app_blacklisted(
            &discord_app.process_name,
            &discord_app.window_title,
            &config_blacklist,
        ) {
            let key = discord_app.process_name.trim().to_lowercase();
            let session = sessions
                .entry(key)
                .or_insert_with(|| state::ActiveSession::new(discord_app.process_name.clone(), "Discord".to_string()));
            session.continuous_seconds += 42;
            last_tracked_app = Some(discord_app.clone());

            status.current_app = Some(discord_app.process_name.clone());
            status.current_app_display_name = Some(session.display_name.clone());
            status.current_window_title = Some(discord_app.window_title.clone());
            status.continuous_seconds = session.continuous_seconds;
            status.is_blacklisted = true;
        }

        assert_eq!(status.current_app.as_deref(), Some("discord.exe"));
        assert_eq!(status.current_app_display_name.as_deref(), Some("Discord"));
        assert_eq!(status.continuous_seconds, 42);
        assert!(status.is_blacklisted);

        // 2. User switches focus to Mindsnap window
        let mindsnap_app = state::ActiveAppInfo {
            process_name: "mindsnap.exe".to_string(),
            window_title: "Mindsnap".to_string(),
            process_id: 5678,
        };

        assert!(is_system_or_ignored_app(&mindsnap_app.process_name));

        // When system or Mindsnap itself is focused, the active session is paused
        if is_system_or_ignored_app(&mindsnap_app.process_name) {
            if let Some(ref tracked) = last_tracked_app {
                let key = tracked.process_name.trim().to_lowercase();
                if let Some(session) = sessions.get(&key) {
                    status.current_app = Some(tracked.process_name.clone());
                    status.current_app_display_name = Some(session.display_name.clone());
                    status.current_window_title = Some(tracked.window_title.clone());
                    status.continuous_seconds = session.continuous_seconds;
                    status.is_blacklisted = true;
                }
            }
        }

        // Verify Mindsnap never reports itself and retains the tracked target
        assert_ne!(status.current_app.as_deref(), Some("mindsnap.exe"));
        assert_eq!(status.current_app.as_deref(), Some("discord.exe"));
        assert_eq!(status.current_app_display_name.as_deref(), Some("Discord"));
        assert_eq!(status.continuous_seconds, 42);
        assert!(status.is_blacklisted);

        // 3. Tracked application window is closed (prune running set)
        let running_set: HashSet<String> = HashSet::new();
        let is_running = running_set.contains("discord.exe");
        if !is_running {
            last_tracked_app = None;
            sessions.remove("discord.exe");
        }

        // When Mindsnap is focused after target app was closed
        if is_system_or_ignored_app(&mindsnap_app.process_name) {
            if let Some(ref tracked) = last_tracked_app {
                let key = tracked.process_name.trim().to_lowercase();
                if let Some(session) = sessions.get(&key) {
                    status.current_app = Some(tracked.process_name.clone());
                    status.current_app_display_name = Some(session.display_name.clone());
                    status.continuous_seconds = session.continuous_seconds;
                    status.is_blacklisted = true;
                }
            } else {
                status.current_app = None;
                status.current_app_display_name = None;
                status.current_window_title = None;
                status.continuous_seconds = 0;
                status.is_blacklisted = false;
            }
        }

        assert_eq!(status.current_app, None);
        assert_eq!(status.current_app_display_name, None);
        assert_eq!(status.continuous_seconds, 0);
        assert!(!status.is_blacklisted);
    }
}
