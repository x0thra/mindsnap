pub mod commands;
pub mod config;
pub mod error;
pub mod i18n;
pub mod tracker;

use std::sync::Arc;
use tokio::sync::RwLock;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, WindowEvent,
};

use crate::commands::AppState;
use crate::config::AppConfig;
use crate::tracker::state::TrackerStatus;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let initial_config = AppConfig::load();
    let config_store = Arc::new(RwLock::new(initial_config.clone()));
    let status_store = Arc::new(RwLock::new(TrackerStatus::default()));

    let app_state = AppState {
        config: Arc::clone(&config_store),
        tracker_status: Arc::clone(&status_store),
    };

    if let Err(err) = tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::save_config,
            commands::sync_locale,
            commands::get_tracker_status,
            commands::get_running_apps,
            commands::get_tracked_apps,
            commands::pick_and_add_exe,
            commands::add_app_to_blacklist,
            commands::remove_app_from_blacklist,
            commands::app_minimize,
            commands::app_toggle_maximize,
            commands::app_close,
            commands::get_app_version,
            commands::open_external_url,
            commands::resolve_locale,
        ])
        .setup(move |app| {
            tracker::platform::init_platform_notifications();

            // System tray menu
            let lang = &initial_config.language;
            let show_item = MenuItem::with_id(app, "show", i18n::tray_show(lang), true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", i18n::tray_quit(lang), true, None::<&str>)?;
            let tray_menu = Menu::with_items(app, &[&show_item, &quit_item])?;

            let tray_icon = match app.default_window_icon() {
                Some(icon) => icon.clone(),
                None => {
                    eprintln!("Default window icon not found.");
                    return Ok(());
                }
            };

            let _tray = TrayIconBuilder::new()
                .icon(tray_icon)
                .menu(&tray_menu)
                .tooltip(i18n::tray_tooltip(lang))
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            if let Ok(is_visible) = window.is_visible() {
                                if is_visible {
                                    let _ = window.hide();
                                } else {
                                    let _ = window.show();
                                    let _ = window.set_focus();
                                }
                            }
                        }
                    }
                })
                .build(app)?;

            // Spawn background tracking loop
            let app_handle = app.handle().clone();
            let thread_config = Arc::clone(&config_store);
            let thread_status = Arc::clone(&status_store);

            tauri::async_runtime::spawn(async move {
                tracker::run_tracker_loop(app_handle, thread_config, thread_status).await;
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let (minimize_to_tray, lang, sound_enabled) = {
                    if let Some(state) = window.try_state::<AppState>() {
                        if let Ok(config) = state.config.try_read() {
                            (
                                config.minimize_to_tray_on_close,
                                config.language.clone(),
                                config.sound_enabled,
                            )
                        } else {
                            (true, "en_us".to_string(), true)
                        }
                    } else {
                        (true, "en_us".to_string(), true)
                    }
                };

                if minimize_to_tray {
                    api.prevent_close();
                    let _ = window.hide();
                    tracker::send_notification(
                        window.app_handle(),
                        &i18n::tray_minimized_title(&lang),
                        &i18n::tray_minimized_body(&lang),
                        sound_enabled,
                    );
                } else {
                    window.app_handle().exit(0);
                }
            }
        })
        .run(tauri::generate_context!())
    {
        eprintln!("Failed to run Mindsnap: {:?}", err);
    }
}
