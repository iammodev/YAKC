//! Windows: base (unshifted) label for each physical keyboard cap in the active
//! layout, via `ToUnicodeEx` — the on-screen keyboard then shows the user's real
//! layout immediately. Parallels the Linux `xkbcommon` path.

use std::collections::HashMap;

use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyboardLayout, MapVirtualKeyExW, ToUnicodeEx, MAPVK_VSC_TO_VK_EX,
};

/// W3C KeyboardEvent.code → PC/AT set-1 scan code (physical), printable keys only.
const SCANCODES: &[(&str, u16)] = &[
    ("Digit1", 0x02), ("Digit2", 0x03), ("Digit3", 0x04), ("Digit4", 0x05),
    ("Digit5", 0x06), ("Digit6", 0x07), ("Digit7", 0x08), ("Digit8", 0x09),
    ("Digit9", 0x0A), ("Digit0", 0x0B), ("Minus", 0x0C), ("Equal", 0x0D),
    ("KeyQ", 0x10), ("KeyW", 0x11), ("KeyE", 0x12), ("KeyR", 0x13), ("KeyT", 0x14),
    ("KeyY", 0x15), ("KeyU", 0x16), ("KeyI", 0x17), ("KeyO", 0x18), ("KeyP", 0x19),
    ("BracketLeft", 0x1A), ("BracketRight", 0x1B),
    ("KeyA", 0x1E), ("KeyS", 0x1F), ("KeyD", 0x20), ("KeyF", 0x21), ("KeyG", 0x22),
    ("KeyH", 0x23), ("KeyJ", 0x24), ("KeyK", 0x25), ("KeyL", 0x26),
    ("Semicolon", 0x27), ("Quote", 0x28), ("Backquote", 0x29), ("Backslash", 0x2B),
    ("KeyZ", 0x2C), ("KeyX", 0x2D), ("KeyC", 0x2E), ("KeyV", 0x2F), ("KeyB", 0x30),
    ("KeyN", 0x31), ("KeyM", 0x32), ("Comma", 0x33), ("Period", 0x34), ("Slash", 0x35),
];

pub fn key_labels(_layout_override: Option<&str>) -> HashMap<String, String> {
    let mut labels = HashMap::new();
    unsafe {
        let hkl = GetKeyboardLayout(0);
        let key_state = [0u8; 256]; // no modifiers held → base character
        for (w3c, scancode) in SCANCODES {
            let vk = MapVirtualKeyExW(*scancode as u32, MAPVK_VSC_TO_VK_EX, hkl);
            if vk == 0 {
                continue;
            }
            let mut buf = [0u16; 8];
            let n = ToUnicodeEx(vk, *scancode as u32, &key_state, &mut buf, 0, hkl);
            if n > 0 {
                let text = String::from_utf16_lossy(&buf[..n as usize]);
                if !text.is_empty() && !text.chars().all(char::is_control) {
                    labels.insert(w3c.to_string(), text);
                }
            }
        }
    }
    labels
}
