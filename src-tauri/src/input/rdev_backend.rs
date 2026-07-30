//! Global input hook for Windows and macOS via rdev (WH_KEYBOARD_LL /
//! CGEventTap). Printable keys arrive with `event.name` already translated by
//! the OS for the active keyboard layout, so any language works.

use std::sync::mpsc::Sender;

use rdev::{Button, Event, EventType, Key};

use super::{BackendIssue, Mods, RawInput};

pub fn spawn_listener(
    tx: Sender<RawInput>,
    on_issue: impl FnOnce(BackendIssue) + Send + 'static,
) {
    std::thread::spawn(move || {
        let mut mods = Mods::default();
        // Keys currently held down: a KeyPress for one of these is an
        // auto-repeat (the OS hooks deliver repeats as fresh KeyPress events).
        let mut held: std::collections::HashSet<Key> = std::collections::HashSet::new();
        // rdev reports absolute cursor positions; we forward relative deltas so
        // the movement widget behaves the same as the Linux (evdev) path.
        let mut last_pos: Option<(f64, f64)> = None;

        let callback = move |event: Event| {
            match event.event_type {
                EventType::KeyPress(key) => {
                    if update_modifier(&mut mods, key, true) {
                        // Forward once (the OS delivers held modifiers as repeats).
                        if held.insert(key) {
                            if let Some(mcode) = modifier_code(key) {
                                let _ = tx.send(RawInput::Modifier {
                                    code: mcode,
                                    pressed: true,
                                });
                            }
                        }
                        return;
                    }
                    let repeat = !held.insert(key);
                    let named = named_for(key);
                    let text = if named.is_none() {
                        event
                            .name
                            .clone()
                            .filter(|s| !s.is_empty() && !s.chars().all(char::is_control))
                    } else {
                        None
                    };
                    if text.is_none() && named.is_none() {
                        return;
                    }
                    let _ = tx.send(RawInput::Key {
                        text,
                        named,
                        code: code_for(key),
                        mods,
                        repeat,
                    });
                }
                EventType::KeyRelease(key) => {
                    held.remove(&key);
                    if update_modifier(&mut mods, key, false) {
                        if let Some(mcode) = modifier_code(key) {
                            let _ = tx.send(RawInput::Modifier {
                                code: mcode,
                                pressed: false,
                            });
                        }
                    }
                }
                EventType::ButtonPress(button) => {
                    let button = match button {
                        Button::Left => 1,
                        Button::Right => 2,
                        Button::Middle => 3,
                        Button::Unknown(code) => code,
                    };
                    let _ = tx.send(RawInput::MouseButton { button });
                }
                EventType::MouseMove { x, y } => {
                    if let Some((px, py)) = last_pos {
                        let (dx, dy) = (x - px, y - py);
                        if dx != 0.0 || dy != 0.0 {
                            let _ = tx.send(RawInput::MouseMotion { dx, dy });
                        }
                    }
                    last_pos = Some((x, y));
                }
                EventType::Wheel { delta_x, delta_y } => {
                    let _ = tx.send(RawInput::Scroll {
                        dx: delta_x as f64,
                        dy: delta_y as f64,
                    });
                }
                _ => {}
            }
        };

        if let Err(err) = rdev::listen(callback) {
            let issue = if cfg!(target_os = "macos") {
                BackendIssue::Accessibility(format!(
                    "Cannot capture input ({err:?}).\n\nYAKC needs the Accessibility permission:\n\
                     System Settings → Privacy & Security → Accessibility → enable YAKC,\n\
                     then restart the app."
                ))
            } else {
                BackendIssue::Other(format!("Cannot capture input: {err:?}"))
            };
            on_issue(issue);
        }
    });
}

/// Tracks modifier keys; returns true when the key was a modifier.
fn update_modifier(mods: &mut Mods, key: Key, pressed: bool) -> bool {
    match key {
        Key::ControlLeft | Key::ControlRight => mods.ctrl = pressed,
        Key::Alt | Key::AltGr => mods.alt = pressed,
        Key::ShiftLeft | Key::ShiftRight => mods.shift = pressed,
        Key::MetaLeft | Key::MetaRight => mods.meta = pressed,
        _ => return false,
    }
    true
}

/// Physical modifier position for the on-screen keyboard.
fn modifier_code(key: Key) -> Option<&'static str> {
    Some(match key {
        Key::ControlLeft => "ControlLeft",
        Key::ControlRight => "ControlRight",
        Key::Alt => "AltLeft",
        Key::AltGr => "AltRight",
        Key::ShiftLeft => "ShiftLeft",
        Key::ShiftRight => "ShiftRight",
        Key::MetaLeft => "MetaLeft",
        Key::MetaRight => "MetaRight",
        _ => return None,
    })
}

/// Physical key position (W3C KeyboardEvent.code style) for the on-screen
/// keyboard. Layout-independent, so the correct cap lights up on any layout.
fn code_for(key: Key) -> Option<&'static str> {
    Some(match key {
        Key::KeyA => "KeyA", Key::KeyB => "KeyB", Key::KeyC => "KeyC", Key::KeyD => "KeyD",
        Key::KeyE => "KeyE", Key::KeyF => "KeyF", Key::KeyG => "KeyG", Key::KeyH => "KeyH",
        Key::KeyI => "KeyI", Key::KeyJ => "KeyJ", Key::KeyK => "KeyK", Key::KeyL => "KeyL",
        Key::KeyM => "KeyM", Key::KeyN => "KeyN", Key::KeyO => "KeyO", Key::KeyP => "KeyP",
        Key::KeyQ => "KeyQ", Key::KeyR => "KeyR", Key::KeyS => "KeyS", Key::KeyT => "KeyT",
        Key::KeyU => "KeyU", Key::KeyV => "KeyV", Key::KeyW => "KeyW", Key::KeyX => "KeyX",
        Key::KeyY => "KeyY", Key::KeyZ => "KeyZ",
        Key::Num1 => "Digit1", Key::Num2 => "Digit2", Key::Num3 => "Digit3", Key::Num4 => "Digit4",
        Key::Num5 => "Digit5", Key::Num6 => "Digit6", Key::Num7 => "Digit7", Key::Num8 => "Digit8",
        Key::Num9 => "Digit9", Key::Num0 => "Digit0",
        Key::Minus => "Minus", Key::Equal => "Equal",
        Key::LeftBracket => "BracketLeft", Key::RightBracket => "BracketRight",
        Key::BackSlash => "Backslash", Key::SemiColon => "Semicolon",
        Key::Quote => "Quote", Key::BackQuote => "Backquote",
        Key::Comma => "Comma", Key::Dot => "Period", Key::Slash => "Slash",
        Key::Space => "Space", Key::Return => "Enter", Key::Tab => "Tab",
        Key::Backspace => "Backspace", Key::CapsLock => "CapsLock", Key::Escape => "Escape",
        Key::UpArrow => "ArrowUp", Key::DownArrow => "ArrowDown",
        Key::LeftArrow => "ArrowLeft", Key::RightArrow => "ArrowRight",
        Key::F1 => "F1", Key::F2 => "F2", Key::F3 => "F3", Key::F4 => "F4",
        Key::F5 => "F5", Key::F6 => "F6", Key::F7 => "F7", Key::F8 => "F8",
        Key::F9 => "F9", Key::F10 => "F10", Key::F11 => "F11", Key::F12 => "F12",
        _ => return None,
    })
}

/// Maps rdev non-printable keys to the shared named-key ids in keymap.rs.
fn named_for(key: Key) -> Option<&'static str> {
    Some(match key {
        Key::Backspace => "backspace",
        Key::Return => "enter",
        Key::KpReturn => "numpadenter",
        Key::Tab => "tab",
        Key::Space => "space",
        Key::Escape => "escape",
        Key::Delete => "delete",
        Key::Insert => "insert",
        Key::CapsLock => "capslock",
        Key::NumLock => "numlock",
        Key::ScrollLock => "scrolllock",
        Key::Pause => "pause",
        Key::PrintScreen => "printscreen",
        Key::UpArrow => "arrowup",
        Key::DownArrow => "arrowdown",
        Key::LeftArrow => "arrowleft",
        Key::RightArrow => "arrowright",
        Key::Home => "home",
        Key::End => "end",
        Key::PageUp => "pageup",
        Key::PageDown => "pagedown",
        Key::F1 => "f1",
        Key::F2 => "f2",
        Key::F3 => "f3",
        Key::F4 => "f4",
        Key::F5 => "f5",
        Key::F6 => "f6",
        Key::F7 => "f7",
        Key::F8 => "f8",
        Key::F9 => "f9",
        Key::F10 => "f10",
        Key::F11 => "f11",
        Key::F12 => "f12",
        Key::KpDivide => "numpaddivide",
        Key::KpMultiply => "numpadmultiply",
        Key::KpMinus => "numpadsubtract",
        Key::KpPlus => "numpadadd",
        Key::KpDelete => "numpaddecimal",
        Key::Kp0 => "numpad0",
        Key::Kp1 => "numpad1",
        Key::Kp2 => "numpad2",
        Key::Kp3 => "numpad3",
        Key::Kp4 => "numpad4",
        Key::Kp5 => "numpad5",
        Key::Kp6 => "numpad6",
        Key::Kp7 => "numpad7",
        Key::Kp8 => "numpad8",
        Key::Kp9 => "numpad9",
        _ => return None,
    })
}
