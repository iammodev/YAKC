use serde::Serialize;

use crate::config::Config;
use crate::input::Mods;

/// Instruction for the overlay popup, emitted as the "click-event" payload.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(tag = "op", rename_all = "lowercase")]
pub enum PopupOp {
    /// Add a token (character or key label) to the current popup.
    Append { text: String },
    /// Remove the last token (text mode Backspace).
    Delete,
    /// The last token is being held down; the overlay renders "tok (xN)".
    Repeat,
}

/// Non-printable keys: internal id (lowercase), display name, optional symbol
/// used when `textToSymbols` is enabled. Ids are shared by all input backends.
/// Ported from the Electron charToUnicode.js map.
const NAMED_KEYS: &[(&str, &str, Option<&str>)] = &[
    ("capslock", "CapsLock", Some("⇪")),
    ("backspace", "Backspace", Some("⌫")),
    ("enter", "Enter", Some("↵")),
    ("space", "Space", Some("␣")),
    ("tab", "Tab", Some("↹")),
    ("delete", "Delete", Some("DEL")),
    ("arrowleft", "ArrowLeft", Some("←")),
    ("arrowup", "ArrowUp", Some("↑")),
    ("arrowright", "ArrowRight", Some("→")),
    ("arrowdown", "ArrowDown", Some("↓")),
    ("escape", "Escape", Some("ESC")),
    ("insert", "Insert", Some("INS")),
    ("pageup", "PageUp", Some("PgUp")),
    ("pagedown", "PageDown", Some("PgDn")),
    ("home", "Home", Some("HOME")),
    ("end", "End", Some("END")),
    ("numlock", "NumLock", Some("NUM")),
    ("scrolllock", "ScrollLock", Some("⇳")),
    ("pause", "Pause", Some("PAUSE")),
    ("printscreen", "PrintScreen", Some("PRINT")),
    ("contextmenu", "Menu", None),
    ("numpaddivide", "Numpad /", Some("/")),
    ("numpadmultiply", "Numpad *", Some("*")),
    ("numpadsubtract", "Numpad -", Some("-")),
    ("numpadadd", "Numpad +", Some("+")),
    ("numpaddecimal", "Numpad .", Some(".")),
    ("numpadenter", "Numpad Enter", Some("↵")),
    ("numpad0", "Numpad 0", Some("0")),
    ("numpad1", "Numpad 1", Some("1")),
    ("numpad2", "Numpad 2", Some("2")),
    ("numpad3", "Numpad 3", Some("3")),
    ("numpad4", "Numpad 4", Some("4")),
    ("numpad5", "Numpad 5", Some("5")),
    ("numpad6", "Numpad 6", Some("6")),
    ("numpad7", "Numpad 7", Some("7")),
    ("numpad8", "Numpad 8", Some("8")),
    ("numpad9", "Numpad 9", Some("9")),
    ("f1", "F1", None),
    ("f2", "F2", None),
    ("f3", "F3", None),
    ("f4", "F4", None),
    ("f5", "F5", None),
    ("f6", "F6", None),
    ("f7", "F7", None),
    ("f8", "F8", None),
    ("f9", "F9", None),
    ("f10", "F10", None),
    ("f11", "F11", None),
    ("f12", "F12", None),
];

fn named_key(id: &str) -> Option<&'static (&'static str, &'static str, Option<&'static str>)> {
    NAMED_KEYS.iter().find(|(key, _, _)| *key == id)
}

/// Turns a key press into a popup instruction, or None when nothing should
/// happen, honoring the configured display mode.
///
/// `text` is the OS-translated character(s) for printable keys (already correct
/// for the active keyboard layout and shift state on every platform).
/// `named` is the backend-normalized id of a non-printable key ("backspace", …).
pub fn key_op(
    text: Option<&str>,
    named: Option<&str>,
    mods: &Mods,
    repeat: bool,
    config: &Config,
) -> Option<PopupOp> {
    if config.is_raw_mode() {
        let label = format_key(text, named, mods, config)?;
        return Some(if repeat {
            PopupOp::Repeat
        } else {
            PopupOp::Append { text: label }
        });
    }

    // Text mode: behave like a text editor — only what typing produces.
    if mods.ctrl || mods.alt || mods.meta {
        return None; // shortcuts don't produce text
    }
    if let Some(id) = named {
        return match id {
            "backspace" => Some(PopupOp::Delete), // repeats keep deleting
            "space" => {
                let s = if config.show_space_as_unicode { "␣" } else { " " };
                text_op(s.to_string(), repeat)
            }
            "enter" | "numpadenter" => text_op("\n".to_string(), repeat),
            "tab" => text_op("\t".to_string(), repeat),
            id if id.starts_with("numpad") && id.len() == 7 => {
                // numpad digits type digits
                text_op(id[6..].to_string(), repeat)
            }
            "numpaddivide" => text_op("/".to_string(), repeat),
            "numpadmultiply" => text_op("*".to_string(), repeat),
            "numpadsubtract" => text_op("-".to_string(), repeat),
            "numpadadd" => text_op("+".to_string(), repeat),
            "numpaddecimal" => text_op(".".to_string(), repeat),
            _ => None, // arrows, F-keys, Esc, … produce no text
        };
    }
    let t = text?;
    if t.is_empty() || t.chars().all(char::is_control) {
        return None;
    }
    text_op(t.to_string(), repeat)
}

fn text_op(text: String, repeat: bool) -> Option<PopupOp> {
    Some(if repeat {
        PopupOp::Repeat
    } else {
        PopupOp::Append { text }
    })
}

/// Builds the raw-mode popup label for a key press, or None when nothing
/// should be shown.
pub fn format_key(
    text: Option<&str>,
    named: Option<&str>,
    mods: &Mods,
    config: &Config,
) -> Option<String> {
    let base: String = if let Some(id) = named {
        if id == "space" {
            if config.show_space_as_unicode {
                "␣".to_string()
            } else {
                " ".to_string()
            }
        } else {
            let (_, display, symbol) = named_key(id)?;
            if config.text_to_symbols {
                symbol.unwrap_or(display).to_string()
            } else {
                (*display).to_string()
            }
        }
    } else {
        let t = text?;
        if t.is_empty() || t.chars().all(|c| c.is_control()) {
            return None;
        }
        t.to_string()
    };

    let has_combo_mods = mods.ctrl || mods.alt || mods.meta;

    if config.only_keys_with_modifiers && !has_combo_mods {
        return None;
    }

    if has_combo_mods {
        let mut parts: Vec<&str> = Vec::new();
        if mods.ctrl {
            parts.push("CTRL");
        }
        if mods.alt {
            parts.push("ALT");
        }
        if mods.shift {
            parts.push("SHIFT");
        }
        if mods.meta {
            parts.push("META");
        }
        Some(format!(
            " {} + {} ",
            parts.join(" + "),
            base.trim().to_uppercase()
        ))
    } else {
        Some(base)
    }
}

/// Builds the popup label for a mouse click, matching the Electron format.
pub fn format_mouse(button: u8, coords: Option<(i32, i32)>, config: &Config) -> String {
    if config.show_mouse_coordinates {
        if let Some((x, y)) = coords {
            return format!(" MOUSE{button} X: {x} Y: {y} ");
        }
    }
    format!(" MOUSE{button} ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mods(ctrl: bool, alt: bool, shift: bool, meta: bool) -> Mods {
        Mods {
            ctrl,
            alt,
            shift,
            meta,
        }
    }

    #[test]
    fn plain_character_passes_through() {
        let config = Config::default();
        let label = format_key(Some("é"), None, &mods(false, false, false, false), &config);
        assert_eq!(label.as_deref(), Some("é"));
    }

    #[test]
    fn shifted_character_comes_from_os_untouched() {
        // The OS already translated Shift+1 to "!" (or layout equivalent).
        let config = Config::default();
        let label = format_key(Some("!"), None, &mods(false, false, true, false), &config);
        assert_eq!(label.as_deref(), Some("!"));
    }

    #[test]
    fn modifier_combo_is_formatted() {
        let config = Config::default();
        let label = format_key(Some("h"), None, &mods(true, true, false, false), &config);
        assert_eq!(label.as_deref(), Some(" CTRL + ALT + H "));
    }

    #[test]
    fn shift_is_listed_only_inside_combos() {
        let config = Config::default();
        let label = format_key(Some("A"), None, &mods(true, false, true, false), &config);
        assert_eq!(label.as_deref(), Some(" CTRL + SHIFT + A "));
    }

    #[test]
    fn only_keys_with_modifiers_filters_plain_keys() {
        let config = Config {
            only_keys_with_modifiers: true,
            ..Config::default()
        };
        assert_eq!(
            format_key(Some("a"), None, &mods(false, false, false, false), &config),
            None
        );
        assert!(
            format_key(Some("a"), None, &mods(true, false, false, false), &config).is_some()
        );
    }

    #[test]
    fn named_keys_respect_text_to_symbols() {
        let with_symbols = Config::default(); // textToSymbols: true
        let label = format_key(
            None,
            Some("backspace"),
            &mods(false, false, false, false),
            &with_symbols,
        );
        assert_eq!(label.as_deref(), Some("⌫"));

        let without_symbols = Config {
            text_to_symbols: false,
            ..Config::default()
        };
        let label = format_key(
            None,
            Some("backspace"),
            &mods(false, false, false, false),
            &without_symbols,
        );
        assert_eq!(label.as_deref(), Some("Backspace"));
    }

    #[test]
    fn space_follows_show_space_as_unicode() {
        let config = Config {
            show_space_as_unicode: true,
            ..Config::default()
        };
        let label = format_key(None, Some("space"), &mods(false, false, false, false), &config);
        assert_eq!(label.as_deref(), Some("␣"));

        let config = Config {
            show_space_as_unicode: false,
            ..Config::default()
        };
        let label = format_key(None, Some("space"), &mods(false, false, false, false), &config);
        assert_eq!(label.as_deref(), Some(" "));
    }

    #[test]
    fn control_characters_are_dropped() {
        let config = Config::default();
        assert_eq!(
            format_key(Some("\u{1}"), None, &mods(false, false, false, false), &config),
            None
        );
        assert_eq!(
            format_key(Some(""), None, &mods(false, false, false, false), &config),
            None
        );
    }

    fn text_cfg() -> Config {
        Config::default() // displayMode defaults to "text"
    }

    fn raw_cfg() -> Config {
        Config {
            display_mode: "raw".into(),
            ..Config::default()
        }
    }

    #[test]
    fn text_mode_appends_typed_characters() {
        let op = key_op(Some("ü"), None, &mods(false, false, false, false), false, &text_cfg());
        assert_eq!(op, Some(PopupOp::Append { text: "ü".into() }));
    }

    #[test]
    fn text_mode_backspace_deletes() {
        let op = key_op(None, Some("backspace"), &mods(false, false, false, false), false, &text_cfg());
        assert_eq!(op, Some(PopupOp::Delete));
        // Held backspace keeps deleting.
        let op = key_op(None, Some("backspace"), &mods(false, false, false, false), true, &text_cfg());
        assert_eq!(op, Some(PopupOp::Delete));
    }

    #[test]
    fn text_mode_hides_shortcuts_and_navigation() {
        let cfg = text_cfg();
        assert_eq!(key_op(Some("c"), None, &mods(true, false, false, false), false, &cfg), None);
        assert_eq!(key_op(None, Some("arrowleft"), &mods(false, false, false, false), false, &cfg), None);
        assert_eq!(key_op(None, Some("escape"), &mods(false, false, false, false), false, &cfg), None);
        assert_eq!(key_op(None, Some("f5"), &mods(false, false, false, false), false, &cfg), None);
    }

    #[test]
    fn text_mode_maps_whitespace_and_numpad() {
        let cfg = text_cfg();
        assert_eq!(
            key_op(None, Some("space"), &mods(false, false, false, false), false, &cfg),
            Some(PopupOp::Append { text: " ".into() })
        );
        assert_eq!(
            key_op(None, Some("enter"), &mods(false, false, false, false), false, &cfg),
            Some(PopupOp::Append { text: "\n".into() })
        );
        assert_eq!(
            key_op(None, Some("numpad7"), &mods(false, false, false, false), false, &cfg),
            Some(PopupOp::Append { text: "7".into() })
        );
    }

    #[test]
    fn held_keys_become_repeat_ops() {
        assert_eq!(
            key_op(Some("a"), None, &mods(false, false, false, false), true, &text_cfg()),
            Some(PopupOp::Repeat)
        );
        assert_eq!(
            key_op(None, Some("backspace"), &mods(false, false, false, false), true, &raw_cfg()),
            Some(PopupOp::Repeat)
        );
    }

    #[test]
    fn raw_mode_keeps_labels_and_combos() {
        let cfg = raw_cfg();
        assert_eq!(
            key_op(None, Some("backspace"), &mods(false, false, false, false), false, &cfg),
            Some(PopupOp::Append { text: "⌫".into() })
        );
        assert_eq!(
            key_op(Some("h"), None, &mods(true, true, false, false), false, &cfg),
            Some(PopupOp::Append { text: " CTRL + ALT + H ".into() })
        );
    }

    #[test]
    fn mouse_labels_match_legacy_format() {
        let mut config = Config::default();
        config.show_mouse_coordinates = false;
        assert_eq!(format_mouse(1, Some((10, 20)), &config), " MOUSE1 ");
        config.show_mouse_coordinates = true;
        assert_eq!(format_mouse(1, Some((10, 20)), &config), " MOUSE1 X: 10 Y: 20 ");
        assert_eq!(format_mouse(2, None, &config), " MOUSE2 ");
    }
}
