use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};

use tauri::{AppHandle, Emitter, Manager};

use crate::config::SharedConfig;
use crate::{keymap, tts};

#[cfg(any(target_os = "windows", target_os = "macos"))]
mod rdev_backend;
#[cfg(any(target_os = "windows", target_os = "macos"))]
use rdev_backend as platform;

#[cfg(target_os = "linux")]
mod evdev_backend;
#[cfg(target_os = "linux")]
mod wayland_keymap;
#[cfg(target_os = "linux")]
use evdev_backend as platform;

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
        mods: Mods,
        /// True when this press is an auto-repeat of a held key.
        repeat: bool,
    },
    MouseButton {
        button: u8,
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

/// Spawns the platform input backend and the consumer thread that turns raw
/// events into popup labels, TTS, and hotkey toggles.
pub fn start(app: AppHandle, config: SharedConfig, capturing: Arc<AtomicBool>) {
    let (tx, rx) = mpsc::channel::<RawInput>();

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

            let op = match &event {
                RawInput::Key {
                    text,
                    named,
                    mods,
                    repeat,
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
            };

            let Some(op) = op else { continue };

            let _ = app.emit_to("overlay", "click-event", &op);

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
