use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use crate::error::Result;
use crate::tracker::state::{ActiveAppInfo, AppItemInfo};

/// Plays the standard system notification sound on Linux environments.
pub fn play_notification_sound() {
    let _ = Command::new("canberra-gtk-play")
        .args(["-i", "message-new-instant"])
        .spawn()
        .or_else(|_| {
            Command::new("paplay")
                .arg("/usr/share/sounds/freedesktop/stereo/message.oga")
                .spawn()
        });
}

/// Filters system services, compositors, and panels specific to Linux and Wayland environments.
pub fn is_platform_ignored_app(name: &str) -> bool {
    let lower = name.trim().to_lowercase();
    let name_only = lower.strip_suffix(".desktop").unwrap_or(&lower);
    let name_only = name_only.strip_suffix(".exe").unwrap_or(name_only);

    // 1. Mindsnap binary and current executable
    if name_only == "mindsnap" {
        return true;
    }
    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(curr_name) = current_exe.file_name().and_then(|n| n.to_str()) {
            let curr_lower = curr_name.to_lowercase();
            let curr_base = curr_lower.strip_suffix(".exe").unwrap_or(&curr_lower);
            if name_only == curr_base || lower == curr_lower {
                return true;
            }
        }
    }

    // 2. Linux WebKitGTK processes
    match name_only {
        "webkitwebprocess" | "webkitnetworkprocess" | "webkitgpuprocess" => return true,
        _ => {}
    }

    // 3. Desktop environments, compositors, panels, and system daemons
    match name_only {
        // Compositors & window managers
        "mutter"
        | "gnome-shell"
        | "kwin"
        | "kwin_wayland"
        | "kwin_x11"
        | "hyprland"
        | "sway"
        | "wayfire"
        | "river"
        | "labwc"
        | "xfwm4"
        | "i3"
        | "bspwm"
        | "openbox"
        // Panels, docks, menus, and launchers
        | "waybar"
        | "plasmashell"
        | "krunner"
        | "gnome-panel"
        | "xfce4-panel"
        | "plank"
        | "latte-dock"
        | "tofi"
        | "rofi"
        | "wofi"
        | "fuzzel"
        | "dmenu"
        | "eww"
        | "ags"
        | "swaybar"
        | "swaync"
        | "mako"
        | "dunst"
        // XDG portals and desktop services
        | "xdg-desktop-portal"
        | "xdg-desktop-portal-gnome"
        | "xdg-desktop-portal-kde"
        | "xdg-desktop-portal-wlr"
        | "xdg-desktop-portal-hyprland"
        | "xdg-desktop-portal-gtk"
        | "xdg-document-portal"
        | "xdg-permission-store"
        // Audio and session infrastructure
        | "pipewire"
        | "pipewire-pulse"
        | "wireplumber"
        | "pulseaudio"
        | "dbus-daemon"
        | "systemd"
        | "ibus-daemon"
        | "fcitx"
        | "fcitx5"
        | "xorg"
        | "xwayland"
        | "gdm"
        | "sddm"
        | "lightdm"
        | "polkit-gnome-authentication-agent-1"
        | "polkit-kde-authentication-agent-1" => true,
        _ => false,
    }
}

/// Resolves popular Linux application names to user-friendly display titles.
pub fn get_friendly_app_name(app_name: &str) -> String {
    let lower = app_name.trim().to_lowercase();
    let clean = lower.strip_suffix(".desktop").unwrap_or(&lower);
    let clean = clean.strip_suffix(".exe").unwrap_or(clean);

    match clean {
        "google-chrome" | "google-chrome-stable" => "Google Chrome".to_string(),
        "chromium" | "chromium-browser" => "Chromium".to_string(),
        "firefox" | "firefox-esr" | "firefox-bin" => "Mozilla Firefox".to_string(),
        "brave-browser" | "brave" => "Brave Browser".to_string(),
        "microsoft-edge" | "microsoft-edge-stable" | "msedge" => "Microsoft Edge".to_string(),
        "discord" | "discord-canary" | "discord-ptb" => "Discord".to_string(),
        "spotify" => "Spotify".to_string(),
        "steam" => "Steam".to_string(),
        "code" | "vscode" | "visual-studio-code" => "Visual Studio Code".to_string(),
        "telegram-desktop" | "telegram" => "Telegram".to_string(),
        "vlc" => "VLC Media Player".to_string(),
        "obs" | "obs-studio" => "OBS Studio".to_string(),
        "slack" => "Slack".to_string(),
        "thunderbird" => "Mozilla Thunderbird".to_string(),
        "gimp" => "GIMP".to_string(),
        "inkscape" => "Inkscape".to_string(),
        "blender" => "Blender".to_string(),
        "lutris" => "Lutris".to_string(),
        "heroic" => "Heroic Games Launcher".to_string(),
        _ => {
            let words: Vec<String> = clean
                .split(['_', '-', ' '])
                .filter(|w| !w.is_empty())
                .map(|w| {
                    let mut chars = w.chars();
                    match chars.next() {
                        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                        None => String::new(),
                    }
                })
                .collect();

            if words.is_empty() {
                clean.to_string()
            } else {
                words.join(" ")
            }
        }
    }
}

/// Detects active window using a prioritized platform cascade (Hyprland -> Sway -> D-Bus -> X11).
pub fn get_active_window() -> Result<Option<ActiveAppInfo>> {
    // 1. Hyprland
    if std::env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok() {
        if let Some(info) = get_active_window_hyprland() {
            return Ok(Some(info));
        }
    }

    // 2. Sway / wlroots
    if std::env::var("SWAYSOCK").is_ok() {
        if let Some(info) = get_active_window_sway() {
            return Ok(Some(info));
        }
    }

    // 3. GNOME Shell / KDE Plasma D-Bus
    if let Some(info) = get_active_window_dbus() {
        return Ok(Some(info));
    }

    // 4. X11 / EWMH standard query
    if std::env::var("DISPLAY").is_ok() {
        if let Some(info) = get_active_window_x11() {
            return Ok(Some(info));
        }
    }

    Ok(None)
}

/// Hyprland `hyprctl activewindow -j` çağrısı ile aktif pencereyi bulur.
fn get_active_window_hyprland() -> Option<ActiveAppInfo> {
    let output = Command::new("hyprctl")
        .args(["activewindow", "-j"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let json_str = String::from_utf8_lossy(&output.stdout);
    let val: serde_json::Value = serde_json::from_str(&json_str).ok()?;

    let class = val.get("class").and_then(|c| c.as_str())?.trim().to_string();
    if class.is_empty() {
        return None;
    }

    let title = val.get("title").and_then(|t| t.as_str()).unwrap_or("").to_string();
    let pid = val.get("pid").and_then(|p| p.as_u64()).unwrap_or(0) as u32;

    Some(ActiveAppInfo {
        process_name: class,
        window_title: title,
        process_id: pid,
    })
}

/// Queries focused window node via Sway IPC (`swaymsg -t get_tree`).
fn get_active_window_sway() -> Option<ActiveAppInfo> {
    let output = Command::new("swaymsg")
        .args(["-t", "get_tree"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let json_str = String::from_utf8_lossy(&output.stdout);
    let val: serde_json::Value = serde_json::from_str(&json_str).ok()?;

    find_focused_sway_node(&val)
}

fn find_focused_sway_node(node: &serde_json::Value) -> Option<ActiveAppInfo> {
    if node.get("focused").and_then(|f| f.as_bool()).unwrap_or(false) {
        let app_id = node
            .get("app_id")
            .and_then(|a| a.as_str())
            .or_else(|| {
                node.get("window_properties")
                    .and_then(|wp| wp.get("class"))
                    .and_then(|c| c.as_str())
            })?
            .trim()
            .to_string();

        if !app_id.is_empty() {
            let name = node.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string();
            let pid = node.get("pid").and_then(|p| p.as_u64()).unwrap_or(0) as u32;
            return Some(ActiveAppInfo {
                process_name: app_id,
                window_title: name,
                process_id: pid,
            });
        }
    }

    if let Some(nodes) = node.get("nodes").and_then(|n| n.as_array()) {
        for child in nodes {
            if let Some(found) = find_focused_sway_node(child) {
                return Some(found);
            }
        }
    }

    if let Some(floating) = node.get("floating_nodes").and_then(|f| f.as_array()) {
        for child in floating {
            if let Some(found) = find_focused_sway_node(child) {
                return Some(found);
            }
        }
    }

    None
}

/// Queries active window via KDE kdotool or D-Bus services.
fn get_active_window_dbus() -> Option<ActiveAppInfo> {
    if let Ok(output) = Command::new("kdotool").args(["getactivewindow"]).output() {
        if output.status.success() {
            let win_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !win_id.is_empty() {
                let name_out = Command::new("kdotool")
                    .args(["getwindowname", &win_id])
                    .output()
                    .ok();
                let title = name_out
                    .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                    .unwrap_or_default();

                return Some(ActiveAppInfo {
                    process_name: "kwin_active".to_string(),
                    window_title: title,
                    process_id: 0,
                });
            }
        }
    }

    None
}

/// Queries active window via standard X11 EWMH utilities (xdotool / xprop).
fn get_active_window_x11() -> Option<ActiveAppInfo> {
    let output = Command::new("xdotool")
        .args(["getactivewindow"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let win_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if win_id.is_empty() {
        return None;
    }

    let title_out = Command::new("xdotool")
        .args(["getwindowname", &win_id])
        .output()
        .ok();
    let title = title_out
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    let pid_out = Command::new("xdotool")
        .args(["getwindowpid", &win_id])
        .output()
        .ok();
    let pid = pid_out
        .and_then(|o| String::from_utf8_lossy(&o.stdout).trim().parse::<u32>().ok())
        .unwrap_or(0);

    let process_name = if pid > 0 {
        get_process_comm_by_pid(pid).unwrap_or_else(|| "x11_app".to_string())
    } else {
        "x11_app".to_string()
    };

    Some(ActiveAppInfo {
        process_name,
        window_title: title,
        process_id: pid,
    })
}

/// Reads process name from /proc/[pid]/comm.
fn get_process_comm_by_pid(pid: u32) -> Option<String> {
    let path = format!("/proc/{}/comm", pid);
    fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

/// Lists running applications matched against installed desktop entries.
pub fn list_running_app_items(blacklist: &[String]) -> Result<Vec<AppItemInfo>> {
    let desktop_map = load_desktop_entries();
    let mut raw_apps: Vec<String> = Vec::new();

    // Scan /proc for running user processes
    if let Ok(entries) = fs::read_dir("/proc") {
        for entry in entries.flatten() {
            if let Ok(file_name) = entry.file_name().into_string() {
                if file_name.chars().all(|c| c.is_ascii_digit()) {
                    let comm_path = entry.path().join("comm");
                    if let Ok(comm) = fs::read_to_string(comm_path) {
                        let proc_name = comm.trim().to_string();
                        if !proc_name.is_empty() && !is_platform_ignored_app(&proc_name) {
                            raw_apps.push(proc_name);
                        }
                    }
                }
            }
        }
    }

    raw_apps.sort_by_key(|a| a.to_lowercase());
    raw_apps.dedup_by(|a, b| a.eq_ignore_ascii_case(b));

    let mut result = Vec::new();
    for proc_name in raw_apps {
        let (display_name, icon_base64) = if let Some(meta) = desktop_map.get(&proc_name.to_lowercase()) {
            (meta.name.clone(), meta.icon_base64.clone())
        } else {
            (get_friendly_app_name(&proc_name), None)
        };

        let is_tracked = crate::tracker::is_app_blacklisted(&proc_name, "", blacklist);

        result.push(AppItemInfo {
            exe_name: proc_name,
            display_name,
            window_title: String::new(),
            icon_base64,
            is_tracked,
        });
    }

    Ok(result)
}

pub fn list_running_windows() -> Result<Vec<ActiveAppInfo>> {
    let items = list_running_app_items(&[])?;
    Ok(items
        .into_iter()
        .map(|item| ActiveAppInfo {
            process_name: item.exe_name,
            window_title: item.window_title,
            process_id: 0,
        })
        .collect())
}

/// Generates rich display items for blacklisted Linux applications.
pub fn get_tracked_app_items(blacklist: &[String]) -> Result<Vec<AppItemInfo>> {
    let desktop_map = load_desktop_entries();
    let mut result = Vec::new();

    for app in blacklist {
        let clean = app.trim().to_string();
        if clean.is_empty() || is_platform_ignored_app(&clean) {
            continue;
        }

        let (display_name, icon_base64) = if let Some(meta) = desktop_map.get(&clean.to_lowercase()) {
            (meta.name.clone(), meta.icon_base64.clone())
        } else {
            (get_friendly_app_name(&clean), None)
        };

        result.push(AppItemInfo {
            exe_name: clean,
            display_name,
            window_title: String::new(),
            icon_base64,
            is_tracked: true,
        });
    }

    Ok(result)
}

/// Opens native file dialog to select a .desktop entry or executable.
pub fn pick_app_file(blacklist: &[String], locale: &str) -> Result<Option<AppItemInfo>> {
    let dialog_title = crate::i18n::file_picker_title(locale);
    let filter_name = crate::i18n::dialog_desktop_filter(locale);
    let file = rfd::FileDialog::new()
        .set_title(&dialog_title)
        .add_filter(&filter_name, &["desktop"])
        .pick_file();

    if let Some(path_buf) = file {
        if let Some(file_name) = path_buf.file_name().and_then(|n| n.to_str()) {
            let mut app_name = file_name.to_string();
            let mut display_name = get_friendly_app_name(&app_name);

            if app_name.ends_with(".desktop") {
                if let Some(entry) = parse_desktop_file(&path_buf) {
                    if let Some(exec) = entry.exec {
                        app_name = exec;
                    }
                    display_name = entry.name;
                }
            }

            if is_platform_ignored_app(&app_name) {
                return Err(crate::error::MindsnapError::Config(
                    crate::i18n::err_cannot_track_system(locale),
                ));
            }

            let is_tracked = crate::tracker::is_app_blacklisted(&app_name, "", blacklist);

            return Ok(Some(AppItemInfo {
                exe_name: app_name,
                display_name,
                window_title: String::new(),
                icon_base64: None,
                is_tracked,
            }));
        }
    }

    Ok(None)
}

pub use pick_app_file as pick_exe_file;

#[derive(Default, Clone)]
struct DesktopMeta {
    name: String,
    exec: Option<String>,
    icon_base64: Option<String>,
}

/// Indexes .desktop application entries from standard XDG data directories.
fn load_desktop_entries() -> HashMap<String, DesktopMeta> {
    let mut map = HashMap::new();
    let mut search_dirs = vec![
        PathBuf::from("/usr/share/applications"),
        PathBuf::from("/usr/local/share/applications"),
        PathBuf::from("/var/lib/flatpak/exports/share/applications"),
    ];

    if let Some(home) = dirs::home_dir() {
        search_dirs.push(home.join(".local/share/applications"));
    }

    for dir in search_dirs {
        if !dir.exists() {
            continue;
        }

        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("desktop") {
                    if let Some(meta) = parse_desktop_file(&path) {
                        if let Some(ref exec) = meta.exec {
                            map.insert(exec.to_lowercase(), meta.clone());
                        }
                        if let Some(file_stem) = path.file_stem().and_then(|s| s.to_str()) {
                            map.insert(file_stem.to_lowercase(), meta);
                        }
                    }
                }
            }
        }
    }

    map
}

/// Parses an individual .desktop file safely.
fn parse_desktop_file(path: &Path) -> Option<DesktopMeta> {
    let content = fs::read_to_string(path).ok()?;
    let mut in_desktop_entry = false;
    let mut name = None;
    let mut exec = None;
    let mut icon_name = None;
    let mut no_display = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "[Desktop Entry]" {
            in_desktop_entry = true;
            continue;
        } else if trimmed.starts_with('[') && in_desktop_entry {
            break;
        }

        if !in_desktop_entry {
            continue;
        }

        if let Some((key, val)) = trimmed.split_once('=') {
            match key.trim() {
                "Name" if name.is_none() => name = Some(val.trim().to_string()),
                "Exec" if exec.is_none() => {
                    let first_word = val.split_whitespace().next().unwrap_or("");
                    let clean_exec = Path::new(first_word)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or(first_word);
                    if !clean_exec.is_empty() {
                        exec = Some(clean_exec.to_string());
                    }
                }
                "Icon" if icon_name.is_none() => icon_name = Some(val.trim().to_string()),
                "NoDisplay" => no_display = val.trim().eq_ignore_ascii_case("true"),
                _ => {}
            }
        }
    }

    if no_display {
        return None;
    }

    let final_name = name.unwrap_or_else(|| {
        path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Application")
            .to_string()
    });

    Some(DesktopMeta {
        name: final_name,
        exec,
        icon_base64: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linux_ignored_apps() {
        assert!(is_platform_ignored_app("mindsnap"));
        assert!(is_platform_ignored_app("WebKitWebProcess"));
        assert!(is_platform_ignored_app("gnome-shell"));
        assert!(is_platform_ignored_app("kwin_wayland"));
        assert!(is_platform_ignored_app("hyprland"));
        assert!(is_platform_ignored_app("sway"));
        assert!(is_platform_ignored_app("waybar"));
        assert!(is_platform_ignored_app("pipewire"));
        assert!(is_platform_ignored_app("dbus-daemon"));

        // User applications must not be ignored
        assert!(!is_platform_ignored_app("google-chrome"));
        assert!(!is_platform_ignored_app("discord"));
        assert!(!is_platform_ignored_app("spotify"));
        assert!(!is_platform_ignored_app("code"));
    }

    #[test]
    fn test_linux_friendly_names() {
        assert_eq!(get_friendly_app_name("google-chrome-stable"), "Google Chrome");
        assert_eq!(get_friendly_app_name("discord"), "Discord");
        assert_eq!(get_friendly_app_name("telegram-desktop"), "Telegram");
        assert_eq!(get_friendly_app_name("custom-media-player"), "Custom Media Player");
    }

    #[test]
    fn test_linux_get_active_window_safe() {
        let active = get_active_window();
        assert!(active.is_ok());
    }

    #[test]
    fn test_linux_play_notification_sound_safe() {
        play_notification_sound();
    }
}

