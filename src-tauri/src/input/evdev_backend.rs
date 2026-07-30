//! Global input hook for Linux, reading /dev/input directly (evdev) so key
//! capture works identically on X11 and Wayland, on any compositor.
//! Keycodes are translated to characters with xkbcommon using the active
//! keyboard layout, so any language works without per-layout tables.
//!
//! Requires read access to /dev/input/event* (user in the `input` group or an
//! equivalent udev rule) — the same requirement tools like showmethekey have.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use evdev::Device;
use xkbcommon::xkb;

use super::{BackendIssue, Mods, RawInput};

const RESCAN_INTERVAL: Duration = Duration::from_secs(3);

/// Raw event forwarded from a device reader thread to the translator thread.
/// Keys need xkb translation (which owns non-Send state); relative-axis events
/// (mouse movement / scroll) are forwarded straight through.
enum RawEvent {
    Key(evdev::KeyCode, i32),
    Rel(evdev::RelativeAxisCode, i32),
}

pub fn spawn_listener(
    tx: Sender<RawInput>,
    layout_override: Option<String>,
    on_issue: impl Fn(BackendIssue) + Send + Sync + 'static,
) {
    // xkb::State is not Send, so device reader threads forward raw
    // (keycode, value) pairs to one translator thread that owns the state.
    let (raw_tx, raw_rx) = std::sync::mpsc::channel::<RawEvent>();

    std::thread::spawn({
        let layout = layout_override.clone();
        move || {
            let mut state = match build_xkb_state(layout.as_deref()) {
                Ok(state) => state,
                Err(err) => {
                    eprintln!("YAKC: cannot initialize keyboard layout (xkb): {err}");
                    // "us" always compiles; reaching this means xkb itself is broken.
                    return;
                }
            };
            for event in raw_rx {
                match event {
                    RawEvent::Key(code, value) => handle_key_event(code, value, &tx, &mut state),
                    RawEvent::Rel(axis, value) => handle_rel_event(axis, value, &tx),
                }
            }
        }
    });

    std::thread::spawn(move || {
        let open_paths: Arc<Mutex<HashSet<PathBuf>>> = Arc::new(Mutex::new(HashSet::new()));
        let mut permission_error_reported = false;

        // Initial scan + hotplug rescan loop.
        loop {
            let mut readable = 0usize;
            let mut denied = 0usize;

            for (path, device) in evdev::enumerate() {
                if open_paths.lock().unwrap().contains(&path) {
                    readable += 1;
                    continue;
                }
                if !is_keyboard(&device) && !is_mouse(&device) {
                    continue;
                }
                readable += 1;
                open_paths.lock().unwrap().insert(path.clone());
                spawn_device_reader(path, device, raw_tx.clone(), open_paths.clone());
            }

            // enumerate() silently skips devices we cannot open; detect that.
            if let Ok(entries) = std::fs::read_dir("/dev/input") {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let is_event_node = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .is_some_and(|n| n.starts_with("event"));
                    if is_event_node && std::fs::File::open(&path).is_err() {
                        denied += 1;
                    }
                }
            }

            if readable == 0 && denied > 0 && !permission_error_reported {
                permission_error_reported = true;
                on_issue(BackendIssue::InputPermission);
            }

            std::thread::sleep(RESCAN_INTERVAL);
        }
    });
}

fn is_keyboard(device: &Device) -> bool {
    device.supported_keys().is_some_and(|keys| {
        keys.contains(evdev::KeyCode::KEY_A) || keys.contains(evdev::KeyCode::KEY_ENTER)
    })
}

fn is_mouse(device: &Device) -> bool {
    device
        .supported_keys()
        .is_some_and(|keys| keys.contains(evdev::KeyCode::BTN_LEFT))
}

fn spawn_device_reader(
    path: PathBuf,
    mut device: Device,
    raw_tx: Sender<RawEvent>,
    open_paths: Arc<Mutex<HashSet<PathBuf>>>,
) {
    std::thread::spawn(move || {
        loop {
            let events = match device.fetch_events() {
                Ok(events) => events.collect::<Vec<_>>(),
                Err(_) => break, // device unplugged or read error: drop the reader
            };
            for event in events {
                match event.destructure() {
                    evdev::EventSummary::Key(_, code, value) => {
                        let _ = raw_tx.send(RawEvent::Key(code, value));
                    }
                    evdev::EventSummary::RelativeAxis(_, axis, value) => {
                        let _ = raw_tx.send(RawEvent::Rel(axis, value));
                    }
                    _ => {}
                }
            }
        }
        open_paths.lock().unwrap().remove(&path);
    });
}

fn handle_key_event(
    code: evdev::KeyCode,
    value: i32,
    tx: &Sender<RawInput>,
    state: &mut xkb::State,
) {
    // Mouse buttons (BTN_*)
    if let Some(button) = mouse_button(code) {
        if value == 1 {
            let _ = tx.send(RawInput::MouseButton { button });
        }
        return;
    }

    // evdev keycode + 8 == X11/xkb keycode
    let keycode = xkb::Keycode::new(code.0 as u32 + 8);

    match value {
        1 | 2 => {
            // press (1) or autorepeat (2); repeats show popups like the original
            if is_modifier(code) {
                if value == 1 {
                    if let Some(mcode) = modifier_code(code) {
                        let _ = tx.send(RawInput::Modifier {
                            code: mcode,
                            pressed: true,
                        });
                    }
                    state.update_key(keycode, xkb::KeyDirection::Down);
                }
                return;
            }

            let mods = Mods {
                ctrl: state.mod_name_is_active(xkb::MOD_NAME_CTRL, xkb::STATE_MODS_EFFECTIVE),
                alt: state.mod_name_is_active(xkb::MOD_NAME_ALT, xkb::STATE_MODS_EFFECTIVE),
                shift: state.mod_name_is_active(xkb::MOD_NAME_SHIFT, xkb::STATE_MODS_EFFECTIVE),
                meta: state.mod_name_is_active(xkb::MOD_NAME_LOGO, xkb::STATE_MODS_EFFECTIVE),
            };

            let named = named_for(code);
            let text = if named.is_none() {
                // key_get_utf8 applies the Control transformation (control
                // codes); fall back to the bare keysym for Ctrl+letter combos.
                let mut text = state.key_get_utf8(keycode);
                if text.is_empty() || text.chars().all(char::is_control) {
                    let keysym = state.key_get_one_sym(keycode);
                    text = xkb::keysym_to_utf8(keysym)
                        .trim_end_matches('\0')
                        .to_string();
                }
                (!text.is_empty() && !text.chars().all(char::is_control)).then_some(text)
            } else {
                None
            };

            if value == 1 {
                state.update_key(keycode, xkb::KeyDirection::Down);
            }

            if text.is_some() || named.is_some() {
                let _ = tx.send(RawInput::Key {
                    text,
                    named,
                    code: code_for(code),
                    mods,
                    repeat: value == 2,
                });
            }
        }
        0 => {
            if is_modifier(code) {
                if let Some(mcode) = modifier_code(code) {
                    let _ = tx.send(RawInput::Modifier {
                        code: mcode,
                        pressed: false,
                    });
                }
            }
            state.update_key(keycode, xkb::KeyDirection::Up);
        }
        _ => {}
    }
}

/// Physical modifier position for the on-screen keyboard.
fn modifier_code(code: evdev::KeyCode) -> Option<&'static str> {
    use evdev::KeyCode as K;
    Some(match code {
        K::KEY_LEFTCTRL => "ControlLeft",
        K::KEY_RIGHTCTRL => "ControlRight",
        K::KEY_LEFTALT => "AltLeft",
        K::KEY_RIGHTALT => "AltRight",
        K::KEY_LEFTSHIFT => "ShiftLeft",
        K::KEY_RIGHTSHIFT => "ShiftRight",
        K::KEY_LEFTMETA => "MetaLeft",
        K::KEY_RIGHTMETA => "MetaRight",
        _ => return None,
    })
}

/// Physical key → W3C KeyboardEvent.code, for the on-screen keyboard's caps
/// (which are addressed by physical position, so any layout lights up right).
/// One table drives both `code_for` (live events) and `key_labels` (startup
/// layout detection).
const CODE_TABLE: &[(evdev::KeyCode, &str)] = {
    use evdev::KeyCode as K;
    &[
        (K::KEY_A, "KeyA"), (K::KEY_B, "KeyB"), (K::KEY_C, "KeyC"), (K::KEY_D, "KeyD"),
        (K::KEY_E, "KeyE"), (K::KEY_F, "KeyF"), (K::KEY_G, "KeyG"), (K::KEY_H, "KeyH"),
        (K::KEY_I, "KeyI"), (K::KEY_J, "KeyJ"), (K::KEY_K, "KeyK"), (K::KEY_L, "KeyL"),
        (K::KEY_M, "KeyM"), (K::KEY_N, "KeyN"), (K::KEY_O, "KeyO"), (K::KEY_P, "KeyP"),
        (K::KEY_Q, "KeyQ"), (K::KEY_R, "KeyR"), (K::KEY_S, "KeyS"), (K::KEY_T, "KeyT"),
        (K::KEY_U, "KeyU"), (K::KEY_V, "KeyV"), (K::KEY_W, "KeyW"), (K::KEY_X, "KeyX"),
        (K::KEY_Y, "KeyY"), (K::KEY_Z, "KeyZ"),
        (K::KEY_1, "Digit1"), (K::KEY_2, "Digit2"), (K::KEY_3, "Digit3"), (K::KEY_4, "Digit4"),
        (K::KEY_5, "Digit5"), (K::KEY_6, "Digit6"), (K::KEY_7, "Digit7"), (K::KEY_8, "Digit8"),
        (K::KEY_9, "Digit9"), (K::KEY_0, "Digit0"),
        (K::KEY_MINUS, "Minus"), (K::KEY_EQUAL, "Equal"),
        (K::KEY_LEFTBRACE, "BracketLeft"), (K::KEY_RIGHTBRACE, "BracketRight"),
        (K::KEY_BACKSLASH, "Backslash"), (K::KEY_SEMICOLON, "Semicolon"),
        (K::KEY_APOSTROPHE, "Quote"), (K::KEY_GRAVE, "Backquote"),
        (K::KEY_COMMA, "Comma"), (K::KEY_DOT, "Period"), (K::KEY_SLASH, "Slash"),
        (K::KEY_SPACE, "Space"), (K::KEY_ENTER, "Enter"), (K::KEY_TAB, "Tab"),
        (K::KEY_BACKSPACE, "Backspace"), (K::KEY_CAPSLOCK, "CapsLock"), (K::KEY_ESC, "Escape"),
        (K::KEY_UP, "ArrowUp"), (K::KEY_DOWN, "ArrowDown"),
        (K::KEY_LEFT, "ArrowLeft"), (K::KEY_RIGHT, "ArrowRight"),
        (K::KEY_F1, "F1"), (K::KEY_F2, "F2"), (K::KEY_F3, "F3"), (K::KEY_F4, "F4"),
        (K::KEY_F5, "F5"), (K::KEY_F6, "F6"), (K::KEY_F7, "F7"), (K::KEY_F8, "F8"),
        (K::KEY_F9, "F9"), (K::KEY_F10, "F10"), (K::KEY_F11, "F11"), (K::KEY_F12, "F12"),
    ]
};

fn code_for(code: evdev::KeyCode) -> Option<&'static str> {
    CODE_TABLE
        .iter()
        .find(|(key, _)| *key == code)
        .map(|(_, w3c)| *w3c)
}

/// Base (unshifted) character for every keyboard cap in the active layout, keyed
/// by W3C code — so the on-screen keyboard shows the user's real layout (QWERTZ,
/// AZERTY, …) immediately, without waiting for keys to be pressed. Uses the same
/// OS layout (xkbcommon) as live translation.
pub fn key_labels(layout_override: Option<&str>) -> std::collections::HashMap<String, String> {
    let mut labels = std::collections::HashMap::new();
    let Ok(state) = build_xkb_state(layout_override) else {
        return labels;
    };
    for (code, w3c) in CODE_TABLE {
        let keycode = xkb::Keycode::new(code.0 as u32 + 8);
        let text = state.key_get_utf8(keycode);
        if !text.is_empty() && !text.chars().all(char::is_control) {
            labels.insert(w3c.to_string(), text);
        }
    }
    labels
}

/// Compiles an xkb keymap for the active layout.
/// Priority: config override → the compositor's own keymap via the Wayland
/// protocol (authoritative: exactly what the user configured in their desktop
/// settings) → setxkbmap -query (X11) → XKB_DEFAULT_* env → localectl → "us".
fn build_xkb_state(layout_override: Option<&str>) -> Result<xkb::State, String> {
    let context = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);

    let overridden = layout_override
        .map(|l| l.trim().to_lowercase())
        .is_some_and(|l| !l.is_empty() && l != "auto");

    if !overridden && std::env::var("WAYLAND_DISPLAY").is_ok() {
        if let Some(keymap_text) = super::wayland_keymap::fetch() {
            if let Some(keymap) = xkb::Keymap::new_from_string(
                &context,
                keymap_text,
                xkb::KEYMAP_FORMAT_TEXT_V1,
                xkb::KEYMAP_COMPILE_NO_FLAGS,
            ) {
                eprintln!("YAKC: using keyboard layout from the Wayland compositor");
                return Ok(xkb::State::new(&keymap));
            }
        }
        eprintln!("YAKC: could not fetch the compositor keymap; falling back");
    }

    let (layout, variant) = resolve_layout(layout_override);
    eprintln!("YAKC: using keyboard layout '{layout}' (variant '{variant}')");
    let keymap = xkb::Keymap::new_from_names(
        &context,
        "",
        "",
        &layout,
        &variant,
        None,
        xkb::KEYMAP_COMPILE_NO_FLAGS,
    )
    .ok_or_else(|| format!("failed to compile keymap for layout '{layout}'"))?;
    Ok(xkb::State::new(&keymap))
}

fn resolve_layout(layout_override: Option<&str>) -> (String, String) {
    if let Some(over) = layout_override {
        let over = over.trim().to_lowercase();
        if !over.is_empty() && over != "auto" {
            // Legacy Electron-era config values.
            let mapped = match over.as_str() {
                "english" => "us",
                "german" => "de",
                other => other,
            };
            return (mapped.to_string(), String::new());
        }
    }

    if std::env::var("DISPLAY").is_ok() {
        if let Some(result) = layout_from_setxkbmap() {
            return result;
        }
    }

    if let Ok(layout) = std::env::var("XKB_DEFAULT_LAYOUT") {
        if !layout.is_empty() {
            let variant = std::env::var("XKB_DEFAULT_VARIANT").unwrap_or_default();
            return (layout, variant);
        }
    }

    if let Some(result) = layout_from_localectl() {
        return result;
    }

    ("us".to_string(), String::new())
}

fn layout_from_setxkbmap() -> Option<(String, String)> {
    let output = std::process::Command::new("setxkbmap")
        .arg("-query")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    parse_kv_layout(&String::from_utf8_lossy(&output.stdout), "layout:", "variant:")
}

fn layout_from_localectl() -> Option<(String, String)> {
    let output = std::process::Command::new("localectl")
        .arg("status")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    parse_kv_layout(
        &String::from_utf8_lossy(&output.stdout),
        "X11 Layout:",
        "X11 Variant:",
    )
}

fn parse_kv_layout(text: &str, layout_key: &str, variant_key: &str) -> Option<(String, String)> {
    let mut layout = None;
    let mut variant = String::new();
    for line in text.lines() {
        let line = line.trim();
        if let Some(value) = line.strip_prefix(layout_key) {
            layout = Some(value.trim().to_string());
        } else if let Some(value) = line.strip_prefix(variant_key) {
            variant = value.trim().to_string();
        }
    }
    let layout = layout.filter(|l| !l.is_empty() && l != "(unset)")?;
    (layout != "n/a").then_some((layout, variant))
}

/// Forwards mouse movement (REL_X/REL_Y) and scroll (REL_WHEEL/REL_HWHEEL).
/// Emitted per axis; the overlay accumulates them, so a separate X and Y event
/// is fine. Works on X11 and Wayland alike (we read the device directly).
fn handle_rel_event(axis: evdev::RelativeAxisCode, value: i32, tx: &Sender<RawInput>) {
    let input = match axis {
        evdev::RelativeAxisCode::REL_X => RawInput::MouseMotion {
            dx: value as f64,
            dy: 0.0,
        },
        evdev::RelativeAxisCode::REL_Y => RawInput::MouseMotion {
            dx: 0.0,
            dy: value as f64,
        },
        // REL_WHEEL is positive-up; keep that convention (dy>0 = up).
        evdev::RelativeAxisCode::REL_WHEEL => RawInput::Scroll {
            dx: 0.0,
            dy: value as f64,
        },
        evdev::RelativeAxisCode::REL_HWHEEL => RawInput::Scroll {
            dx: value as f64,
            dy: 0.0,
        },
        _ => return,
    };
    let _ = tx.send(input);
}

fn mouse_button(code: evdev::KeyCode) -> Option<u8> {
    match code {
        evdev::KeyCode::BTN_LEFT => Some(1),
        evdev::KeyCode::BTN_RIGHT => Some(2),
        evdev::KeyCode::BTN_MIDDLE => Some(3),
        evdev::KeyCode::BTN_SIDE => Some(4),
        evdev::KeyCode::BTN_EXTRA => Some(5),
        _ => None,
    }
}

fn is_modifier(code: evdev::KeyCode) -> bool {
    matches!(
        code,
        evdev::KeyCode::KEY_LEFTCTRL
            | evdev::KeyCode::KEY_RIGHTCTRL
            | evdev::KeyCode::KEY_LEFTALT
            | evdev::KeyCode::KEY_RIGHTALT
            | evdev::KeyCode::KEY_LEFTSHIFT
            | evdev::KeyCode::KEY_RIGHTSHIFT
            | evdev::KeyCode::KEY_LEFTMETA
            | evdev::KeyCode::KEY_RIGHTMETA
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Translate an evdev keycode with an explicit layout, optionally with
    /// shift held, using the same xkb pipeline as the live translator thread.
    fn translate(layout: &str, code: evdev::KeyCode, shift: bool) -> String {
        let mut state = build_xkb_state(Some(layout)).expect("keymap compiles");
        if shift {
            let shift_code = xkb::Keycode::new(evdev::KeyCode::KEY_LEFTSHIFT.0 as u32 + 8);
            state.update_key(shift_code, xkb::KeyDirection::Down);
        }
        state.key_get_utf8(xkb::Keycode::new(code.0 as u32 + 8))
    }

    #[test]
    fn us_layout_translates_qwerty() {
        assert_eq!(translate("us", evdev::KeyCode::KEY_Y, false), "y");
        assert_eq!(translate("us", evdev::KeyCode::KEY_1, true), "!");
    }

    #[test]
    fn german_layout_swaps_y_and_z_and_has_umlauts() {
        // Physical QWERTY "Y" key produces "z" on QWERTZ…
        assert_eq!(translate("de", evdev::KeyCode::KEY_Y, false), "z");
        assert_eq!(translate("de", evdev::KeyCode::KEY_Z, false), "y");
        // …and the key right of "P" is "ü".
        assert_eq!(translate("de", evdev::KeyCode::KEY_LEFTBRACE, false), "ü");
        assert_eq!(translate("de", evdev::KeyCode::KEY_LEFTBRACE, true), "Ü");
    }

    #[test]
    fn turkish_layout_produces_dotless_i() {
        assert_eq!(translate("tr", evdev::KeyCode::KEY_I, false), "ı");
        assert_eq!(translate("tr", evdev::KeyCode::KEY_I, true), "I");
    }

    #[test]
    fn legacy_layout_names_map_to_xkb() {
        assert_eq!(resolve_layout(Some("english")).0, "us");
        assert_eq!(resolve_layout(Some("german")).0, "de");
        assert_eq!(resolve_layout(Some("fr")).0, "fr");
    }
}

/// Maps evdev non-printable keys to the shared named-key ids in keymap.rs.
fn named_for(code: evdev::KeyCode) -> Option<&'static str> {
    Some(match code {
        evdev::KeyCode::KEY_BACKSPACE => "backspace",
        evdev::KeyCode::KEY_ENTER => "enter",
        evdev::KeyCode::KEY_KPENTER => "numpadenter",
        evdev::KeyCode::KEY_TAB => "tab",
        evdev::KeyCode::KEY_SPACE => "space",
        evdev::KeyCode::KEY_ESC => "escape",
        evdev::KeyCode::KEY_DELETE => "delete",
        evdev::KeyCode::KEY_INSERT => "insert",
        evdev::KeyCode::KEY_CAPSLOCK => "capslock",
        evdev::KeyCode::KEY_NUMLOCK => "numlock",
        evdev::KeyCode::KEY_SCROLLLOCK => "scrolllock",
        evdev::KeyCode::KEY_PAUSE => "pause",
        evdev::KeyCode::KEY_SYSRQ | evdev::KeyCode::KEY_PRINT => "printscreen",
        evdev::KeyCode::KEY_COMPOSE => "contextmenu",
        evdev::KeyCode::KEY_UP => "arrowup",
        evdev::KeyCode::KEY_DOWN => "arrowdown",
        evdev::KeyCode::KEY_LEFT => "arrowleft",
        evdev::KeyCode::KEY_RIGHT => "arrowright",
        evdev::KeyCode::KEY_HOME => "home",
        evdev::KeyCode::KEY_END => "end",
        evdev::KeyCode::KEY_PAGEUP => "pageup",
        evdev::KeyCode::KEY_PAGEDOWN => "pagedown",
        evdev::KeyCode::KEY_F1 => "f1",
        evdev::KeyCode::KEY_F2 => "f2",
        evdev::KeyCode::KEY_F3 => "f3",
        evdev::KeyCode::KEY_F4 => "f4",
        evdev::KeyCode::KEY_F5 => "f5",
        evdev::KeyCode::KEY_F6 => "f6",
        evdev::KeyCode::KEY_F7 => "f7",
        evdev::KeyCode::KEY_F8 => "f8",
        evdev::KeyCode::KEY_F9 => "f9",
        evdev::KeyCode::KEY_F10 => "f10",
        evdev::KeyCode::KEY_F11 => "f11",
        evdev::KeyCode::KEY_F12 => "f12",
        evdev::KeyCode::KEY_KPSLASH => "numpaddivide",
        evdev::KeyCode::KEY_KPASTERISK => "numpadmultiply",
        evdev::KeyCode::KEY_KPMINUS => "numpadsubtract",
        evdev::KeyCode::KEY_KPPLUS => "numpadadd",
        evdev::KeyCode::KEY_KPDOT => "numpaddecimal",
        evdev::KeyCode::KEY_KP0 => "numpad0",
        evdev::KeyCode::KEY_KP1 => "numpad1",
        evdev::KeyCode::KEY_KP2 => "numpad2",
        evdev::KeyCode::KEY_KP3 => "numpad3",
        evdev::KeyCode::KEY_KP4 => "numpad4",
        evdev::KeyCode::KEY_KP5 => "numpad5",
        evdev::KeyCode::KEY_KP6 => "numpad6",
        evdev::KeyCode::KEY_KP7 => "numpad7",
        evdev::KeyCode::KEY_KP8 => "numpad8",
        evdev::KeyCode::KEY_KP9 => "numpad9",
        _ => return None,
    })
}
