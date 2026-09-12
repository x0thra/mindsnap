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

/// Initializes platform-specific notification subsystem by provisioning the app icon into standard XDG directories.
pub fn init_platform_notifications() {
    if let Some(home) = dirs::home_dir() {
        let icon_bytes: &[u8] = include_bytes!("../../../../src/icon.png");
        let icon_dirs = [
            home.join(".local/share/icons/hicolor/256x256/apps"),
            home.join(".local/share/icons/hicolor/128x128/apps"),
            home.join(".local/share/pixmaps"),
        ];

        for dir in &icon_dirs {
            if fs::create_dir_all(dir).is_ok() {
                let icon_file = dir.join("mindsnap.png");
                if !icon_file.exists() {
                    let _ = fs::write(&icon_file, icon_bytes);
                }
            }
        }
    }
}

/// Returns the primary path to the installed Mindsnap desktop notification icon on Linux.
pub fn get_notification_icon_path() -> Option<String> {
    let home = dirs::home_dir()?;
    let path = home.join(".local/share/icons/hicolor/256x256/apps/mindsnap.png");
    if path.exists() {
        Some(path.to_string_lossy().to_string())
    } else {
        None
    }
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

/// Detects active window using a prioritized platform cascade (Hyprland -> Sway -> KDE KWin -> X11/Xwayland).
pub fn get_active_window() -> Result<Option<ActiveAppInfo>> {
    // 1. Hyprland Wayland
    if std::env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok() {
        if let Some(info) = get_active_window_hyprland() {
            return Ok(Some(info));
        }
    }

    // 2. Sway / wlroots Wayland
    if std::env::var("SWAYSOCK").is_ok() {
        if let Some(info) = get_active_window_sway() {
            return Ok(Some(info));
        }
    }

    // 3. KDE Plasma (KWin D-Bus supportInformation query)
    if let Some(info) = get_active_window_kwin() {
        return Ok(Some(info));
    }

    // 4. Standard X11 / Xwayland query via xprop or xdotool
    if std::env::var("DISPLAY").is_ok() {
        if let Some(info) = get_active_window_x11() {
            return Ok(Some(info));
        }
    }

    Ok(None)
}

/// Hyprland `hyprctl activewindow -j` query.
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

/// Queries active window on KDE Plasma via KWin D-Bus supportInformation.
fn get_active_window_kwin() -> Option<ActiveAppInfo> {
    let output = Command::new("gdbus")
        .args([
            "call",
            "--session",
            "--dest",
            "org.kde.KWin",
            "--object-path",
            "/KWin",
            "--method",
            "org.kde.KWin.supportInformation",
        ])
        .output()
        .or_else(|_| {
            Command::new("qdbus")
                .args(["org.kde.KWin", "/KWin", "supportInformation"])
                .output()
        })
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8_lossy(&output.stdout);
    parse_kwin_active_client(&text)
}

/// Parses active client metadata from KWin supportInformation output.
pub fn parse_kwin_active_client(text: &str) -> Option<ActiveAppInfo> {
    let mut in_active_section = false;
    let mut caption = String::new();
    let mut resource_class = String::new();
    let mut resource_name = String::new();
    let mut pid: u32 = 0;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.contains("Active Client") || trimmed.contains("Active Window") {
            in_active_section = true;
            continue;
        }

        if in_active_section {
            if trimmed.starts_with("Client")
                || trimmed.starts_with("All Clients")
                || trimmed.starts_with("===")
                || trimmed.starts_with("Screens:")
                || trimmed.starts_with("Outputs:")
            {
                break;
            }

            if let Some((k, v)) = trimmed.split_once(':') {
                let key = k.trim().to_lowercase();
                let val = v.trim().trim_matches('"').trim_matches('\'').trim_matches('\\');
                match key.as_str() {
                    "caption" if caption.is_empty() => caption = val.to_string(),
                    "resourceclass" if resource_class.is_empty() => {
                        resource_class = val.to_string()
                    }
                    "resourcename" if resource_name.is_empty() => {
                        resource_name = val.to_string()
                    }
                    "pid" if pid == 0 => pid = val.parse::<u32>().unwrap_or(0),
                    _ => {}
                }
            }
        }
    }

    let process_name = if !resource_class.is_empty() {
        resource_class
    } else if !resource_name.is_empty() {
        resource_name
    } else if pid > 0 {
        get_process_comm_by_pid(pid).unwrap_or_default()
    } else {
        return None;
    };

    if process_name.is_empty() || is_platform_ignored_app(&process_name) {
        return None;
    }

    Some(ActiveAppInfo {
        process_name,
        window_title: caption,
        process_id: pid,
    })
}

/// Queries active window via standard X11 / Xwayland utilities (xprop or xdotool).
fn get_active_window_x11() -> Option<ActiveAppInfo> {
    // 1. Try xprop -root _NET_ACTIVE_WINDOW (standard across all X11/Xwayland sessions)
    if let Ok(output) = Command::new("xprop").args(["-root", "_NET_ACTIVE_WINDOW"]).output() {
        if output.status.success() {
            let out_str = String::from_utf8_lossy(&output.stdout);
            if let Some(win_id) = out_str.split('#').nth(1).map(|s| s.trim()) {
                if !win_id.is_empty() && win_id != "0x0" {
                    if let Some(info) = get_x11_window_info_by_id(win_id) {
                        return Some(info);
                    }
                }
            }
        }
    }

    // 2. Fallback to xdotool if available
    if let Ok(output) = Command::new("xdotool").args(["getactivewindow"]).output() {
        if output.status.success() {
            let win_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !win_id.is_empty() && win_id != "0" {
                if let Some(info) = get_x11_window_info_by_id(&win_id) {
                    return Some(info);
                }
            }
        }
    }

    None
}

fn get_x11_window_info_by_id(win_id: &str) -> Option<ActiveAppInfo> {
    let output = Command::new("xprop")
        .args(["-id", win_id, "WM_CLASS", "_NET_WM_NAME", "_NET_WM_PID"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let mut wm_class = String::new();
    let mut title = String::new();
    let mut pid: u32 = 0;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("WM_CLASS") {
            if let Some((_, val)) = trimmed.split_once('=') {
                if let Some(first_class) = val.split(',').next() {
                    wm_class = first_class.trim().trim_matches('"').to_string();
                }
            }
        } else if (trimmed.starts_with("_NET_WM_NAME") || trimmed.starts_with("WM_NAME")) && title.is_empty() {
            if let Some((_, val)) = trimmed.split_once('=') {
                title = val.trim().trim_matches('"').to_string();
            }
        } else if trimmed.starts_with("_NET_WM_PID") {
            if let Some((_, val)) = trimmed.split_once('=') {
                pid = val.trim().parse::<u32>().unwrap_or(0);
            }
        }
    }

    let process_name = if !wm_class.is_empty() {
        wm_class
    } else if pid > 0 {
        get_process_comm_by_pid(pid).unwrap_or_else(|| "x11_app".to_string())
    } else {
        return None;
    };

    if process_name.is_empty() || is_platform_ignored_app(&process_name) {
        return None;
    }

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

/// Reads non-truncated process name from /proc/[pid]/cmdline or falls back to comm.
fn get_process_name_from_proc(pid_dir: &Path) -> Option<String> {
    if let Ok(cmdline_bytes) = fs::read(pid_dir.join("cmdline")) {
        if let Some(first_null) = cmdline_bytes.iter().position(|&b| b == 0) {
            let arg0 = String::from_utf8_lossy(&cmdline_bytes[..first_null]);
            if let Some(name) = Path::new(arg0.as_ref()).file_name().and_then(|n| n.to_str()) {
                let trimmed = name.trim();
                if !trimmed.is_empty() {
                    return Some(trimmed.to_string());
                }
            }
        }
    }
    get_process_comm_by_pid_dir(pid_dir)
}

fn get_process_comm_by_pid_dir(pid_dir: &Path) -> Option<String> {
    fs::read_to_string(pid_dir.join("comm")).ok().map(|s| s.trim().to_string())
}

/// Gets the effective user UID of the current process from /proc/self/status.
fn get_current_user_uid() -> Option<u32> {
    let content = fs::read_to_string("/proc/self/status").ok()?;
    for line in content.lines() {
        if line.starts_with("Uid:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                return parts[1].parse::<u32>().ok();
            }
        }
    }
    None
}

/// Checks whether a process in /proc is owned by the current user.
fn is_process_owned_by_user(proc_path: &Path, current_uid: Option<u32>) -> bool {
    let status_path = proc_path.join("status");
    let content = match fs::read_to_string(status_path) {
        Ok(c) => c,
        Err(_) => return false,
    };
    for line in content.lines() {
        if line.starts_with("Uid:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                if let Ok(uid) = parts[1].parse::<u32>() {
                    if let Some(curr) = current_uid {
                        return uid == curr;
                    } else {
                        return uid >= 1000;
                    }
                }
            }
            break;
        }
    }
    false
}

/// Encodes binary data to standard Base64 string.
fn base64_encode_bytes(bytes: &[u8]) -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;

        result.push(CHARSET[((triple >> 18) & 0x3F) as usize] as char);
        result.push(CHARSET[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            result.push(CHARSET[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(CHARSET[(triple & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}

/// Reads an icon file from disk and formats it as a Base64 data URI.
fn read_icon_as_data_uri(path: &Path) -> Option<String> {
    let bytes = fs::read(path).ok()?;
    if bytes.is_empty() {
        return None;
    }

    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    let mime = match ext.as_str() {
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "xpm" => "image/x-xpixmap",
        _ => "image/png",
    };

    let encoded = base64_encode_bytes(&bytes);
    Some(format!("data:{mime};base64,{encoded}"))
}

/// Searches standard XDG icon themes and pixmap directories to resolve an icon to a data URI.
fn resolve_linux_icon(icon_name: &str) -> Option<String> {
    let trimmed = icon_name.trim();
    if trimmed.is_empty() {
        return None;
    }

    let direct_path = Path::new(trimmed);
    if direct_path.is_absolute() {
        if direct_path.is_file() {
            return read_icon_as_data_uri(direct_path);
        }
        for ext in &["png", "svg"] {
            let with_ext = direct_path.with_extension(ext);
            if with_ext.is_file() {
                return read_icon_as_data_uri(&with_ext);
            }
        }
    }

    let mut search_bases = vec![
        PathBuf::from("/usr/share/pixmaps"),
        PathBuf::from("/usr/share/icons/hicolor"),
        PathBuf::from("/usr/local/share/icons/hicolor"),
        PathBuf::from("/var/lib/flatpak/exports/share/icons/hicolor"),
        PathBuf::from("/var/lib/snapd/desktop/icons"),
    ];

    if let Some(home) = dirs::home_dir() {
        search_bases.push(home.join(".local/share/icons/hicolor"));
        search_bases.push(home.join(".local/share/pixmaps"));
    }

    let clean_name = trimmed
        .strip_suffix(".png")
        .or_else(|| trimmed.strip_suffix(".svg"))
        .unwrap_or(trimmed);

    let resolutions = [
        "256x256/apps",
        "128x128/apps",
        "64x64/apps",
        "48x48/apps",
        "scalable/apps",
        "32x32/apps",
    ];

    for base in &search_bases {
        if !base.exists() {
            continue;
        }

        for ext in &["png", "svg"] {
            let candidate = base.join(format!("{clean_name}.{ext}"));
            if candidate.is_file() {
                if let Some(uri) = read_icon_as_data_uri(&candidate) {
                    return Some(uri);
                }
            }
        }

        for res in &resolutions {
            let res_dir = base.join(res);
            if res_dir.exists() {
                for ext in &["png", "svg"] {
                    let candidate = res_dir.join(format!("{clean_name}.{ext}"));
                    if candidate.is_file() {
                        if let Some(uri) = read_icon_as_data_uri(&candidate) {
                            return Some(uri);
                        }
                    }
                }
            }
        }
    }

    None
}

#[derive(Default, Clone)]
struct DesktopMeta {
    name: String,
    exec: Option<String>,
    startup_wm_class: Option<String>,
    icon_base64: Option<String>,
}

/// Indexes .desktop application entries from standard XDG data directories.
fn load_desktop_entries() -> HashMap<String, DesktopMeta> {
    let mut map = HashMap::new();
    let mut search_dirs = vec![
        PathBuf::from("/usr/share/applications"),
        PathBuf::from("/usr/local/share/applications"),
        PathBuf::from("/var/lib/flatpak/exports/share/applications"),
        PathBuf::from("/var/lib/snapd/desktop/applications"),
    ];

    if let Some(home) = dirs::home_dir() {
        search_dirs.push(home.join(".local/share/applications"));
        search_dirs.push(home.join(".local/share/flatpak/exports/share/applications"));
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
                        if let Some(ref wm_class) = meta.startup_wm_class {
                            map.insert(wm_class.to_lowercase(), meta.clone());
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

/// Matches an application against indexed desktop entries using exact, class, and hyphenated forms.
fn find_desktop_meta<'a>(
    map: &'a HashMap<String, DesktopMeta>,
    app_name: &str,
) -> Option<&'a DesktopMeta> {
    let lower = app_name.trim().to_lowercase();
    let clean = lower.strip_suffix(".desktop").unwrap_or(&lower);
    let clean = clean.strip_suffix(".exe").unwrap_or(clean);

    if let Some(meta) = map.get(clean) {
        return Some(meta);
    }

    for (key, meta) in map {
        if key == clean
            || key.starts_with(&format!("{clean}-"))
            || clean.starts_with(&format!("{key}-"))
            || (meta
                .startup_wm_class
                .as_deref()
                .map(|c| c.eq_ignore_ascii_case(clean))
                .unwrap_or(false))
        {
            return Some(meta);
        }
    }

    None
}

/// Parses an individual .desktop file safely.
fn parse_desktop_file(path: &Path) -> Option<DesktopMeta> {
    let content = fs::read_to_string(path).ok()?;
    let mut in_desktop_entry = false;
    let mut name = None;
    let mut exec = None;
    let mut icon_name = None;
    let mut startup_wm_class = None;
    let mut no_display = false;
    let mut is_application = true;

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
            let key = key.trim();
            let val = val.trim();
            match key {
                "Type" => {
                    if !val.eq_ignore_ascii_case("Application") {
                        is_application = false;
                    }
                }
                "Name" if name.is_none() => name = Some(val.to_string()),
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
                "Icon" if icon_name.is_none() => icon_name = Some(val.to_string()),
                "StartupWMClass" if startup_wm_class.is_none() => {
                    startup_wm_class = Some(val.to_string())
                }
                "NoDisplay" => no_display = val.eq_ignore_ascii_case("true"),
                _ => {}
            }
        }
    }

    if !is_application || no_display {
        return None;
    }

    let final_name = name.unwrap_or_else(|| {
        path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Application")
            .to_string()
    });

    let icon_base64 = icon_name.and_then(|ic| resolve_linux_icon(&ic));

    Some(DesktopMeta {
        name: final_name,
        exec,
        startup_wm_class,
        icon_base64,
    })
}

/// Lists running applications matched against installed desktop entries and active processes.
pub fn list_running_app_items(blacklist: &[String]) -> Result<Vec<AppItemInfo>> {
    let desktop_map = load_desktop_entries();
    let current_uid = get_current_user_uid();
    let mut raw_apps: Vec<String> = Vec::new();

    if let Ok(entries) = fs::read_dir("/proc") {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Ok(file_name) = entry.file_name().into_string() {
                if file_name.chars().all(|c| c.is_ascii_digit()) {
                    if !is_process_owned_by_user(&path, current_uid) {
                        continue;
                    }

                    if let Some(proc_name) = get_process_name_from_proc(&path) {
                        if proc_name.is_empty() || is_platform_ignored_app(&proc_name) {
                            continue;
                        }

                        let is_tracked = crate::tracker::is_app_blacklisted(&proc_name, "", blacklist);
                        let is_desktop_app = find_desktop_meta(&desktop_map, &proc_name).is_some();

                        if is_desktop_app || is_tracked {
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
        let (display_name, icon_base64) = if let Some(meta) = find_desktop_meta(&desktop_map, &proc_name) {
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

        let (display_name, icon_base64) = if let Some(meta) = find_desktop_meta(&desktop_map, &clean) {
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
            let mut icon_base64 = None;

            if app_name.ends_with(".desktop") {
                if let Some(entry) = parse_desktop_file(&path_buf) {
                    if let Some(exec) = entry.exec {
                        app_name = exec;
                    }
                    display_name = entry.name;
                    icon_base64 = entry.icon_base64;
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
                icon_base64,
                is_tracked,
            }));
        }
    }

    Ok(None)
}

pub use pick_app_file as pick_exe_file;

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

    #[test]
    fn test_kwin_active_client_parsing() {
        let sample = "Support Information:\nActive Client:\nresourceClass: \"google-chrome\"\nresourceName: \"google-chrome\"\ncaption: \"New Tab - Google Chrome\"\npid: 12345\nClient:\ncaption: \"Other\"\n";
        let parsed = parse_kwin_active_client(sample);
        assert!(parsed.is_some());
        if let Some(info) = parsed {
            assert_eq!(info.process_name, "google-chrome");
            assert_eq!(info.window_title, "New Tab - Google Chrome");
            assert_eq!(info.process_id, 12345);
        }
    }

    #[test]
    fn test_base64_encode_bytes() {
        assert_eq!(base64_encode_bytes(b"hello"), "aGVsbG8=");
        assert_eq!(base64_encode_bytes(b""), "");
    }
}

