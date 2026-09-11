//! Mindsnap backend internationalization module.
//! Loads localized strings dynamically from JSON locale files with embedded fallbacks.
//! Allows translators to add or modify languages simply by placing JSON files in locales/.

use std::fs;
use std::path::PathBuf;
use serde_json::Value;

const DEFAULT_LANGUAGES_JSON: &str = include_str!("../../src/locales/languages.json");
const DEFAULT_EN_JSON: &str = include_str!("../../src/locales/en_us.json");
const DEFAULT_TR_JSON: &str = include_str!("../../src/locales/tr_tr.json");

#[cfg(target_os = "windows")]
#[link(name = "kernel32")]
extern "system" {
    fn GetUserDefaultUILanguage() -> u16;
}

/// Detects the host system language code (e.g. "tr", "en", "de").
fn detect_system_lang_code() -> String {
    #[cfg(target_os = "windows")]
    {
        let lang_id = unsafe { GetUserDefaultUILanguage() };
        let primary_lang = lang_id & 0x03FF;
        match primary_lang {
            0x1F => return "tr".to_string(),
            0x07 => return "de".to_string(),
            0x0C => return "fr".to_string(),
            0x0A => return "es".to_string(),
            0x10 => return "it".to_string(),
            0x19 => return "ru".to_string(),
            0x04 => return "zh".to_string(),
            0x11 => return "ja".to_string(),
            0x12 => return "ko".to_string(),
            0x09 => return "en".to_string(),
            _ => {}
        }
    }

    for var in ["LANG", "LC_ALL", "LC_MESSAGES"] {
        if let Ok(val) = std::env::var(var) {
            let lower = val.to_lowercase();
            if let Some(prefix) = lower.split(['_', '.', '-']).next() {
                if !prefix.is_empty() {
                    return prefix.to_string();
                }
            }
        }
    }

    "en".to_string()
}

/// Finds the best directory where locale files are stored.
fn find_locales_dir() -> Option<PathBuf> {
    let candidate_dirs = [
        PathBuf::from("locales"),
        PathBuf::from("../src/locales"),
        PathBuf::from("src/locales"),
    ];

    for dir in candidate_dirs {
        if dir.is_dir() {
            return Some(dir);
        }
    }

    if let Ok(mut exe_dir) = std::env::current_exe() {
        exe_dir.pop();
        let exe_locales = exe_dir.join("locales");
        if exe_locales.is_dir() {
            return Some(exe_locales);
        }
    }

    None
}

/// Loads languages metadata, prioritizing disk then embedded fallback.
fn load_languages_metadata() -> Value {
    if let Some(dir) = find_locales_dir() {
        let path = dir.join("languages.json");
        if let Ok(content) = fs::read_to_string(path) {
            if let Ok(val) = serde_json::from_str::<Value>(&content) {
                return val;
            }
        }
    }

    serde_json::from_str::<Value>(DEFAULT_LANGUAGES_JSON).unwrap_or_else(|_| Value::Array(Vec::new()))
}

/// Resolves a requested locale preference ("auto", "tr_tr", "en-us", etc.) to a canonical locale code.
pub fn resolve_locale_code(preference: &str) -> String {
    let clean = preference.trim().to_lowercase().replace('-', "_");
    let languages = load_languages_metadata();

    let target = if clean.is_empty() || clean == "auto" {
        detect_system_lang_code()
    } else {
        clean.clone()
    };

    if let Some(arr) = languages.as_array() {
        for item in arr {
            if let Some(code) = item.get("code").and_then(|c| c.as_str()) {
                if code.eq_ignore_ascii_case(&target) {
                    return code.to_string();
                }
            }
        }

        for item in arr {
            if let Some(code) = item.get("code").and_then(|c| c.as_str()) {
                if let Some(aliases) = item.get("aliases").and_then(|a| a.as_array()) {
                    for alias in aliases {
                        if let Some(a_str) = alias.as_str() {
                            if a_str.eq_ignore_ascii_case(&target) {
                                return code.to_string();
                            }
                        }
                    }
                }
            }
        }

        let prefix = target.split('_').next().unwrap_or(&target);
        for item in arr {
            if let Some(code) = item.get("code").and_then(|c| c.as_str()) {
                if code.starts_with(prefix) {
                    return code.to_string();
                }
            }
        }
    }

    if !clean.is_empty() && clean != "auto" {
        clean
    } else {
        "en_us".to_string()
    }
}

/// Loads a locale JSON document from disk or embedded fallback.
fn load_locale_data(canonical_code: &str) -> Option<Value> {
    if let Some(dir) = find_locales_dir() {
        let path = dir.join(format!("{}.json", canonical_code));
        if let Ok(content) = fs::read_to_string(path) {
            if let Ok(val) = serde_json::from_str::<Value>(&content) {
                return Some(val);
            }
        }
    }

    match canonical_code {
        "tr_tr" => serde_json::from_str::<Value>(DEFAULT_TR_JSON).ok(),
        "en_us" => serde_json::from_str::<Value>(DEFAULT_EN_JSON).ok(),
        _ => None,
    }
}

/// Traverses a dotted key path (e.g. "notifications.first_alert_title") within a JSON Value.
fn resolve_json_key<'a>(mut current: &'a Value, path: &str) -> Option<&'a str> {
    for segment in path.split('.') {
        match current {
            Value::Object(map) => {
                current = map.get(segment)?;
            }
            _ => return None,
        }
    }
    current.as_str()
}

/// Translates a key for the given locale preference, performing optional parameter interpolation.
pub fn t(locale_pref: &str, key: &str, params: &[(&str, &str)]) -> String {
    let canonical = resolve_locale_code(locale_pref);

    let mut text_found: Option<String> = None;

    if let Some(doc) = load_locale_data(&canonical) {
        if let Some(text) = resolve_json_key(&doc, key) {
            text_found = Some(text.to_string());
        }
    }

    if text_found.is_none() && canonical != "en_us" {
        if let Some(doc) = load_locale_data("en_us") {
            if let Some(text) = resolve_json_key(&doc, key) {
                text_found = Some(text.to_string());
            }
        }
    }

    let mut result = text_found.unwrap_or_else(|| key.to_string());

    for &(placeholder, val) in params {
        let pattern = format!("{{{}}}", placeholder);
        result = result.replace(&pattern, val);
    }

    result
}

// ---------------------------------------------------------------------------
// Convenience Helper Functions for Mindsnap Core and Notifications
// ---------------------------------------------------------------------------

pub fn first_alert_title(locale: &str) -> String {
    t(locale, "notifications.first_alert_title", &[])
}

pub fn first_alert_body(locale: &str, app_name: &str, minutes: u32) -> String {
    let min_str = minutes.to_string();
    t(
        locale,
        "notifications.first_alert_body",
        &[("app", app_name), ("minutes", &min_str)],
    )
}

pub fn repeat_alert_title(locale: &str) -> String {
    t(locale, "notifications.repeat_alert_title", &[])
}

pub fn repeat_alert_body(locale: &str, app_name: &str, total_minutes: u64) -> String {
    let min_str = total_minutes.to_string();
    t(
        locale,
        "notifications.repeat_alert_body",
        &[("app", app_name), ("minutes", &min_str)],
    )
}

pub fn tray_minimized_title(locale: &str) -> String {
    t(locale, "notifications.tray_minimized_title", &[])
}

pub fn tray_minimized_body(locale: &str) -> String {
    t(locale, "notifications.tray_minimized_body", &[])
}

pub fn tray_show(locale: &str) -> String {
    t(locale, "tray.show", &[])
}

pub fn tray_quit(locale: &str) -> String {
    t(locale, "tray.quit", &[])
}

pub fn tray_tooltip(locale: &str) -> String {
    t(locale, "tray.tooltip", &[])
}

pub fn file_picker_title(locale: &str) -> String {
    t(locale, "dialogs.file_picker_title", &[])
}

pub fn dialog_exe_filter(locale: &str) -> String {
    t(locale, "dialogs.exe_filter", &[])
}

pub fn dialog_desktop_filter(locale: &str) -> String {
    t(locale, "dialogs.desktop_filter", &[])
}

pub fn err_cannot_track_system(locale: &str) -> String {
    t(locale, "errors.cannot_track_system", &[])
}

pub fn err_invalid_app_name(locale: &str) -> String {
    t(locale, "errors.invalid_app_name", &[])
}

pub fn err_min_initial_alert(locale: &str) -> String {
    t(locale, "errors.min_initial_alert", &[])
}

pub fn err_min_repeat_alert(locale: &str) -> String {
    t(locale, "errors.min_repeat_alert", &[])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_locale_code() {
        assert_eq!(resolve_locale_code("tr_tr"), "tr_tr");
        assert_eq!(resolve_locale_code("tr-TR"), "tr_tr");
        assert_eq!(resolve_locale_code("TR"), "tr_tr");
        assert_eq!(resolve_locale_code("en_us"), "en_us");
        assert_eq!(resolve_locale_code("en-US"), "en_us");
    }

    #[test]
    fn test_dynamic_translations() {
        let title_tr = first_alert_title("tr_tr");
        assert_eq!(title_tr, "Kullanım Hatırlatıcısı");

        let title_en = first_alert_title("en_us");
        assert_eq!(title_en, "Usage Reminder");

        let body_tr = first_alert_body("tr_tr", "Discord", 5);
        assert!(body_tr.contains("Discord"));
        assert!(body_tr.contains("5"));

        let body_en = first_alert_body("en_us", "Discord", 5);
        assert!(body_en.contains("Discord"));
        assert!(body_en.contains("5"));
    }

    #[test]
    fn test_custom_language_fallback() {
        // Unknown locale falls back to English dictionary without error
        let title = first_alert_title("unknown_locale");
        assert_eq!(title, "Usage Reminder");
    }

    #[test]
    fn test_dialog_and_error_translations() {
        assert_eq!(file_picker_title("tr_tr"), "Mindsnap - Takip Edilecek Uygulamayı Seç");
        assert_eq!(file_picker_title("en_us"), "Mindsnap - Select Application to Track");
        assert_eq!(err_invalid_app_name("tr_tr"), "Geçersiz uygulama adı.");
        assert_eq!(err_invalid_app_name("en_us"), "Invalid application name.");
        assert_eq!(err_min_initial_alert("tr_tr"), "İlk uyarı süresi en az 1 dakika olmalıdır.");
        assert_eq!(err_min_initial_alert("en_us"), "Initial alert duration must be at least 1 minute.");
    }
}
