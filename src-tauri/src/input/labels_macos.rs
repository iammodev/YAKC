//! macOS: base (unshifted) label for each physical keyboard cap in the active
//! layout, via `UCKeyTranslate` on the current `TISInputSource` — the on-screen
//! keyboard then shows the user's real layout immediately. Parallels the Linux
//! `xkbcommon` path.

use std::collections::HashMap;
use std::os::raw::c_void;

use core_foundation::base::{CFRelease, TCFType};
use core_foundation::data::{CFData, CFDataRef};
use core_foundation::string::CFStringRef;

// Carbon / HIToolbox APIs for reading the active Unicode keyboard layout.
#[link(name = "Carbon", kind = "framework")]
extern "C" {
    fn TISCopyCurrentKeyboardLayoutInputSource() -> *mut c_void;
    fn TISGetInputSourceProperty(input_source: *mut c_void, key: CFStringRef) -> *mut c_void;
    static kTISPropertyUnicodeKeyLayoutData: CFStringRef;
    fn LMGetKbdType() -> u8;
    #[allow(clippy::too_many_arguments)]
    fn UCKeyTranslate(
        key_layout_ptr: *const u8,
        virtual_key_code: u16,
        key_action: u16,
        modifier_key_state: u32,
        keyboard_type: u32,
        key_translate_options: u32,
        dead_key_state: *mut u32,
        max_string_length: u32,
        actual_string_length: *mut u32,
        unicode_string: *mut u16,
    ) -> i32;
}

const K_UC_KEY_ACTION_DISPLAY: u16 = 3;

/// W3C KeyboardEvent.code → macOS ANSI virtual keycode (kVK_ANSI_*), printable only.
const KEYCODES: &[(&str, u16)] = &[
    ("KeyA", 0x00), ("KeyS", 0x01), ("KeyD", 0x02), ("KeyF", 0x03), ("KeyH", 0x04),
    ("KeyG", 0x05), ("KeyZ", 0x06), ("KeyX", 0x07), ("KeyC", 0x08), ("KeyV", 0x09),
    ("KeyB", 0x0B), ("KeyQ", 0x0C), ("KeyW", 0x0D), ("KeyE", 0x0E), ("KeyR", 0x0F),
    ("KeyY", 0x10), ("KeyT", 0x11),
    ("Digit1", 0x12), ("Digit2", 0x13), ("Digit3", 0x14), ("Digit4", 0x15),
    ("Digit6", 0x16), ("Digit5", 0x17), ("Equal", 0x18), ("Digit9", 0x19),
    ("Digit7", 0x1A), ("Minus", 0x1B), ("Digit8", 0x1C), ("Digit0", 0x1D),
    ("BracketRight", 0x1E), ("KeyO", 0x1F), ("KeyU", 0x20), ("BracketLeft", 0x21),
    ("KeyI", 0x22), ("KeyP", 0x23), ("KeyL", 0x25), ("KeyJ", 0x26), ("Quote", 0x27),
    ("KeyK", 0x28), ("Semicolon", 0x29), ("Backslash", 0x2A), ("Comma", 0x2B),
    ("Slash", 0x2C), ("KeyN", 0x2D), ("KeyM", 0x2E), ("Period", 0x2F), ("Backquote", 0x32),
];

pub fn key_labels(_layout_override: Option<&str>) -> HashMap<String, String> {
    let mut labels = HashMap::new();
    unsafe {
        let source = TISCopyCurrentKeyboardLayoutInputSource();
        if source.is_null() {
            return labels;
        }
        let data_ref = TISGetInputSourceProperty(source, kTISPropertyUnicodeKeyLayoutData);
        if data_ref.is_null() {
            CFRelease(source);
            return labels;
        }
        // +0 (get rule) → wrap so it's retained for the duration and released on drop.
        let layout_data: CFData = CFData::wrap_under_get_rule(data_ref as CFDataRef);
        let layout_ptr = layout_data.bytes().as_ptr(); // UCKeyboardLayout*
        let kbd_type = LMGetKbdType() as u32;

        for (w3c, vk) in KEYCODES {
            let mut dead_key_state: u32 = 0;
            let mut buf = [0u16; 8];
            let mut len: u32 = 0;
            let status = UCKeyTranslate(
                layout_ptr,
                *vk,
                K_UC_KEY_ACTION_DISPLAY,
                0, // no modifiers → base character
                kbd_type,
                0,
                &mut dead_key_state,
                buf.len() as u32,
                &mut len,
                buf.as_mut_ptr(),
            );
            if status == 0 && len > 0 {
                let text = String::from_utf16_lossy(&buf[..len as usize]);
                if !text.is_empty() && !text.chars().all(char::is_control) {
                    labels.insert(w3c.to_string(), text);
                }
            }
        }
        CFRelease(source);
    }
    labels
}
