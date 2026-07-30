#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod filter;
mod input;
mod keymap;
mod obs_server;
mod overlay;
mod profiles;
mod setup;
mod tts;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};

use tauri::menu::{IsMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Listener, Manager, State, Wry};

use config::{Config, SharedConfig};
use keymap::KnownKey;

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
fn get_known_keys() -> Vec<KnownKey> {
    keymap::known_keys()
}

/// Names of currently-running processes, de-duplicated and sorted, so the
/// settings UI can offer a pick-from-a-list process filter instead of relying
/// on the user typing exact executable names. Cross-platform via `sysinfo`.
#[tauri::command]
fn get_running_processes() -> Vec<String> {
    use sysinfo::{ProcessRefreshKind, RefreshKind, System};
    let sys = System::new_with_specifics(
        RefreshKind::nothing().with_processes(ProcessRefreshKind::nothing()),
    );
    let mut names: Vec<String> = sys
        .processes()
        .values()
        .map(|p| p.name().to_string_lossy().to_string())
        .filter(|n| !n.is_empty())
        .collect();
    names.sort_by_key(|n| n.to_lowercase());
    names.dedup();
    names
}

/// Human-readable descriptors for each connected monitor, index-aligned with
/// `showOnMonitor`, so the settings UI can show a dropdown of real monitors
/// instead of a bare index the user has to guess.
#[tauri::command]
fn get_monitors(app: AppHandle) -> Vec<String> {
    let Some(window) = app.get_webview_window("overlay") else {
        return Vec::new();
    };
    match window.available_monitors() {
        Ok(monitors) => monitors
            .iter()
            .enumerate()
            .map(|(i, m)| {
                let name = m
                    .name()
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| "Monitor".to_string());
                let size = m.size();
                format!("{i}: {name} ({}×{})", size.width, size.height)
            })
            .collect(),
        Err(_) => Vec::new(),
    }
}

/// Base labels per physical key from the OS layout, so the on-screen keyboard
/// shows the user's real layout immediately (QWERTZ/AZERTY/…), not QWERTY.
#[tauri::command]
fn get_key_labels(state: State<SharedConfig>) -> std::collections::HashMap<String, String> {
    let layout = state
        .read()
        .ok()
        .map(|cfg| cfg.keyboard_layout.clone())
        .filter(|layout| !layout.trim().is_empty());
    input::key_labels(layout.as_deref())
}

/// Enters drag-to-position mode on the overlay (temporary non-click-through).
#[tauri::command]
fn begin_overlay_move(app: AppHandle) {
    overlay::begin_move(&app);
}

/// Leaves drag-to-position mode, restoring click-through.
#[tauri::command]
fn end_overlay_move(app: AppHandle, state: State<SharedConfig>) {
    let config = state.read().map(|cfg| cfg.clone()).unwrap_or_default();
    overlay::end_move(&app, &config);
}

/// Makes `config` the live configuration: stores it, persists config.json, and
/// applies every side effect (filter re-evaluation, overlay placement/visibility,
/// OBS server, and the config-updated event to the overlay + browser source).
/// Shared by save, profile load, preset apply, and import.
pub fn apply_active(app: &AppHandle, config: &Config) -> Result<(), String> {
    if let Some(state) = app.try_state::<SharedConfig>() {
        if let Ok(mut guard) = state.write() {
            *guard = config.clone();
        }
    }
    config::save(app, config)?;
    // Apply the (possibly toggled) process filter immediately, so enabling or
    // disabling it takes effect at once — no wait for a poll tick.
    if let Some(engine) = app.try_state::<Arc<filter::Engine>>() {
        engine.reevaluate();
    }
    overlay::apply_placement(app, config);
    // Start the OBS server if this just enabled it (no restart needed).
    obs_server::ensure_started(app, config);
    if let Ok(json) = serde_json::to_string(config) {
        obs_server::broadcast(app, "config-updated", &json);
    }
    app.emit("config-updated", config).map_err(|e| e.to_string())
}

#[tauri::command]
fn save_config(app: AppHandle, config: Config) -> Result<(), String> {
    apply_active(&app, &config)
}

fn toggle_capturing(capturing: &Capturing) {
    let now = !capturing.load(Ordering::Relaxed);
    capturing.store(now, Ordering::Relaxed);
}

/// Builds the tray menu, including a Profiles submenu listing saved profiles
/// (the active one marked ●). Rebuilt whenever profiles change.
fn build_tray_menu(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let toggle = MenuItem::with_id(app, "toggle", "Toggle Capturing", true, None::<&str>)?;
    let move_item = MenuItem::with_id(app, "move", "Move overlay position", true, None::<&str>)?;

    // Profiles submenu: one entry per saved profile + a Manage… shortcut.
    let active = profiles::get_active_profile(app.clone());
    let names = profiles::list_profiles(app.clone());
    let mut owned: Vec<MenuItem<Wry>> = Vec::new();
    for name in &names {
        let label = if *name == active {
            format!("● {name}")
        } else {
            format!("    {name}")
        };
        owned.push(MenuItem::with_id(
            app,
            format!("profile:{name}"),
            label,
            true,
            None::<&str>,
        )?);
    }
    let separator = PredefinedMenuItem::separator(app)?;
    let manage = MenuItem::with_id(app, "manage_profiles", "Manage profiles…", true, None::<&str>)?;
    let mut items: Vec<&dyn IsMenuItem<Wry>> = owned
        .iter()
        .map(|item| item as &dyn IsMenuItem<Wry>)
        .collect();
    if !owned.is_empty() {
        items.push(&separator);
    }
    items.push(&manage);
    let profiles_menu = Submenu::with_items(app, "Profiles", true, &items)?;

    let settings = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    Menu::with_items(
        app,
        &[&toggle, &move_item, &profiles_menu, &settings, &quit],
    )
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
            get_known_keys,
            get_running_processes,
            get_monitors,
            get_key_labels,
            get_pending_errors,
            save_config,
            begin_overlay_move,
            end_overlay_move,
            profiles::list_profiles,
            profiles::get_active_profile,
            profiles::save_profile,
            profiles::load_profile,
            profiles::delete_profile,
            profiles::rename_profile,
            profiles::list_presets,
            profiles::apply_preset,
            profiles::export_config,
            profiles::import_config
        ])
        .setup(|app| {
            let handle = app.handle().clone();

            let cfg = config::load(&handle);
            let shared: SharedConfig = Arc::new(RwLock::new(cfg.clone()));
            let capturing: Capturing = Arc::new(AtomicBool::new(true));
            app.manage(shared.clone());
            app.manage(capturing.clone());
            app.manage(input::PendingErrors::default());

            // OBS browser-source server (opt-in; starts on demand from settings).
            app.manage(Arc::new(obs_server::ObsHub::new()));
            obs_server::ensure_started(&handle, &cfg);

            overlay::create(&handle, &cfg)?;
            overlay::create_settings(&handle)?;
            if std::env::var("YAKC_SHOW_SETTINGS").is_ok() {
                overlay::show_settings(&handle);
            }

            // Tray
            let menu = build_tray_menu(&handle)?;
            let tray_capturing = capturing.clone();
            TrayIconBuilder::with_id("main")
                .icon(app.default_window_icon().cloned().expect("bundled icon"))
                .tooltip("YAKC - Yet Another Key Caster")
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(move |app, event| {
                    let id = event.id.as_ref();
                    match id {
                        "toggle" => toggle_capturing(&tray_capturing),
                        "move" => overlay::begin_move(app),
                        "settings" => overlay::show_settings(app),
                        "manage_profiles" => {
                            overlay::show_settings(app);
                            let _ = app.emit("open-profiles-tab", ());
                        }
                        "quit" => app.exit(0),
                        _ if id.starts_with("profile:") => {
                            let name = id.trim_start_matches("profile:").to_string();
                            let _ = profiles::load_profile(app.clone(), name);
                        }
                        _ => {}
                    }
                })
                .build(app)?;

            // Rebuild the tray menu whenever profiles change, so the submenu and
            // the active-profile marker stay current.
            let tray_handle = handle.clone();
            handle.listen("profiles-updated", move |_| {
                if let (Some(tray), Ok(menu)) = (
                    tray_handle.tray_by_id("main"),
                    build_tray_menu(&tray_handle),
                ) {
                    let _ = tray.set_menu(Some(menu));
                }
            });

            input::start(handle.clone(), shared.clone(), capturing.clone());

            // Process filter. On a Wayland session the focused window is hidden
            // from X11 tools, so we take a push-based source (KDE exposes focus
            // via KWin's D-Bus scripting); elsewhere the filter polls the focused
            // window itself.
            let wayland_active: Option<filter::ActiveApp> = {
                #[cfg(target_os = "linux")]
                {
                    if std::env::var("WAYLAND_DISPLAY").is_ok() {
                        Some(Arc::new(std::sync::Mutex::new(None)))
                    } else {
                        None
                    }
                }
                #[cfg(not(target_os = "linux"))]
                {
                    None
                }
            };
            let filter_engine = filter::Engine::new(shared, capturing, wayland_active);
            app.manage(filter_engine.clone());
            #[cfg(target_os = "linux")]
            input::kwin_focus::start(filter_engine.clone());
            filter::spawn(filter_engine);

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
