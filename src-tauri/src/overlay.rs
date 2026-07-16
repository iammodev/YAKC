use tauri::{
    AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};

use crate::config::Config;

/// Creates the transparent, click-through, always-on-top overlay window
/// covering the configured monitor.
pub fn create(app: &AppHandle, config: &Config) -> tauri::Result<WebviewWindow> {
    let window = WebviewWindowBuilder::new(app, "overlay", WebviewUrl::App("index.html".into()))
        .title("YAKC - Yet Another Key Caster")
        .transparent(true)
        .decorations(false)
        .shadow(false)
        .resizable(false)
        .maximizable(false)
        .minimizable(false)
        .focused(false)
        .focusable(false)
        .skip_taskbar(true)
        .always_on_top(true)
        .visible_on_all_workspaces(true)
        .build()?;

    window.set_ignore_cursor_events(true)?;
    place_on_monitor(app, &window, config);

    #[cfg(target_os = "macos")]
    raise_above_fullscreen(&window);

    Ok(window)
}

/// Moves/sizes the overlay to fill the monitor selected by `showOnMonitor`.
pub fn place_on_monitor(app: &AppHandle, window: &WebviewWindow, config: &Config) {
    let monitor = app
        .available_monitors()
        .ok()
        .and_then(|monitors| monitors.into_iter().nth(config.show_on_monitor))
        .or_else(|| app.primary_monitor().ok().flatten());

    let Some(monitor) = monitor else { return };
    let position = *monitor.position();
    let size = *monitor.size();

    let _ = window.set_position(PhysicalPosition::new(position.x, position.y));
    let _ = window.set_size(PhysicalSize::new(size.width, size.height));
    let _ = window.set_always_on_top(true);
}

/// Re-applies monitor placement after a settings change.
pub fn apply_placement(app: &AppHandle, config: &Config) {
    if let Some(window) = app.get_webview_window("overlay") {
        place_on_monitor(app, &window, config);
    }
}

/// Creates the (initially hidden) settings window.
pub fn create_settings(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    WebviewWindowBuilder::new(app, "settings", WebviewUrl::App("settings.html".into()))
        .title("YAKC Settings")
        .inner_size(560.0, 680.0)
        .min_inner_size(420.0, 400.0)
        .visible(false)
        .build()
}

pub fn show_settings(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("settings") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

/// macOS: lift the overlay to screen-saver level so it also renders above
/// fullscreen apps, and keep it on every Space. (kCGScreenSaverWindowLevel =
/// 1000; collection behavior: CanJoinAllSpaces | Stationary | FullScreenAuxiliary)
#[cfg(target_os = "macos")]
fn raise_above_fullscreen(window: &WebviewWindow) {
    use objc2::msg_send;
    use objc2::runtime::AnyObject;

    if let Ok(ns_window) = window.ns_window() {
        let ns_window = ns_window as *mut AnyObject;
        unsafe {
            let _: () = msg_send![ns_window, setLevel: 1000isize];
            let behavior: usize = (1 << 0) | (1 << 4) | (1 << 8);
            let _: () = msg_send![ns_window, setCollectionBehavior: behavior];
        }
    }
}
