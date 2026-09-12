//! Wayland display server compatibility shim for Linux AppImage packages.
//!
//! When Tauri bundles an AppImage on an older distribution (such as Ubuntu 22.04),
//! the bundler includes `libwayland-client.so.0` inside `APPDIR/usr/lib`.
//! On modern Wayland desktops (e.g., Fedora, Arch, CachyOS, openSUSE) with Mesa 25+,
//! the host graphics driver (`libEGL_mesa.so`) requires the host's native
//! `libwayland-client` ABI. When the bundled older copy is loaded via `LD_LIBRARY_PATH`,
//! `eglGetPlatformDisplay` fails with `EGL_BAD_PARAMETER`, causing WebKitGTK to abort.
//!
//! To resolve this transparently without user intervention or disabling hardware
//! acceleration, Mindsnap detects when it is running as an AppImage on Wayland,
//! locates the host's native `libwayland-client.so.0`, prepends it to `LD_PRELOAD`,
//! and re-execs once before any GTK or WebKit initialization.

#[cfg(target_os = "linux")]
use std::path::Path;

/// Environment variable guard to prevent re-exec loops.
pub const WAYLAND_PRELOAD_GUARD_ENV: &str = "MINDSNAP_WAYLAND_PRELOAD_DONE";

/// Canonical system paths for `libwayland-client.so.0` across standard Linux distributions.
pub const CANDIDATE_HOST_LIBWAYLAND_PATHS: &[&str] = &[
    "/usr/lib64/libwayland-client.so.0",                 // Fedora, RHEL, openSUSE
    "/usr/lib/x86_64-linux-gnu/libwayland-client.so.0", // Debian, Ubuntu
    "/usr/lib/libwayland-client.so.0",                  // Arch Linux, CachyOS, Manjaro, Alpine
    "/lib64/libwayland-client.so.0",
    "/lib/x86_64-linux-gnu/libwayland-client.so.0",
    "/lib/libwayland-client.so.0",
];

/// Checks if the process is running inside an AppImage environment.
pub fn is_appimage_env(appimage: Option<&str>, appdir: Option<&str>) -> bool {
    matches!(appimage, Some(v) if !v.trim().is_empty())
        || matches!(appdir, Some(v) if !v.trim().is_empty())
}

/// Checks if the current session is running under Wayland.
pub fn is_wayland_session(wayland_display: Option<&str>, xdg_session_type: Option<&str>) -> bool {
    matches!(wayland_display, Some(v) if !v.trim().is_empty())
        || matches!(xdg_session_type, Some(v) if v.eq_ignore_ascii_case("wayland"))
}

/// Parses resolved file paths from `ldconfig -p` output for a specific soname.
pub fn parse_ldconfig_paths<'a>(soname: &str, ldconfig_output: &'a str) -> Vec<&'a str> {
    let mut results = Vec::new();
    for line in ldconfig_output.lines() {
        let trimmed = line.trim_start();
        if trimmed.split_whitespace().next() == Some(soname) {
            if let Some(idx) = trimmed.find("=> ") {
                let path = trimmed[idx + 3..].trim();
                if !path.is_empty() {
                    results.push(path);
                }
            }
        }
    }
    results
}

/// Determines whether a candidate library path is located outside the AppImage bundle directory.
pub fn is_path_outside_appdir(path: &str, appdir: Option<&str>) -> bool {
    match appdir {
        Some(dir) if !dir.trim().is_empty() => {
            let normalized_dir = dir.trim_end_matches('/');
            if path == normalized_dir {
                return false;
            }
            if let Some(rest) = path.strip_prefix(normalized_dir) {
                if rest.starts_with('/') {
                    return false;
                }
            }
            true
        }
        _ => true,
    }
}

/// Selects the best system `libwayland-client.so.0` path, preferring `ldconfig`
/// and falling back to canonical filesystem paths outside the AppDir.
pub fn pick_system_libwayland(
    appdir: Option<&str>,
    ldconfig_output: Option<&str>,
    file_exists: impl Fn(&str) -> bool,
) -> Option<String> {
    if let Some(output) = ldconfig_output {
        for path in parse_ldconfig_paths("libwayland-client.so.0", output) {
            if is_path_outside_appdir(path, appdir) && file_exists(path) {
                return Some(path.to_string());
            }
        }
    }

    for &candidate in CANDIDATE_HOST_LIBWAYLAND_PATHS {
        if is_path_outside_appdir(candidate, appdir) && file_exists(candidate) {
            return Some(candidate.to_string());
        }
    }

    None
}

/// Composes the new `LD_PRELOAD` string by prepending the chosen library.
/// Returns `None` if the library is already present in `LD_PRELOAD`.
pub fn compose_ld_preload(sys_lib: &str, existing_preload: Option<&str>) -> Option<String> {
    match existing_preload {
        Some(existing) if !existing.trim().is_empty() => {
            if existing.split(':').any(|item| item == sys_lib) {
                None
            } else {
                Some(format!("{sys_lib}:{existing}"))
            }
        }
        _ => Some(sys_lib.to_string()),
    }
}

/// Executes `ldconfig -p` to inspect the system dynamic linker cache.
#[cfg(target_os = "linux")]
fn run_ldconfig_p() -> Option<String> {
    let commands = ["ldconfig", "/sbin/ldconfig", "/usr/sbin/ldconfig"];
    for cmd in commands {
        if let Ok(output) = std::process::Command::new(cmd).arg("-p").output() {
            if output.status.success() {
                return Some(String::from_utf8_lossy(&output.stdout).into_owned());
            }
        }
    }
    None
}

/// Performs transparent self-re-exec on Linux AppImages running under Wayland.
#[cfg(target_os = "linux")]
pub fn preload_system_libwayland() {
    use std::os::unix::process::CommandExt;

    // Skip if already re-exec'd to prevent infinite loops
    if std::env::var_os(WAYLAND_PRELOAD_GUARD_ENV).is_some() {
        return;
    }

    let wayland_display = std::env::var("WAYLAND_DISPLAY").ok();
    let xdg_session_type = std::env::var("XDG_SESSION_TYPE").ok();
    if !is_wayland_session(wayland_display.as_deref(), xdg_session_type.as_deref()) {
        return;
    }

    let appimage = std::env::var("APPIMAGE").ok();
    let appdir = std::env::var("APPDIR").ok();
    if !is_appimage_env(appimage.as_deref(), appdir.as_deref()) {
        return;
    }

    let ldconfig_data = run_ldconfig_p();
    let host_lib = match pick_system_libwayland(
        appdir.as_deref(),
        ldconfig_data.as_deref(),
        |path| Path::new(path).exists(),
    ) {
        Some(path) => path,
        None => {
            eprintln!("Mindsnap: Host libwayland-client.so.0 not located; continuing with bundled copy.");
            return;
        }
    };

    let existing_preload = std::env::var("LD_PRELOAD").ok();
    let new_preload = match compose_ld_preload(&host_lib, existing_preload.as_deref()) {
        Some(val) => val,
        None => return, // Already preloaded
    };

    let current_exe = match std::env::current_exe() {
        Ok(exe) => exe,
        Err(_) => return,
    };

    let args: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();

    // Re-exec the process image in place with the host libwayland-client preloaded
    let err = std::process::Command::new(&current_exe)
        .args(&args)
        .env(WAYLAND_PRELOAD_GUARD_ENV, "1")
        .env("LD_PRELOAD", &new_preload)
        .exec();

    eprintln!("Mindsnap: Failed to re-exec with host libwayland-client: {err}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_appimage_env() {
        assert!(is_appimage_env(Some("/home/user/Mindsnap.AppImage"), None));
        assert!(is_appimage_env(None, Some("/tmp/.mount_Mind1234")));
        assert!(is_appimage_env(Some(""), Some("/tmp/.mount_Mind1234")));
        assert!(!is_appimage_env(None, None));
        assert!(!is_appimage_env(Some(""), None));
        assert!(!is_appimage_env(Some("   "), Some("")));
    }

    #[test]
    fn test_is_wayland_session() {
        assert!(is_wayland_session(Some("wayland-0"), None));
        assert!(is_wayland_session(None, Some("wayland")));
        assert!(is_wayland_session(None, Some("Wayland")));
        assert!(!is_wayland_session(None, Some("x11")));
        assert!(!is_wayland_session(None, None));
        assert!(!is_wayland_session(Some(""), Some("")));
    }

    #[test]
    fn test_parse_ldconfig_paths() {
        let output = "\tlibfoo.so.1 (libc6,x86-64) => /usr/lib64/libfoo.so.1\n\
                      \tlibwayland-client.so.0 (libc6,x86-64) => /usr/lib64/libwayland-client.so.0\n\
                      \tlibwayland-client.so.0.3 (libc6,x86-64) => /usr/lib64/libwayland-client.so.0.3\n\
                      \tlibbar.so (libc6) => /usr/lib/libbar.so\n";

        let parsed = parse_ldconfig_paths("libwayland-client.so.0", output);
        assert_eq!(parsed, vec!["/usr/lib64/libwayland-client.so.0"]);
    }

    #[test]
    fn test_is_path_outside_appdir() {
        let appdir = Some("/tmp/.mount_Mindsnap123");
        assert!(!is_path_outside_appdir("/tmp/.mount_Mindsnap123/usr/lib/libwayland-client.so.0", appdir));
        assert!(!is_path_outside_appdir("/tmp/.mount_Mindsnap123", appdir));
        assert!(is_path_outside_appdir("/tmp/.mount_Mindsnap123_backup/libwayland-client.so.0", appdir));
        assert!(is_path_outside_appdir("/usr/lib64/libwayland-client.so.0", appdir));
        assert!(is_path_outside_appdir("/usr/lib64/libwayland-client.so.0", None));
    }

    #[test]
    fn test_pick_system_libwayland() {
        let ldconfig = "\tlibwayland-client.so.0 (libc6,x86-64) => /usr/lib64/libwayland-client.so.0\n";
        let picked = pick_system_libwayland(Some("/tmp/.mount_Mindsnap"), Some(ldconfig), |p| {
            p == "/usr/lib64/libwayland-client.so.0"
        });
        assert_eq!(picked.as_deref(), Some("/usr/lib64/libwayland-client.so.0"));

        // Fallback when ldconfig is not available
        let picked_fallback = pick_system_libwayland(None, None, |p| {
            p == "/usr/lib/x86_64-linux-gnu/libwayland-client.so.0"
        });
        assert_eq!(picked_fallback.as_deref(), Some("/usr/lib/x86_64-linux-gnu/libwayland-client.so.0"));
    }

    #[test]
    fn test_compose_ld_preload() {
        let sys_lib = "/usr/lib64/libwayland-client.so.0";
        assert_eq!(compose_ld_preload(sys_lib, None).as_deref(), Some(sys_lib));
        assert_eq!(compose_ld_preload(sys_lib, Some("")).as_deref(), Some(sys_lib));
        assert_eq!(
            compose_ld_preload(sys_lib, Some("/opt/libfoo.so")).as_deref(),
            Some("/usr/lib64/libwayland-client.so.0:/opt/libfoo.so")
        );
        // Do not preload if already present
        assert_eq!(compose_ld_preload(sys_lib, Some(sys_lib)), None);
        assert_eq!(
            compose_ld_preload(sys_lib, Some("/opt/libfoo.so:/usr/lib64/libwayland-client.so.0")),
            None
        );
    }
}
