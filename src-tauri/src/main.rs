#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod filter;
mod input;
mod keymap;
mod overlay;
mod setup;
mod tts;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};

use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, State};

use config::{Config, SharedConfig};

/// Whether keystroke capture is currently active (toggled by tray, hotkey and
/// the process filter).
type Capturing = Arc<AtomicBool>;

#[tauri::command]
fn get_config(config: State<SharedConfig>) -> Config {
    config.read().map(|c| c.clone()).unwrap_or_default()
}

#[tauri::command]
fn get_config_path(app: AppHandle) -> String {
    config::config_path(&app).display().to_string()
}

/// Drains errors that were raised before the overlay page was listening.
#[tauri::command]
fn get_pending_errors(pending: State<input::PendingErrors>) -> Vec<String> {
    pending
        .0
        .lock()
        .map(|mut buffer| std::mem::take(&mut *buffer))
        .unwrap_or_default()
}

#[tauri::command]
fn save_config(
    app: AppHandle,
    state: State<SharedConfig>,
    config: Config,
) -> Result<(), String> {
    if let Ok(mut guard) = state.write() {
        *guard = config.clone();
    }
    config::save(&app, &config)?;
    overlay::apply_placement(&app, &config);
    app.emit("config-updated", &config).map_err(|e| e.to_string())
}

fn toggle_capturing(capturing: &Capturing) {
    let now = !capturing.load(Ordering::Relaxed);
    capturing.store(now, Ordering::Relaxed);
}

fn main() {
    // On Wayland, run the overlay through XWayland: always-on-top, global
    // positioning and click-through all work there on every compositor
    // (native Wayland would need layer-shell, which GNOME doesn't support).
    // Set YAKC_NATIVE_WAYLAND=1 to opt out and run natively.
    #[cfg(target_os = "linux")]
    if std::env::var("WAYLAND_DISPLAY").is_ok()
        && std::env::var("YAKC_NATIVE_WAYLAND").is_err()
        && std::env::var("DISPLAY").is_ok()
    {
        std::env::set_var("GDK_BACKEND", "x11");
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|_app, _args, _cwd| {
            // Second launch: nothing to do, the running instance keeps going.
        }))
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            get_config,
            get_config_path,
            get_pending_errors,
            save_config
        ])
        .setup(|app| {
            let handle = app.handle().clone();

            let cfg = config::load(&handle);
            let shared: SharedConfig = Arc::new(RwLock::new(cfg.clone()));
            let capturing: Capturing = Arc::new(AtomicBool::new(true));
            app.manage(shared.clone());
            app.manage(capturing.clone());
            app.manage(input::PendingErrors::default());

            overlay::create(&handle, &cfg)?;
            overlay::create_settings(&handle)?;
            if std::env::var("YAKC_SHOW_SETTINGS").is_ok() {
                overlay::show_settings(&handle);
            }

            // Tray
            let toggle_item =
                MenuItem::with_id(app, "toggle", "Toggle Capturing", true, None::<&str>)?;
            let settings_item =
                MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&toggle_item, &settings_item, &quit_item])?;

            let tray_capturing = capturing.clone();
            TrayIconBuilder::with_id("main")
                .icon(app.default_window_icon().cloned().expect("bundled icon"))
                .tooltip("YAKC - Yet Another Key Caster")
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(move |app, event| match event.id.as_ref() {
                    "toggle" => toggle_capturing(&tray_capturing),
                    "settings" => overlay::show_settings(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;

            input::start(handle.clone(), shared.clone(), capturing.clone());
            filter::spawn(shared, capturing);

            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing the settings window hides it; the app lives in the tray.
            if window.label() == "settings" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running YAKC");
}
