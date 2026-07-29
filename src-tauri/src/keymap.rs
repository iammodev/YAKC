#[cfg(test)]
use std::collections::HashMap;

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
        // Key-label override takes priority over everything else.
        if let Some(overridden) = config.key_label_overrides.get(id) {
            overridden.clone()
        } else if id == "space" {
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
        let mut parts: Vec<String> = Vec::new();
        if mods.ctrl {
            parts.push(
                config
                    .key_label_overrides
                    .get("ctrl")
                    .cloned()
                    .unwrap_or_else(|| "CTRL".to_string()),
            );
        }
        if mods.alt {
            parts.push(
                config
                    .key_label_overrides
                    .get("alt")
                    .cloned()
                    .unwrap_or_else(|| "ALT".to_string()),
            );
        }
        if mods.shift {
            parts.push(
                config
                    .key_label_overrides
                    .get("shift")
                    .cloned()
                    .unwrap_or_else(|| "SHIFT".to_string()),
            );
        }
        if mods.meta {
            parts.push(
                config
                    .key_label_overrides
                    .get("meta")
                    .cloned()
                    .unwrap_or_else(|| "META".to_string()),
            );
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

/// Scroll-wheel directions, as (id, default label). Ids are overridable via
/// `keyLabelOverrides`, exactly like named keys.
const SCROLL_KEYS: &[(&str, &str)] = &[
    ("scrollup", "Scroll↑"),
    ("scrolldown", "Scroll↓"),
    ("scrollleft", "Scroll←"),
    ("scrollright", "Scroll→"),
];

/// Gamepad buttons, as (id, default label). Shared with the gamepad backend.
const GAMEPAD_BUTTONS: &[(&str, &str)] = &[
    ("gp_a", "A"),
    ("gp_b", "B"),
    ("gp_x", "X"),
    ("gp_y", "Y"),
    ("gp_lb", "LB"),
    ("gp_rb", "RB"),
    ("gp_lt", "LT"),
    ("gp_rt", "RT"),
    ("gp_back", "BACK"),
    ("gp_start", "START"),
    ("gp_guide", "GUIDE"),
    ("gp_ls", "L3"),
    ("gp_rs", "R3"),
    ("dpad_up", "D↑"),
    ("dpad_down", "D↓"),
    ("dpad_left", "D←"),
    ("dpad_right", "D→"),
];

/// Resolves a label for an id, honoring `keyLabelOverrides` first, then the
/// table default, then an uppercased fallback.
fn labeled(id: &str, table: &[(&str, &str)], config: &Config) -> String {
    if let Some(overridden) = config.key_label_overrides.get(id) {
        return overridden.clone();
    }
    table
        .iter()
        .find(|(key, _)| *key == id)
        .map(|(_, label)| label.to_string())
        .unwrap_or_else(|| id.to_uppercase())
}

/// Popup label for a scroll tick. `dy > 0` is up, `dx > 0` is right; the
/// dominant axis wins.
pub fn format_scroll(dx: f64, dy: f64, config: &Config) -> String {
    let id = if dy.abs() >= dx.abs() {
        if dy > 0.0 {
            "scrollup"
        } else {
            "scrolldown"
        }
    } else if dx > 0.0 {
        "scrollright"
    } else {
        "scrollleft"
    };
    format!(" {} ", labeled(id, SCROLL_KEYS, config))
}

/// Popup label for a gamepad button press.
pub fn format_gamepad_button(id: &str, config: &Config) -> String {
    format!(" {} ", labeled(id, GAMEPAD_BUTTONS, config))
}

/// A known key that can be overridden, exposed to the settings UI.
#[derive(Debug, Clone, Serialize)]
pub struct KnownKey {
    pub id: String,
    pub default_label: String,
    pub group: String,
}

/// Returns the complete list of keys users can override via `keyLabelOverrides`.
pub fn known_keys() -> Vec<KnownKey> {
    let mut keys = Vec::new();

    // Modifier keys (hardcoded in format_key, not in NAMED_KEYS).
    for (id, label) in [
        ("ctrl", "CTRL"),
        ("alt", "ALT"),
        ("shift", "SHIFT"),
        ("meta", "META"),
    ] {
        keys.push(KnownKey {
            id: id.into(),
            default_label: label.into(),
            group: "modifier".into(),
        });
    }

    for (id, display, _symbol) in NAMED_KEYS {
        let group = if *id == "space"
            || *id == "enter"
            || *id == "backspace"
            || *id == "delete"
            || *id == "insert"
            || *id == "tab"
        {
            "editing"
        } else if *id == "arrowleft"
            || *id == "arrowright"
            || *id == "arrowup"
            || *id == "arrowdown"
            || *id == "home"
            || *id == "end"
            || *id == "pageup"
            || *id == "pagedown"
        {
            "navigation"
        } else if id.starts_with("numpad") {
            "numpad"
        } else if id.starts_with('f') && id.len() <= 3 {
            // f1–f12
            "function"
        } else if *id == "escape"
            || *id == "capslock"
            || *id == "numlock"
            || *id == "scrolllock"
            || *id == "pause"
            || *id == "printscreen"
            || *id == "contextmenu"
        {
            "system"
        } else {
            "other"
        };
        keys.push(KnownKey {
            id: id.to_string(),
            default_label: display.to_string(),
            group: group.into(),
        });
    }

    for (id, label) in SCROLL_KEYS {
        keys.push(KnownKey {
            id: id.to_string(),
            default_label: label.to_string(),
            group: "mouse".into(),
        });
    }

    for (id, label) in GAMEPAD_BUTTONS {
        keys.push(KnownKey {
            id: id.to_string(),
            default_label: label.to_string(),
            group: "gamepad".into(),
        });
    }

    keys
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

    #[test]
    fn scroll_labels_pick_dominant_axis() {
        let config = Config::default();
        assert_eq!(format_scroll(0.0, 1.0, &config), " Scroll↑ ");
        assert_eq!(format_scroll(0.0, -1.0, &config), " Scroll↓ ");
        assert_eq!(format_scroll(1.0, 0.0, &config), " Scroll→ ");
        assert_eq!(format_scroll(-1.0, 0.0, &config), " Scroll← ");
        // Vertical wins ties / mixed input.
        assert_eq!(format_scroll(0.5, 1.0, &config), " Scroll↑ ");
    }

    #[test]
    fn scroll_and_gamepad_labels_honor_overrides() {
        let config = Config {
            key_label_overrides: HashMap::from([
                ("scrollup".into(), "WHEEL-UP".into()),
                ("gp_a".into(), "✕".into()),
            ]),
            ..Config::default()
        };
        assert_eq!(format_scroll(0.0, 2.0, &config), " WHEEL-UP ");
        assert_eq!(format_gamepad_button("gp_a", &config), " ✕ ");
        // Non-overridden buttons keep their default label.
        assert_eq!(format_gamepad_button("gp_lt", &config), " LT ");
        assert_eq!(format_gamepad_button("dpad_up", &config), " D↑ ");
    }

    #[test]
    fn known_keys_include_scroll_and_gamepad_groups() {
        let keys = known_keys();
        assert!(keys
            .iter()
            .any(|k| k.id == "scrollup" && k.group == "mouse"));
        assert!(keys
            .iter()
            .any(|k| k.id == "gp_a" && k.group == "gamepad" && k.default_label == "A"));
    }

    #[test]
    fn named_key_override_takes_priority() {
        let config = Config {
            key_label_overrides: HashMap::from([("backspace".into(), "DEL".into())]),
            ..Config::default()
        };
        let label = format_key(
            None,
            Some("backspace"),
            &mods(false, false, false, false),
            &config,
        );
        assert_eq!(label.as_deref(), Some("DEL"));
    }

    #[test]
    fn named_key_override_survives_text_to_symbols() {
        // Override should win even when text_to_symbols is on.
        let config = Config {
            text_to_symbols: true,
            key_label_overrides: HashMap::from([("enter".into(), "⏎".into())]),
            ..Config::default()
        };
        let label = format_key(
            None,
            Some("enter"),
            &mods(false, false, false, false),
            &config,
        );
        assert_eq!(label.as_deref(), Some("⏎"));
    }

    #[test]
    fn modifier_combo_uses_override() {
        let config = Config {
            key_label_overrides: HashMap::from([("meta".into(), "MOD".into())]),
            ..Config::default()
        };
        let label = format_key(
            Some("h"),
            None,
            &mods(false, false, false, true),
            &config,
        );
        assert_eq!(label.as_deref(), Some(" MOD + H "));
    }

    #[test]
    fn modifier_combo_overrides_all_four() {
        let config = Config {
            key_label_overrides: HashMap::from([
                ("ctrl".into(), "C".into()),
                ("alt".into(), "A".into()),
                ("shift".into(), "S".into()),
                ("meta".into(), "M".into()),
            ]),
            ..Config::default()
        };
        let label = format_key(
            Some("x"),
            None,
            &mods(true, true, true, true),
            &config,
        );
        assert_eq!(label.as_deref(), Some(" C + A + S + M + X "));
    }

    #[test]
    fn partial_modifier_override_leaves_others_unchanged() {
        // Only override meta; ctrl, alt, shift should keep their defaults.
        let config = Config {
            key_label_overrides: HashMap::from([("meta".into(), "SUPER".into())]),
            ..Config::default()
        };
        let label = format_key(
            Some("k"),
            None,
            &mods(true, true, false, true),
            &config,
        );
        assert_eq!(label.as_deref(), Some(" CTRL + ALT + SUPER + K "));
    }

    #[test]
    fn override_does_not_affect_unrelated_named_keys() {
        let config = Config {
            key_label_overrides: HashMap::from([("meta".into(), "MOD".into())]),
            ..Config::default()
        };
        let label = format_key(
            None,
            Some("tab"),
            &mods(false, false, false, false),
            &config,
        );
        assert_eq!(label.as_deref(), Some("↹")); // default symbol
    }

    #[test]
    fn named_key_override_in_combo() {
        let config = Config {
            key_label_overrides: HashMap::from([
                ("backspace".into(), "DEL".into()),
            ]),
            ..Config::default()
        };
        let label = format_key(
            None,
            Some("backspace"),
            &mods(true, false, false, false),
            &config,
        );
        assert_eq!(label.as_deref(), Some(" CTRL + DEL "));
    }
}
