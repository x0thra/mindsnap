use std::collections::HashMap;
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::path::Path;
use std::sync::Mutex;
use windows_sys::Win32::Foundation::{CloseHandle, BOOL, HWND, LPARAM};
use windows_sys::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows_sys::Win32::Media::Audio::{
    PlaySoundW, SND_ALIAS, SND_ASYNC, SND_FILENAME, SND_NODEFAULT,
};
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW, HKEY,
    HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, REG_DWORD, REG_EXPAND_SZ, REG_SZ,
};
use windows_sys::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetForegroundWindow, GetWindowTextLengthW, GetWindowTextW,
    GetWindowThreadProcessId, IsWindowVisible,
};
use crate::error::Result;
use crate::tracker::state::{ActiveAppInfo, AppItemInfo};

/// Plays the authentic Windows default notification sound.
pub fn play_notification_sound() {
    let sound_alias: Vec<u16> = "Notification.Default\0".encode_utf16().collect();
    let played = unsafe {
        PlaySoundW(
            sound_alias.as_ptr(),
            std::ptr::null_mut(),
            SND_ALIAS | SND_ASYNC | SND_NODEFAULT,
        )
    };
    if played == 0 {
        // Fallback to the standard Windows notification audio file if registry alias is not mapped
        let fallback_path: Vec<u16> = "C:\\Windows\\media\\Windows Notify System Generic.wav\0"
            .encode_utf16()
            .collect();
        unsafe {
            PlaySoundW(
                fallback_path.as_ptr(),
                std::ptr::null_mut(),
                SND_FILENAME | SND_ASYNC,
            );
        }
    }
}

/// Returns the primary notification icon path (Windows uses AUMID and embedded resources).
pub fn get_notification_icon_path() -> Option<String> {
    None
}

unsafe fn set_reg_dword(hkey: HKEY, name: &str, value: u32) {
    let name_wide: Vec<u16> = format!("{name}\0").encode_utf16().collect();
    let data = value.to_ne_bytes();
    let _ = RegSetValueExW(
        hkey,
        name_wide.as_ptr(),
        0,
        REG_DWORD,
        data.as_ptr(),
        data.len() as u32,
    );
}

unsafe fn set_reg_sz(hkey: HKEY, name: &str, value: &str) {
    let name_wide: Vec<u16> = format!("{name}\0").encode_utf16().collect();
    let value_wide: Vec<u16> = format!("{value}\0").encode_utf16().collect();
    let _ = RegSetValueExW(
        hkey,
        name_wide.as_ptr(),
        0,
        REG_SZ,
        value_wide.as_ptr() as *const u8,
        (value_wide.len() * 2) as u32,
    );
}

fn register_notification_registry_settings() {
    let current_exe = std::env::current_exe()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "mindsnap.exe".to_string());

    unsafe {
        // 1. Register AppUserModelId for Windows Action Center
        let aumid_path = "Software\\Classes\\AppUserModelId\\com.mindsnap.desktop\0";
        let aumid_wide: Vec<u16> = aumid_path.encode_utf16().collect();
        let mut hkey: HKEY = std::ptr::null_mut();
        if RegCreateKeyW(HKEY_CURRENT_USER, aumid_wide.as_ptr(), &mut hkey) == 0 {
            set_reg_sz(hkey, "DisplayName", "Mindsnap");
            set_reg_sz(hkey, "IconUri", &current_exe);
            set_reg_dword(hkey, "ShowInSettings", 1);
            RegCloseKey(hkey);
        }

        // 2. Enable Banners & Action Center display
        let settings_path = "Software\\Microsoft\\Windows\\CurrentVersion\\Notifications\\Settings\\com.mindsnap.desktop\0";
        let settings_wide: Vec<u16> = settings_path.encode_utf16().collect();
        let mut hkey_settings: HKEY = std::ptr::null_mut();
        if RegCreateKeyW(HKEY_CURRENT_USER, settings_wide.as_ptr(), &mut hkey_settings) == 0 {
            set_reg_dword(hkey_settings, "Enabled", 1);
            set_reg_dword(hkey_settings, "ShowInActionCenter", 1);
            set_reg_dword(hkey_settings, "ShowBanners", 1);
            RegCloseKey(hkey_settings);
        }
    }
}

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

fn ensure_start_menu_shortcut() {
    let current_exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(_) => return,
    };
    let app_data = match dirs::data_dir() {
        Some(d) => d,
        None => return,
    };
    let shortcut_path = app_data.join("Microsoft\\Windows\\Start Menu\\Programs\\Mindsnap.lnk");

    if shortcut_path.exists() {
        return;
    }

    let target_str = current_exe.to_string_lossy().to_string();
    let shortcut_str = shortcut_path.to_string_lossy().to_string();

    std::thread::spawn(move || {
        use std::os::windows::process::CommandExt;
        let script = format!(
            r#"$cs = @"
using System;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.ComTypes;
namespace SC {{
    [ComImport, Guid("00021401-0000-0000-C000-000000000046")]
    public class SL {{}}
    [ComImport, InterfaceType(ComInterfaceType.InterfaceIsIUnknown), Guid("000214F9-0000-0000-C000-000000000046")]
    public interface IS {{
        void GP(out IntPtr a, int b, out IntPtr c, int d);
        void GI(out IntPtr a);
        void SI(IntPtr a);
        void GD(out IntPtr a, int b);
        void SD(string a);
        void GW(out IntPtr a, int b);
        void SW(string a);
        void GA(out IntPtr a, int b);
        void SA(string a);
        void GH(out short a);
        void SH(short a);
        void GS(out int a);
        void SS(int a);
        void GL(out IntPtr a, int b, out int c);
        void SLI(string a, int b);
        void SR(string a, int b);
        void R(IntPtr a, int b);
        void SetPath([MarshalAs(UnmanagedType.LPWStr)] string a);
    }}
    [ComImport, InterfaceType(ComInterfaceType.InterfaceIsIUnknown), Guid("886D8EEB-8CF2-4446-8D02-CDBA1DBDCF99")]
    public interface IP {{
        void GC(out uint a);
        void GA(uint a, out PK b);
        void GV(ref PK a, out PV b);
        void SV(ref PK a, ref PV b);
        void Commit();
    }}
    [StructLayout(LayoutKind.Sequential, Pack = 4)]
    public struct PK {{
        public Guid a; public uint b;
        public PK(Guid g, uint i) {{ a = g; b = i; }}
    }}
    [StructLayout(LayoutKind.Explicit)]
    public struct PV {{
        [FieldOffset(0)] public ushort a;
        [FieldOffset(8)] public IntPtr b;
        public static PV S(string s) {{
            var v = new PV(); v.a = 31; v.b = Marshal.StringToCoTaskMemUni(s); return v;
        }}
    }}
    public class C {{
        public static void M(string p, string t, string a) {{
            var l = (IS)new SL();
            l.SetPath(t);
            var ps = (IP)l;
            var k = new PK(new Guid("9F4C2855-9F79-4B39-A8D0-E1D42DE1D5F3"), 5);
            var v = PV.S(a);
            ps.SV(ref k, ref v);
            ps.Commit();
            ((IPersistFile)l).Save(p, true);
        }}
    }}
}}
"@
Add-Type -TypeDefinition $cs
[SC.C]::M('{shortcut_str}', '{target_str}', 'com.mindsnap.desktop')
"#
        );

        let utf16_bytes: Vec<u8> = script
            .encode_utf16()
            .flat_map(|u| u.to_le_bytes())
            .collect();
        let encoded = base64_encode_bytes(&utf16_bytes);

        let _ = std::process::Command::new("powershell")
            .args(["-NoProfile", "-WindowStyle", "Hidden", "-EncodedCommand", &encoded])
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .output();
    });
}

/// Initializes Windows Toast Notification subsystem: registers AUMID, ensures banner permissions and shortcut.
pub fn init_platform_notifications() {
    let aumid: Vec<u16> = "com.mindsnap.desktop\0".encode_utf16().collect();
    unsafe {
        let _ = SetCurrentProcessExplicitAppUserModelID(aumid.as_ptr());
    }
    register_notification_registry_settings();
    ensure_start_menu_shortcut();
}

static ICON_CACHE: Mutex<Option<HashMap<String, Option<String>>>> = Mutex::new(None);
static EXE_ICON_CACHE: Mutex<Option<HashMap<String, String>>> = Mutex::new(None);

/// Retrieves cached icon for an executable name (case-insensitive).
pub fn get_cached_exe_icon(exe_name: &str) -> Option<String> {
    let mut guard = match EXE_ICON_CACHE.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    guard.get_or_insert_with(HashMap::new).get(&exe_name.to_lowercase()).cloned()
}

/// Caches icon for an executable name.
pub fn cache_exe_icon(exe_name: &str, icon: String) {
    let mut guard = match EXE_ICON_CACHE.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    let map = guard.get_or_insert_with(HashMap::new);
    if map.len() >= 250 {
        map.clear();
    }
    map.insert(exe_name.to_lowercase(), icon);
}

/// Queries the Windows Registry App Paths keys for an executable's installed path.
fn get_app_path_from_registry(exe_name: &str) -> Option<String> {
    unsafe {
        for root in [HKEY_LOCAL_MACHINE, HKEY_CURRENT_USER] {
            let subkey = format!(
                "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\App Paths\\{}\0",
                exe_name
            );
            let subkey_wide: Vec<u16> = subkey.encode_utf16().collect();
            let mut hkey: HKEY = std::ptr::null_mut();

            if RegOpenKeyExW(root, subkey_wide.as_ptr(), 0, KEY_READ, &mut hkey) == 0 {
                let mut data_type: u32 = 0;
                let mut data_size: u32 = 0;

                let res = RegQueryValueExW(
                    hkey,
                    std::ptr::null(),
                    std::ptr::null_mut(),
                    &mut data_type,
                    std::ptr::null_mut(),
                    &mut data_size,
                );

                if res == 0 && data_size > 0 && (data_type == REG_SZ || data_type == REG_EXPAND_SZ) {
                    let mut buffer: Vec<u8> = vec![0; data_size as usize];
                    let read_res = RegQueryValueExW(
                        hkey,
                        std::ptr::null(),
                        std::ptr::null_mut(),
                        &mut data_type,
                        buffer.as_mut_ptr(),
                        &mut data_size,
                    );

                    RegCloseKey(hkey);

                    if read_res == 0 {
                        let u16_slice = std::slice::from_raw_parts(
                            buffer.as_ptr() as *const u16,
                            (data_size as usize) / 2,
                        );
                        let mut path = String::from_utf16_lossy(u16_slice);
                        path = path.trim_matches('\0').trim().to_string();
                        let clean_path = path.trim_matches('"').to_string();
                        if Path::new(&clean_path).is_file() {
                            return Some(clean_path);
                        }
                    }
                } else {
                    RegCloseKey(hkey);
                }
            }
        }
    }
    None
}

/// Searches common installation folders for applications that might not register in App Paths.
fn find_known_app_path(exe_name: &str) -> Option<String> {
    let lower = exe_name.to_lowercase();
    let local_appdata = std::env::var("LOCALAPPDATA").ok();
    let appdata = std::env::var("APPDATA").ok();
    let program_files = std::env::var("ProgramFiles").ok();
    let program_files_x86 = std::env::var("ProgramFiles(x86)").ok();

    match lower.as_str() {
        "discord.exe" => {
            if let Some(ref local) = local_appdata {
                let discord_dir = Path::new(local).join("Discord");
                if let Ok(entries) = std::fs::read_dir(discord_dir) {
                    for entry in entries.flatten() {
                        let p = entry.path().join("Discord.exe");
                        if p.is_file() {
                            return Some(p.to_string_lossy().to_string());
                        }
                    }
                }
            }
            None
        }
        "spotify.exe" => {
            if let Some(ref roaming) = appdata {
                let p = Path::new(roaming).join("Spotify").join("Spotify.exe");
                if p.is_file() {
                    return Some(p.to_string_lossy().to_string());
                }
            }
            None
        }
        "code.exe" => {
            if let Some(ref local) = local_appdata {
                let p = Path::new(local).join("Programs").join("Microsoft VS Code").join("Code.exe");
                if p.is_file() {
                    return Some(p.to_string_lossy().to_string());
                }
            }
            if let Some(ref pf) = program_files {
                let p = Path::new(pf).join("Microsoft VS Code").join("Code.exe");
                if p.is_file() {
                    return Some(p.to_string_lossy().to_string());
                }
            }
            None
        }
        "steam.exe" => {
            if let Some(ref pfx86) = program_files_x86 {
                let p = Path::new(pfx86).join("Steam").join("steam.exe");
                if p.is_file() {
                    return Some(p.to_string_lossy().to_string());
                }
            }
            if let Some(ref pf) = program_files {
                let p = Path::new(pf).join("Steam").join("steam.exe");
                if p.is_file() {
                    return Some(p.to_string_lossy().to_string());
                }
            }
            None
        }
        "telegram.exe" => {
            if let Some(ref roaming) = appdata {
                let p = Path::new(roaming).join("Telegram Desktop").join("Telegram.exe");
                if p.is_file() {
                    return Some(p.to_string_lossy().to_string());
                }
            }
            None
        }
        _ => None,
    }
}

/// Attempts to locate the installed executable path on disk even when closed.
pub fn find_executable_path(exe_name: &str) -> Option<String> {
    if let Some(path) = get_app_path_from_registry(exe_name) {
        return Some(path);
    }
    if let Some(path) = find_known_app_path(exe_name) {
        return Some(path);
    }
    None
}

/// Retrieves and caches the Base64 PNG icon for the specified file path.
pub fn get_icon_for_path(path: &str) -> Option<String> {
    let mut guard = match ICON_CACHE.lock() {
        Ok(g) => g,
        Err(_) => return None,
    };
    let cache = guard.get_or_insert_with(HashMap::new);

    let key = path.to_lowercase();
    if let Some(cached) = cache.get(&key) {
        return cached.clone();
    }

    if cache.len() >= 250 {
        cache.clear();
    }

    let icon_data = match windows_icons::get_icon_base64_by_path(path) {
        Ok(b64) if !b64.is_empty() => {
            if b64.starts_with("data:") {
                Some(b64)
            } else {
                Some(format!("data:image/png;base64,{}", b64))
            }
        }
        _ => None,
    };

    cache.insert(key, icon_data.clone());
    icon_data
}

/// Resolves popular process executable names to user-friendly display titles.
pub fn get_friendly_app_name(exe_name: &str) -> String {
    let lower = exe_name.to_lowercase();
    match lower.as_str() {
        "chrome.exe" => "Google Chrome".to_string(),
        "msedge.exe" => "Microsoft Edge".to_string(),
        "discord.exe" => "Discord".to_string(),
        "spotify.exe" => "Spotify".to_string(),
        "steam.exe" => "Steam".to_string(),
        "code.exe" => "Visual Studio Code".to_string(),
        "telegram.exe" => "Telegram".to_string(),
        "twitter.exe" => "Twitter / X".to_string(),
        "whatsapp.exe" => "WhatsApp".to_string(),
        "slack.exe" => "Slack".to_string(),
        "notion.exe" => "Notion".to_string(),
        "obs64.exe" | "obs32.exe" => "OBS Studio".to_string(),
        "devenv.exe" => "Visual Studio".to_string(),
        "rider64.exe" => "JetBrains Rider".to_string(),
        "idea64.exe" => "IntelliJ IDEA".to_string(),
        "pycharm64.exe" => "PyCharm".to_string(),
        "clion64.exe" => "CLion".to_string(),
        "vlc.exe" => "VLC Media Player".to_string(),
        "brave.exe" => "Brave Browser".to_string(),
        "firefox.exe" => "Mozilla Firefox".to_string(),
        "opera.exe" => "Opera".to_string(),
        "epicgameslauncher.exe" => "Epic Games".to_string(),
        "leagueclient.exe" => "League of Legends".to_string(),
        "valorant.exe" => "VALORANT".to_string(),
        "teams.exe" | "ms-teams.exe" => "Microsoft Teams".to_string(),
        "outlook.exe" => "Microsoft Outlook".to_string(),
        "excel.exe" => "Microsoft Excel".to_string(),
        "winword.exe" => "Microsoft Word".to_string(),
        "powerpnt.exe" => "Microsoft PowerPoint".to_string(),
        "blender.exe" => "Blender".to_string(),
        "figma.exe" => "Figma".to_string(),
        "postman.exe" => "Postman".to_string(),
        "dbeaver.exe" => "DBeaver".to_string(),
        "battle.net.exe" => "Battle.net".to_string(),
        "riotclientux.exe" => "Riot Client".to_string(),
        "zoom.exe" => "Zoom".to_string(),
        "skype.exe" => "Skype".to_string(),
        "cursor.exe" => "Cursor".to_string(),
        "zed.exe" => "Zed".to_string(),
        "sublime_text.exe" => "Sublime Text".to_string(),
        "notepad++.exe" => "Notepad++".to_string(),
        "notepad.exe" => "Notepad".to_string(),
        "thunderbird.exe" => "Mozilla Thunderbird".to_string(),
        "arc.exe" => "Arc Browser".to_string(),
        "zen.exe" => "Zen Browser".to_string(),
        "tor.exe" => "Tor Browser".to_string(),
        "vivaldi.exe" => "Vivaldi".to_string(),
        "chatgpt.exe" => "ChatGPT".to_string(),
        _ => {
            let base = exe_name.strip_suffix(".exe").unwrap_or(exe_name);
            let words: Vec<String> = base
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
                base.to_string()
            } else {
                words.join(" ")
            }
        }
    }
}

/// Retrieves active foreground window and process metadata.
pub fn get_active_window() -> Result<Option<ActiveAppInfo>> {
    unsafe {
        let hwnd: HWND = GetForegroundWindow();
        if hwnd.is_null() {
            return Ok(None);
        }

        let mut process_id: u32 = 0;
        GetWindowThreadProcessId(hwnd, &mut process_id);
        if process_id == 0 {
            return Ok(None);
        }

        let process_name = match get_process_name_by_pid(process_id) {
            Some(name) => name,
            None => return Ok(None),
        };

        let window_title = get_window_title(hwnd);

        Ok(Some(ActiveAppInfo {
            process_name,
            window_title,
            process_id,
        }))
    }
}

/// Retrieves window title text for a window handle.
unsafe fn get_window_title(hwnd: HWND) -> String {
    let title_len = GetWindowTextLengthW(hwnd);
    if title_len <= 0 {
        return String::new();
    }

    let buffer_len = (title_len + 1) as usize;
    let mut buffer: Vec<u16> = vec![0; buffer_len];
    let copied = GetWindowTextW(hwnd, buffer.as_mut_ptr(), buffer_len as i32);
    if copied > 0 {
        let os_str = OsString::from_wide(&buffer[..copied as usize]);
        os_str.to_string_lossy().trim().to_string()
    } else {
        String::new()
    }
}

/// Resolves executable file name from process ID.
unsafe fn get_process_name_by_pid(pid: u32) -> Option<String> {
    get_process_full_path_by_pid(pid).and_then(|full_path| {
        let path = Path::new(&full_path);
        path.file_name()
            .and_then(|name| name.to_str())
            .map(|s| s.to_string())
    })
}

/// Resolves full executable filesystem path from process ID.
unsafe fn get_process_full_path_by_pid(pid: u32) -> Option<String> {
    let process_handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
    if process_handle.is_null() {
        return None;
    }

    let mut buffer = [0u16; 1024];
    let mut size = buffer.len() as u32;

    let success = QueryFullProcessImageNameW(
        process_handle,
        0,
        buffer.as_mut_ptr(),
        &mut size,
    );

    CloseHandle(process_handle);

    if success != 0 && size > 0 {
        let path_str = OsString::from_wide(&buffer[..size as usize]);
        Some(path_str.to_string_lossy().to_string())
    } else {
        None
    }
}

/// Filters out desktop shell and background system window titles.
fn is_ignored_window_title(title: &str) -> bool {
    let lower = title.trim().to_lowercase();
    if lower.is_empty() {
        return true;
    }
    matches!(
        lower.as_str(),
        "program manager"
            | "windows input experience"
            | "windows giriş deneyimi"
            | "default ime"
            | "msctfime ui"
            | "settings"
            | "ayarlar"
    )
}

/// Enumerates open top-level windows with resolved display names and icons.
pub fn list_running_app_items(blacklist: &[String]) -> Result<Vec<AppItemInfo>> {
    let mut raw_apps: Vec<(String, String, String)> = Vec::new(); // (exe_name, window_title, full_path)

    unsafe {
        let apps_ptr = &mut raw_apps as *mut Vec<(String, String, String)> as LPARAM;
        EnumWindows(Some(enum_windows_full_callback), apps_ptr);
    }

    raw_apps.sort_by_key(|a| a.0.to_lowercase());
    raw_apps.dedup_by(|a, b| a.0.eq_ignore_ascii_case(&b.0));

    let mut result = Vec::new();
    for (exe_name, window_title, full_path) in raw_apps {
        if crate::tracker::is_system_or_ignored_app(&exe_name) {
            continue;
        }

        let display_name = get_friendly_app_name(&exe_name);
        let icon_base64 = get_icon_for_path(&full_path);
        if let Some(ref ic) = icon_base64 {
            cache_exe_icon(&exe_name, ic.clone());
        }
        let is_tracked = crate::tracker::is_app_blacklisted(&exe_name, &window_title, blacklist);

        result.push(AppItemInfo {
            exe_name,
            display_name,
            window_title,
            icon_base64,
            is_tracked,
        });
    }

    Ok(result)
}

/// Returns list of running top-level windows for backward compatibility.
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

/// Generates rich display items (icons and friendly names) for blacklisted apps.
pub fn get_tracked_app_items(blacklist: &[String]) -> Result<Vec<AppItemInfo>> {
    let running = list_running_app_items(blacklist).unwrap_or_default();
    let mut path_map: HashMap<String, String> = HashMap::new();
    for item in running {
        if let Some(icon) = item.icon_base64 {
            cache_exe_icon(&item.exe_name, icon.clone());
            path_map.insert(item.exe_name.to_lowercase(), icon);
        }
    }

    let mut result = Vec::new();
    for app in blacklist {
        let exe_name = app.trim().to_string();
        if exe_name.is_empty() || crate::tracker::is_system_or_ignored_app(&exe_name) {
            continue;
        }

        let display_name = get_friendly_app_name(&exe_name);
        let key = exe_name.to_lowercase();

        let icon_base64 = path_map.get(&key).cloned().or_else(|| {
            get_cached_exe_icon(&exe_name).or_else(|| {
                find_executable_path(&exe_name).and_then(|path| {
                    let icon = get_icon_for_path(&path);
                    if let Some(ref ic) = icon {
                        cache_exe_icon(&exe_name, ic.clone());
                    }
                    icon
                })
            })
        });

        result.push(AppItemInfo {
            exe_name,
            display_name,
            window_title: String::new(),
            icon_base64,
            is_tracked: true,
        });
    }

    Ok(result)
}

/// Identifies Windows shell utilities, system services, and background components.
pub fn is_platform_ignored_app(exe_name: &str) -> bool {
    let lower = exe_name.trim().to_lowercase();
    let name_only = lower.strip_suffix(".exe").unwrap_or(&lower);

    if name_only == "mindsnap" || lower == "mindsnap.exe" {
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

    if matches!(name_only, "msedgewebview2" | "webview2") {
        return true;
    }

    matches!(
        name_only,
        "explorer"
            | "shellexperiencehost"
            | "startmenuexperiencehost"
            | "searchhost"
            | "searchapp"
            | "searchui"
            | "textinputhost"
            | "applicationframehost"
            | "systemsettings"
            | "systemsettingsadminflows"
            | "lockapp"
            | "runtimebroker"
            | "taskhostw"
            | "dwm"
            | "svchost"
            | "ctfmon"
            | "conhost"
            | "dllhost"
            | "sihost"
            | "fontdrvhost"
            | "widgetservice"
            | "widgets"
            | "smartscreen"
            | "securityhealthsystray"
            | "securityhealthservice"
            | "compattelrunner"
            | "audiodg"
            | "spoolsv"
            | "taskmgr"
    )
}

/// Opens native file dialog and returns selected executable item info.
pub fn pick_app_file(blacklist: &[String], locale: &str) -> Result<Option<AppItemInfo>> {
    let dialog_title = crate::i18n::file_picker_title(locale);
    let filter_name = crate::i18n::dialog_exe_filter(locale);
    let file = rfd::FileDialog::new()
        .add_filter(&filter_name, &["exe"])
        .set_title(&dialog_title)
        .pick_file();

    if let Some(path_buf) = file {
        if let Some(file_name) = path_buf.file_name().and_then(|n| n.to_str()) {
            let exe_name = file_name.to_string();
            if crate::tracker::is_system_or_ignored_app(&exe_name) {
                return Err(crate::error::MindsnapError::Config(
                    crate::i18n::err_cannot_track_system(locale),
                ));
            }

            let display_name = get_friendly_app_name(&exe_name);
            let full_path_str = path_buf.to_string_lossy().to_string();
            let icon_base64 = get_icon_for_path(&full_path_str);
            if let Some(ref ic) = icon_base64 {
                cache_exe_icon(&exe_name, ic.clone());
            }
            let is_tracked = crate::tracker::is_app_blacklisted(&exe_name, "", blacklist);

            return Ok(Some(AppItemInfo {
                exe_name,
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

unsafe extern "system" fn enum_windows_full_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    if IsWindowVisible(hwnd) == 0 {
        return 1;
    }

    let title_len = GetWindowTextLengthW(hwnd);
    if title_len <= 0 {
        return 1;
    }

    let mut process_id: u32 = 0;
    GetWindowThreadProcessId(hwnd, &mut process_id);
    if process_id == 0 {
        return 1;
    }

    if let Some(full_path) = get_process_full_path_by_pid(process_id) {
        let path = Path::new(&full_path);
        if let Some(process_name) = path.file_name().and_then(|n| n.to_str()) {
            if !crate::tracker::is_system_or_ignored_app(process_name) {
                let window_title = get_window_title(hwnd);
                if !is_ignored_window_title(&window_title) {
                    let apps = &mut *(lparam as *mut Vec<(String, String, String)>);
                    apps.push((process_name.to_string(), window_title, full_path));
                }
            }
        }
    }

    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_running_app_items() {
        let windows = list_running_app_items(&[]);
        assert!(windows.is_ok());
        if let Ok(apps) = windows {
            for app in apps {
                assert!(!crate::tracker::is_system_or_ignored_app(&app.exe_name));
            }
        }
    }

    #[test]
    fn test_get_friendly_app_name() {
        assert_eq!(get_friendly_app_name("chrome.exe"), "Google Chrome");
        assert_eq!(get_friendly_app_name("discord.exe"), "Discord");
        assert_eq!(get_friendly_app_name("my-custom-game.exe"), "My Custom Game");
    }

    #[test]
    fn test_get_active_window() {
        let active = get_active_window();
        assert!(active.is_ok());
    }

    #[test]
    fn test_exe_icon_caching() {
        cache_exe_icon("testapp.exe", "data:image/png;base64,testdata".to_string());
        assert_eq!(
            get_cached_exe_icon("TESTAPP.EXE").as_deref(),
            Some("data:image/png;base64,testdata")
        );
        assert_eq!(
            get_cached_exe_icon("testapp.exe").as_deref(),
            Some("data:image/png;base64,testdata")
        );
        assert!(get_cached_exe_icon("nonexistent_unknown.exe").is_none());
    }

    #[test]
    fn test_play_notification_sound() {
        // Must execute safely without panic
        play_notification_sound();
    }

    #[test]
    fn test_init_platform_notifications() {
        // Must execute safely and initialize registry and notifications
        init_platform_notifications();
        // Give background thread a moment to create the start menu shortcut if needed
        std::thread::sleep(std::time::Duration::from_millis(2500));
        if let Some(app_data) = dirs::data_dir() {
            let shortcut = app_data.join("Microsoft\\Windows\\Start Menu\\Programs\\Mindsnap.lnk");
            assert!(shortcut.exists(), "Mindsnap.lnk shortcut should have been created");
        }
    }
}
