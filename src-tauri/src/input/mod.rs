use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::config::SharedConfig;
use crate::{keymap, tts};

mod gamepad;

#[cfg(any(target_os = "windows", target_os = "macos"))]
mod rdev_backend;
#[cfg(any(target_os = "windows", target_os = "macos"))]
use rdev_backend as platform;

#[cfg(target_os = "linux")]
mod evdev_backend;
#[cfg(target_os = "linux")]
mod wayland_keymap;
#[cfg(target_os = "linux")]
pub mod kwin_focus;
#[cfg(target_os = "linux")]
use evdev_backend as platform;

// Per-OS enumeration of each physical key's base label in the active layout, so
// the on-screen keyboard shows the user's real layout immediately. Each platform
// uses its native API (Linux xkbcommon, Windows ToUnicodeEx, macOS UCKeyTranslate).
#[cfg(target_os = "linux")]
pub use evdev_backend::key_labels;

#[cfg(target_os = "windows")]
mod labels_windows;
#[cfg(target_os = "windows")]
pub use labels_windows::key_labels;

#[cfg(target_os = "macos")]
mod labels_macos;
#[cfg(target_os = "macos")]
pub use labels_macos::key_labels;

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
pub fn key_labels(_layout_override: Option<&str>) -> std::collections::HashMap<String, String> {
    std::collections::HashMap::new()
}

/// Modifier state at the time of a key event.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Mods {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub meta: bool,
}

/// Normalized input event; every platform backend emits exactly this.
#[derive(Debug)]
pub enum RawInput {
    Key {
        /// OS-translated character(s) for printable keys (layout- and
        /// shift-correct on every platform, any language).
        text: Option<String>,
        /// Backend-normalized id for non-printable keys ("backspace", "f1", …).
        named: Option<&'static str>,
        /// Physical key position, W3C-KeyboardEvent-style ("KeyQ", "Digit1",
        /// "Enter", …). Layout-independent, so the on-screen keyboard lights the
        /// right cap on QWERTZ/AZERTY/etc. `None` for keys not on the keyboard.
        code: Option<&'static str>,
        mods: Mods,
        /// True when this press is an auto-repeat of a held key.
        repeat: bool,
    },
    /// A modifier key changed state, with its physical side ("ShiftLeft", …).
    /// Modifiers don't produce popups; this drives the on-screen keyboard so a
    /// held modifier lights up (and the correct left/right cap).
    Modifier {
        code: &'static str,
        pressed: bool,
    },
    MouseButton {
        button: u8,
    },
    /// Relative mouse movement since the last event. Emitted per axis on Linux
    /// (evdev sends REL_X and REL_Y separately), combined on Windows/macOS.
    MouseMotion {
        dx: f64,
        dy: f64,
    },
    /// Scroll wheel: positive dy = up, positive dx = right (one tick per notch).
    Scroll {
        dx: f64,
        dy: f64,
    },
    /// A gamepad button changed state. `id` is a shared id ("gp_a", "dpad_up", …).
    GamepadButton {
        id: &'static str,
        pressed: bool,
    },
    /// A gamepad analog axis moved. `axis` is a shared id ("ls_x", "rt", …);
    /// sticks range -1.0..1.0, triggers 0.0..1.0.
    GamepadAxis {
        axis: &'static str,
        value: f64,
    },
    /// A gamepad connected (true) or disconnected (false); drives widget visibility.
    GamepadConnection {
        connected: bool,
    },
}

/// A problem a platform backend ran into that needs user-visible handling.
#[derive(Debug)]
#[allow(dead_code)] // variants are platform-specific
pub enum BackendIssue {
    /// Linux: /dev/input exists but nothing is readable.
    InputPermission,
    /// macOS: the Accessibility permission is missing.
    Accessibility(String),
    Other(String),
}

/// Parsed toggle-capture hotkey, e.g. "Ctrl+Alt+Y".
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Hotkey {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub meta: bool,
    /// Lowercase key: single character ("y") or named id ("f9").
    pub key: String,
}

impl Hotkey {
    pub fn parse(spec: &str) -> Option<Self> {
        let mut hotkey = Hotkey::default();
        for part in spec.split('+') {
            let part = part.trim();
            match part.to_lowercase().as_str() {
                "" => return None,
                "ctrl" | "control" => hotkey.ctrl = true,
                "alt" | "option" => hotkey.alt = true,
                "shift" => hotkey.shift = true,
                "meta" | "super" | "cmd" | "win" => hotkey.meta = true,
                key => {
                    if !hotkey.key.is_empty() {
                        return None; // two non-modifier keys
                    }
                    hotkey.key = key.to_string();
                }
            }
        }
        // Require at least one modifier so plain typing can never toggle capture.
        if hotkey.key.is_empty() || !(hotkey.ctrl || hotkey.alt || hotkey.shift || hotkey.meta) {
            return None;
        }
        Some(hotkey)
    }

    pub fn matches(&self, text: Option<&str>, named: Option<&str>, mods: &Mods) -> bool {
        if mods.ctrl != self.ctrl
            || mods.alt != self.alt
            || mods.shift != self.shift
            || mods.meta != self.meta
        {
            return false;
        }
        if let Some(id) = named {
            return id == self.key;
        }
        if let Some(t) = text {
            return t.to_lowercase() == self.key;
        }
        false
    }
}

/// Latest analog device state, updated by the consumer thread and sampled by
/// the emitter thread at ~60 Hz. Mouse motion accumulates between samples;
/// gamepad axes hold their latest value.
#[derive(Default)]
struct DeviceState {
    mouse_dx: f64,
    mouse_dy: f64,
    ls_x: f64,
    ls_y: f64,
    rs_x: f64,
    rs_y: f64,
    lt: f64,
    rt: f64,
    gamepad_connected: bool,
}

impl DeviceState {
    fn set_axis(&mut self, axis: &str, value: f64) {
        match axis {
            "ls_x" => self.ls_x = value,
            "ls_y" => self.ls_y = value,
            "rs_x" => self.rs_x = value,
            "rs_y" => self.rs_y = value,
            "lt" => self.lt = value,
            "rt" => self.rt = value,
            _ => {}
        }
    }

    fn reset_gamepad(&mut self) {
        self.ls_x = 0.0;
        self.ls_y = 0.0;
        self.rs_x = 0.0;
        self.rs_y = 0.0;
        self.lt = 0.0;
        self.rt = 0.0;
    }
}

/// Snapshot emitted to the overlay's device widget. Carries the config knobs
/// the widget needs so the frontend never has to fetch config separately.
#[derive(Debug, Clone, Serialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
struct DeviceSnapshot {
    mouse_dx: f64,
    mouse_dy: f64,
    ls_x: f64,
    ls_y: f64,
    rs_x: f64,
    rs_y: f64,
    lt: f64,
    rt: f64,
    gamepad_connected: bool,
    show_mouse_movement: bool,
    show_gamepad: bool,
    sensitivity: f64,
    decay_seconds: f64,
    scale: f64,
}

/// Emits `device-state` to the overlay at ~60 Hz, but only when something
/// changed (mouse moved, an axis moved, or a controller connected). The
/// overlay runs its own animation loop for smooth decay, so idle frames are
/// unnecessary.
fn spawn_device_emitter(app: AppHandle, config: SharedConfig, state: Arc<Mutex<DeviceState>>) {
    std::thread::spawn(move || {
        // Axes/connection last emitted, to detect change (mouse delta excluded).
        let mut last = DeviceSnapshot::default();
        loop {
            std::thread::sleep(Duration::from_millis(16));
            let cfg = match config.read() {
                Ok(cfg) => cfg.clone(),
                Err(_) => continue,
            };
            if !cfg.show_mouse_movement && !cfg.show_gamepad {
                continue;
            }

            let snapshot = {
                let mut guard = match state.lock() {
                    Ok(guard) => guard,
                    Err(_) => continue,
                };
                let snapshot = DeviceSnapshot {
                    mouse_dx: guard.mouse_dx,
                    mouse_dy: guard.mouse_dy,
                    ls_x: guard.ls_x,
                    ls_y: guard.ls_y,
                    rs_x: guard.rs_x,
                    rs_y: guard.rs_y,
                    lt: guard.lt,
                    rt: guard.rt,
                    gamepad_connected: guard.gamepad_connected,
                    show_mouse_movement: cfg.show_mouse_movement,
                    show_gamepad: cfg.show_gamepad,
                    sensitivity: cfg.mouse_movement_sensitivity,
                    decay_seconds: cfg.mouse_movement_decay_seconds,
                    scale: cfg.device_widget_scale,
                };
                // Drain the accumulated motion; axes persist.
                guard.mouse_dx = 0.0;
                guard.mouse_dy = 0.0;
                snapshot
            };

            let moved = snapshot.mouse_dx != 0.0 || snapshot.mouse_dy != 0.0;
            // Compare everything except the (already-drained) mouse delta.
            let axes_changed = DeviceSnapshot {
                mouse_dx: 0.0,
                mouse_dy: 0.0,
                ..snapshot.clone()
            } != last;
            if !moved && !axes_changed {
                continue;
            }
            last = DeviceSnapshot {
                mouse_dx: 0.0,
                mouse_dy: 0.0,
                ..snapshot.clone()
            };
            let _ = app.emit_to("overlay", "device-state", &snapshot);
            if crate::obs_server::has_clients(&app) {
                if let Ok(json) = serde_json::to_string(&snapshot) {
                    crate::obs_server::broadcast(&app, "device-state", &json);
                }
            }
        }
    });
}

/// Spawns the platform input backend and the consumer thread that turns raw
/// events into popup labels, TTS, and hotkey toggles.
pub fn start(app: AppHandle, config: SharedConfig, capturing: Arc<AtomicBool>) {
    let (tx, rx) = mpsc::channel::<RawInput>();

    let device_state = Arc::new(Mutex::new(DeviceState::default()));
    spawn_device_emitter(app.clone(), config.clone(), device_state.clone());

    // Gamepad runs on every platform and feeds the same channel.
    gamepad::spawn_listener(tx.clone());

    let on_issue = {
        let app = app.clone();
        move |issue: BackendIssue| handle_issue(&app, issue)
    };

    #[cfg(target_os = "linux")]
    {
        let layout_override = config
            .read()
            .ok()
            .map(|cfg| cfg.keyboard_layout.clone())
            .filter(|layout| !layout.trim().is_empty());
        platform::spawn_listener(tx, layout_override, on_issue);
    }
    #[cfg(not(target_os = "linux"))]
    platform::spawn_listener(tx, on_issue);

    std::thread::spawn(move || {
        let mut speaker = tts::Speaker::new();
        let mut tts_setup_offered = false;

        for event in rx {
            let cfg = match config.read() {
                Ok(guard) => guard.clone(),
                Err(_) => continue,
            };

            // Hotkey first: it must work even while capturing is off.
            if let RawInput::Key {
                text,
                named,
                mods,
                repeat,
                ..
            } = &event
            {
                if !repeat {
                    if let Some(hotkey) = Hotkey::parse(&cfg.toggle_capture_hotkey) {
                        if hotkey.matches(text.as_deref(), *named, mods) {
                            let now = !capturing.load(Ordering::Relaxed);
                            capturing.store(now, Ordering::Relaxed);
                            continue;
                        }
                    }
                }
            }

            if !capturing.load(Ordering::Relaxed) {
                continue;
            }

            // Analog motion / gamepad axes / connection feed the device widget
            // (via the 60 Hz emitter), never the popup stack.
            match &event {
                RawInput::MouseMotion { dx, dy } => {
                    if cfg.show_mouse_movement {
                        if let Ok(mut guard) = device_state.lock() {
                            guard.mouse_dx += dx;
                            guard.mouse_dy += dy;
                        }
                    }
                    continue;
                }
                RawInput::GamepadAxis { axis, value } => {
                    if cfg.show_gamepad {
                        if let Ok(mut guard) = device_state.lock() {
                            // Any axis activity proves a controller is present,
                            // even if we missed the connect event at startup.
                            guard.gamepad_connected = true;
                            guard.set_axis(axis, *value);
                        }
                    }
                    continue;
                }
                RawInput::GamepadConnection { connected } => {
                    if let Ok(mut guard) = device_state.lock() {
                        guard.gamepad_connected = *connected;
                        if !*connected {
                            guard.reset_gamepad();
                        }
                    }
                    continue;
                }
                // A held modifier lights (and un-lights) its exact cap.
                RawInput::Modifier { code, pressed } => {
                    if cfg.display_style == "keyboard" && cfg.show_keyboard_click {
                        let payload = serde_json::json!({ "code": code, "pressed": pressed });
                        emit_key_flash(&app, &payload);
                    }
                    continue;
                }
                _ => {}
            }

            // Keyboard-skin mode: flash the pressed cap by physical position, and
            // relabel it from the character the OS produced (so the displayed
            // keyboard matches the user's actual layout).
            if cfg.display_style == "keyboard" && cfg.show_keyboard_click {
                if let RawInput::Key {
                    text,
                    code: Some(code),
                    mods,
                    repeat: false,
                    ..
                } = &event
                {
                    // Only relabel from an unshifted press, to capture base chars.
                    let label = if mods.shift { None } else { text.as_deref() };
                    let payload = serde_json::json!({ "code": code, "label": label });
                    emit_key_flash(&app, &payload);
                }
            }

            let op = match &event {
                RawInput::Key {
                    text,
                    named,
                    mods,
                    repeat,
                    ..
                } => {
                    if !cfg.show_keyboard_click {
                        continue;
                    }
                    keymap::key_op(text.as_deref(), *named, mods, *repeat, &cfg)
                }
                RawInput::MouseButton { button } => {
                    if !cfg.show_mouse_click {
                        continue;
                    }
                    let coords = if cfg.show_mouse_coordinates {
                        cursor_position(&app)
                    } else {
                        None
                    };
                    Some(keymap::PopupOp::Append {
                        text: keymap::format_mouse(*button, coords, &cfg),
                    })
                }
                RawInput::Scroll { dx, dy } => {
                    if !cfg.show_mouse_scroll {
                        continue;
                    }
                    Some(keymap::PopupOp::Append {
                        text: keymap::format_scroll(*dx, *dy, &cfg),
                    })
                }
                RawInput::GamepadButton { id, pressed } => {
                    if !cfg.show_gamepad {
                        continue;
                    }
                    if let Ok(mut guard) = device_state.lock() {
                        guard.gamepad_connected = true;
                    }
                    // Only presses produce a popup token; releases just kept the
                    // connected flag fresh above.
                    if !*pressed {
                        continue;
                    }
                    Some(keymap::PopupOp::Append {
                        text: keymap::format_gamepad_button(id, &cfg),
                    })
                }
                // Analog / connection variants were handled above.
                _ => continue,
            };

            let Some(op) = op else { continue };

            let _ = app.emit_to("overlay", "click-event", &op);
            if crate::obs_server::has_clients(&app) {
                if let Ok(json) = serde_json::to_string(&op) {
                    crate::obs_server::broadcast(&app, "click-event", &json);
                }
            }

            if cfg.text_to_speech {
                if let keymap::PopupOp::Append { text } = &op {
                    // First use without a working engine: offer to install one
                    // (Linux; Windows/macOS engines are part of the OS).
                    if !speaker.available() && !tts_setup_offered {
                        tts_setup_offered = true;
                        #[cfg(target_os = "linux")]
                        if crate::setup::offer_tts_install(&app) {
                            speaker = tts::Speaker::new();
                        }
                    }
                    speaker.speak(text, cfg.text_to_speech_cancel_speech_on_new_key);
                }
            }
        }
    });
}

/// Emits a `key-flash` event to the native overlay and any OBS browser clients.
fn emit_key_flash(app: &AppHandle, payload: &serde_json::Value) {
    let _ = app.emit_to("overlay", "key-flash", payload);
    if crate::obs_server::has_clients(app) {
        crate::obs_server::broadcast(app, "key-flash", &payload.to_string());
    }
}

/// Global cursor position. Works natively on Windows/macOS/X11. On Wayland the
/// compositor hides the global cursor from applications, and the XWayland
/// fallback returns stale garbage — better to show nothing than wrong numbers.
fn cursor_position(app: &AppHandle) -> Option<(i32, i32)> {
    #[cfg(target_os = "linux")]
    if std::env::var("WAYLAND_DISPLAY").is_ok() {
        return None;
    }
    app.cursor_position()
        .ok()
        .map(|pos| (pos.x as i32, pos.y as i32))
}

/// Routes a backend issue to the right user-visible handling: guided setup
/// dialogs where a fix can be applied, the overlay banner otherwise.
fn handle_issue(app: &AppHandle, issue: BackendIssue) {
    match issue {
        BackendIssue::InputPermission => {
            #[cfg(target_os = "linux")]
            {
                if crate::setup::offer_input_access_fix(app) {
                    return; // device rescan picks the keyboards up shortly
                }
                report_error(
                    app,
                    "YAKC cannot read your input devices.\n\nManual fix:\n\
                     sudo usermod -aG input $USER\n…then log out and back in."
                        .to_string(),
                );
            }
            #[cfg(not(target_os = "linux"))]
            let _ = app;
        }
        BackendIssue::Accessibility(message) => {
            #[cfg(target_os = "macos")]
            crate::setup::offer_accessibility_fix(app);
            report_error(app, message);
        }
        BackendIssue::Other(message) => report_error(app, message),
    }
}

/// Buffer of errors raised before the overlay page attached its listeners;
/// the overlay drains it via the `get_pending_errors` command on startup.
#[derive(Default)]
pub struct PendingErrors(pub std::sync::Mutex<Vec<String>>);

/// Surfaces a backend error on the overlay (styled banner) and stderr.
pub fn report_error(app: &AppHandle, message: String) {
    eprintln!("YAKC: {message}");
    if let Some(pending) = app.try_state::<PendingErrors>() {
        if let Ok(mut buffer) = pending.0.lock() {
            buffer.push(message.clone());
        }
    }
    if let Some(overlay) = app.get_webview_window("overlay") {
        let _ = overlay.emit("yakc-error", &message);
    }
    if let Ok(json) = serde_json::to_string(&message) {
        crate::obs_server::broadcast(app, "yakc-error", &json);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hotkey_spec() {
        let hotkey = Hotkey::parse("Ctrl+Alt+Y").unwrap();
        assert!(hotkey.ctrl && hotkey.alt && !hotkey.shift && !hotkey.meta);
        assert_eq!(hotkey.key, "y");
    }

    #[test]
    fn rejects_hotkey_without_modifiers() {
        assert_eq!(Hotkey::parse("y"), None);
        assert_eq!(Hotkey::parse(""), None);
    }

    #[test]
    fn hotkey_matches_exact_modifier_state() {
        let hotkey = Hotkey::parse("Ctrl+Alt+Y").unwrap();
        let full = Mods {
            ctrl: true,
            alt: true,
            shift: false,
            meta: false,
        };
        assert!(hotkey.matches(Some("y"), None, &full));
        assert!(hotkey.matches(Some("Y"), None, &full));
        let partial = Mods {
            ctrl: true,
            ..Default::default()
        };
        assert!(!hotkey.matches(Some("y"), None, &partial));
    }

    #[test]
    fn hotkey_matches_named_keys() {
        let hotkey = Hotkey::parse("Ctrl+F9").unwrap();
        let mods = Mods {
            ctrl: true,
            ..Default::default()
        };
        assert!(hotkey.matches(None, Some("f9"), &mods));
    }
}
