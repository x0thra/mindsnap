use std::sync::Arc;
use tokio::sync::RwLock;
use tauri::{State, Window};

use crate::config::AppConfig;
use crate::tracker::platform::{get_tracked_app_items, list_running_app_items, pick_app_file};
use crate::tracker::state::{AppItemInfo, TrackerStatus};

pub struct AppState {
    pub config: Arc<RwLock<AppConfig>>,
    pub tracker_status: Arc<RwLock<TrackerStatus>>,
}

#[tauri::command]
pub async fn get_config(state: State<'_, AppState>) -> Result<AppConfig, String> {
    let config = state.config.read().await;
    Ok(config.clone())
}

#[tauri::command]
pub async fn save_config(
    new_config: AppConfig,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let lang = &new_config.language;
    if new_config.initial_alert_minutes == 0 {
        return Err(crate::i18n::err_min_initial_alert(lang));
    }
    if new_config.repeat_alert_minutes == 0 {
        return Err(crate::i18n::err_min_repeat_alert(lang));
    }

    let mut sanitized = new_config;
    sanitized.blacklisted_apps.retain(|app| !crate::tracker::is_system_or_ignored_app(app));
    sanitized.save().map_err(|e| e.to_string())?;

    let mut config_guard = state.config.write().await;
    *config_guard = sanitized;

    Ok(())
}

#[tauri::command]
pub async fn get_tracker_status(state: State<'_, AppState>) -> Result<TrackerStatus, String> {
    let status = state.tracker_status.read().await;
    Ok(status.clone())
}

/// Lists running application windows with resolved display names and icons.
#[tauri::command]
pub async fn get_running_apps(state: State<'_, AppState>) -> Result<Vec<AppItemInfo>, String> {
    let config = state.config.read().await;
    list_running_app_items(&config.blacklisted_apps).map_err(|e| e.to_string())
}

/// Returns tracked blacklist applications with resolved display names and icons.
#[tauri::command]
pub async fn get_tracked_apps(state: State<'_, AppState>) -> Result<Vec<AppItemInfo>, String> {
    let config = state.config.read().await;
    get_tracked_app_items(&config.blacklisted_apps).map_err(|e| e.to_string())
}

/// Opens native file picker to select an executable and adds it to the blacklist.
#[tauri::command]
pub async fn pick_and_add_exe(state: State<'_, AppState>) -> Result<Option<AppItemInfo>, String> {
    let mut config_guard = state.config.write().await;
    let lang = config_guard.language.clone();
    let picked = pick_app_file(&config_guard.blacklisted_apps, &lang).map_err(|e| e.to_string())?;

    if let Some(ref item) = picked {
        if crate::tracker::is_system_or_ignored_app(&item.exe_name) {
            return Err(crate::i18n::err_cannot_track_system(&lang));
        }

        let exists = config_guard
            .blacklisted_apps
            .iter()
            .any(|a| a.eq_ignore_ascii_case(&item.exe_name));

        if !exists {
            config_guard.blacklisted_apps.push(item.exe_name.clone());
            config_guard.save().map_err(|e| e.to_string())?;
        }
    }

    Ok(picked)
}

/// Adds an application executable directly to the blacklist.
#[tauri::command]
pub async fn add_app_to_blacklist(
    exe_name: String,
    state: State<'_, AppState>,
) -> Result<Vec<AppItemInfo>, String> {
    let mut config_guard = state.config.write().await;
    let lang = config_guard.language.clone();

    let clean = exe_name.trim().to_string();
    if clean.is_empty() {
        return Err(crate::i18n::err_invalid_app_name(&lang));
    }

    if crate::tracker::is_system_or_ignored_app(&clean) {
        return Err(crate::i18n::err_cannot_track_system(&lang));
    }

    let exists = config_guard
        .blacklisted_apps
        .iter()
        .any(|a| a.eq_ignore_ascii_case(&clean));

    if !exists {
        config_guard.blacklisted_apps.push(clean);
        config_guard.save().map_err(|e| e.to_string())?;
    }

    get_tracked_app_items(&config_guard.blacklisted_apps).map_err(|e| e.to_string())
}

/// Removes an application executable from the blacklist.
#[tauri::command]
pub async fn remove_app_from_blacklist(
    exe_name: String,
    state: State<'_, AppState>,
) -> Result<Vec<AppItemInfo>, String> {
    let mut config_guard = state.config.write().await;
    config_guard
        .blacklisted_apps
        .retain(|a| !a.eq_ignore_ascii_case(&exe_name));

    config_guard.save().map_err(|e| e.to_string())?;

    get_tracked_app_items(&config_guard.blacklisted_apps).map_err(|e| e.to_string())
}

/// Synchronizes UI language preference with stored configuration.
#[tauri::command]
pub async fn sync_locale(
    language: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut config_guard = state.config.write().await;
    if config_guard.language != language {
        config_guard.language = language;
        config_guard.save().map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Resolves a requested language preference ("auto", "tr_tr", etc.) to a canonical supported locale code.
#[tauri::command]
pub fn resolve_locale(preference: Option<String>) -> String {
    let pref = preference.unwrap_or_else(|| "auto".to_string());
    crate::i18n::resolve_locale_code(&pref)
}


/// Minimizes window to the taskbar.
#[tauri::command]
pub async fn app_minimize(window: Window) -> Result<(), String> {
    window.minimize().map_err(|e| e.to_string())
}

/// Toggles between maximized and restored window states.
#[tauri::command]
pub async fn app_toggle_maximize(window: Window) -> Result<bool, String> {
    let is_max = window.is_maximized().unwrap_or(false);
    if is_max {
        window.unmaximize().map_err(|e| e.to_string())?;
        #[cfg(target_os = "linux")]
        {
            let _ = window.set_size(tauri::LogicalSize::new(920.0, 660.0));
        }
        Ok(false)
    } else {
        window.maximize().map_err(|e| e.to_string())?;
        Ok(true)
    }
}

/// Initiates native window dragging from custom titlebar regions.
#[tauri::command]
pub async fn app_start_dragging(window: Window) -> Result<(), String> {
    window.start_dragging().map_err(|e| e.to_string())
}

/// Closes window (minimizing to tray if configured).
#[tauri::command]
pub async fn app_close(window: Window) -> Result<(), String> {
    window.close().map_err(|e| e.to_string())
}

/// Returns the package version defined in Cargo.toml.
#[tauri::command]
pub fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Safely launches an external HTTP/HTTPS URL in the default browser.
#[tauri::command]
pub fn open_external_url(url: String) -> Result<(), String> {
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("Invalid URL protocol".to_string());
    }

    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::UI::Shell::ShellExecuteW;
        let url_wide: Vec<u16> = format!("{url}\0").encode_utf16().collect();
        let open_wide: Vec<u16> = "open\0".encode_utf16().collect();
        unsafe {
            let res = ShellExecuteW(
                std::ptr::null_mut(),
                open_wide.as_ptr(),
                url_wide.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                1, // SW_SHOWNORMAL
            );
            if (res as usize) <= 32 {
                return Err("Failed to open browser".to_string());
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&url)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

