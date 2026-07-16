use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use serde::{Deserialize, Deserializer, Serialize};
use tauri::{AppHandle, Manager};

pub type SharedConfig = Arc<RwLock<Config>>;

/// Runtime configuration. Field names serialize to the same camelCase keys the
/// Electron version used, so existing config.json files keep working.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Config {
    /// Linux only: xkb layout override (e.g. "us", "de", "tr"). Empty = auto-detect.
    /// Legacy values "english"/"german" are mapped to "us"/"de".
    pub keyboard_layout: String,
    #[serde(deserialize_with = "lenient_usize")]
    pub show_on_monitor: usize,
    #[serde(deserialize_with = "lenient_f64")]
    pub popup_text_max_width_in_percentage: f64,
    #[serde(deserialize_with = "lenient_f64")]
    pub popup_opacity: f64,
    #[serde(deserialize_with = "lenient_f64")]
    pub popup_fade_in_seconds: f64,
    #[serde(deserialize_with = "lenient_f64")]
    pub popup_remove_after_seconds: f64,
    #[serde(deserialize_with = "lenient_f64")]
    pub popup_inactive_after_seconds: f64,
    #[serde(deserialize_with = "lenient_f64")]
    pub popup_font_size: f64,
    pub popup_font_family: String,
    pub popup_font_weight: String,
    #[serde(deserialize_with = "lenient_f64")]
    pub popup_border_radius: f64,
    pub popup_font_color: String,
    pub popup_background_color: String,
    pub show_keyboard_click: bool,
    pub show_mouse_click: bool,
    pub show_mouse_coordinates: bool,
    pub only_keys_with_modifiers: bool,
    pub show_space_as_unicode: bool,
    pub text_to_symbols: bool,
    pub text_to_speech: bool,
    pub text_to_speech_cancel_speech_on_new_key: bool,
    pub position: String,
    #[serde(deserialize_with = "lenient_f64")]
    pub top_offset: f64,
    #[serde(deserialize_with = "lenient_f64")]
    pub bottom_offset: f64,
    #[serde(deserialize_with = "lenient_f64")]
    pub left_offset: f64,
    #[serde(deserialize_with = "lenient_f64")]
    pub right_offset: f64,
    pub filter: bool,
    pub filter_process_name: Vec<String>,
    #[serde(deserialize_with = "lenient_f64")]
    pub filter_check_every_second: f64,
    /// Global hotkey toggling keystroke capture, e.g. "Ctrl+Alt+Y".
    pub toggle_capture_hotkey: String,
    /// "text": popups behave like a text editor — only typed characters show,
    /// Backspace deletes. "raw": every key shows (modifiers, ⌫, arrows, …).
    pub display_mode: String,
}

impl Config {
    pub fn is_raw_mode(&self) -> bool {
        self.display_mode.eq_ignore_ascii_case("raw")
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            keyboard_layout: String::new(),
            show_on_monitor: 0,
            popup_text_max_width_in_percentage: 60.0,
            popup_opacity: 0.9,
            popup_fade_in_seconds: 0.5,
            popup_remove_after_seconds: 3.0,
            popup_inactive_after_seconds: 0.5,
            popup_font_size: 20.0,
            popup_font_family: "Tahoma, sans-serif".into(),
            popup_font_weight: "bold".into(),
            popup_border_radius: 10.0,
            popup_font_color: "#ffffff".into(),
            popup_background_color: "#000000".into(),
            show_keyboard_click: true,
            show_mouse_click: false,
            show_mouse_coordinates: false,
            only_keys_with_modifiers: false,
            show_space_as_unicode: false,
            text_to_symbols: true,
            text_to_speech: false,
            text_to_speech_cancel_speech_on_new_key: false,
            position: "top-left".into(),
            top_offset: 0.0,
            bottom_offset: 0.0,
            left_offset: 0.0,
            right_offset: 0.0,
            filter: false,
            filter_process_name: Vec::new(),
            filter_check_every_second: 0.5,
            toggle_capture_hotkey: "Ctrl+Alt+Y".into(),
            display_mode: "text".into(),
        }
    }
}

/// Accepts a JSON number or a numeric string ("0.9"), which the Electron-era
/// config files used throughout.
fn lenient_f64<'de, D: Deserializer<'de>>(deserializer: D) -> Result<f64, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum NumOrStr {
        Num(f64),
        Str(String),
    }
    match NumOrStr::deserialize(deserializer)? {
        NumOrStr::Num(n) => Ok(n),
        NumOrStr::Str(s) => s.trim().parse().map_err(serde::de::Error::custom),
    }
}

fn lenient_usize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<usize, D::Error> {
    Ok(lenient_f64(deserializer)?.max(0.0) as usize)
}

/// Path of the active config file: a config.json next to the executable wins
/// (portable installs and the old Electron layout), otherwise the platform
/// config directory.
pub fn config_path(app: &AppHandle) -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let portable = dir.join("config.json");
            if portable.exists() {
                return portable;
            }
        }
    }
    match app.path().app_config_dir() {
        Ok(dir) => dir.join("config.json"),
        Err(_) => PathBuf::from("config.json"),
    }
}

/// Loads the config, creating it with defaults on first run.
pub fn load(app: &AppHandle) -> Config {
    let path = config_path(app);
    match std::fs::read_to_string(&path) {
        Ok(raw) => match serde_json::from_str(&raw) {
            Ok(config) => config,
            Err(err) => {
                eprintln!(
                    "YAKC: failed to parse {} ({err}); using default config",
                    path.display()
                );
                Config::default()
            }
        },
        Err(_) => {
            let config = Config::default();
            if let Err(err) = save(app, &config) {
                eprintln!("YAKC: could not write default config: {err}");
            }
            config
        }
    }
}

pub fn save(app: &AppHandle, config: &Config) -> Result<(), String> {
    let path = config_path(app);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_legacy_stringly_typed_config() {
        // Excerpt of a real Electron-era config.json: numbers stored as strings.
        let legacy = r##"{
            "keyboardLayout": "english",
            "showOnMonitor": "0",
            "popupTextMaxWidthInPercentage": "60",
            "popupOpacity": "0.9",
            "popupFadeInSeconds": "0.5",
            "popupRemoveAfterSeconds": "3",
            "popupInactiveAfterSeconds": "0.5",
            "popupFontSize": "20",
            "popupFontFamily": "Tahoma, sans-serif",
            "popupFontWeight": "bold",
            "popupBorderRadius": "10",
            "popupFontColor": "#ffffff",
            "popupBackgroundColor": "#000000",
            "showKeyboardClick": true,
            "showMouseClick": false,
            "showMouseCoordinates": false,
            "onlyKeysWithModifiers": false,
            "showSpaceAsUnicode": false,
            "textToSymbols": true,
            "textToSpeech": false,
            "textToSpeechCancelSpeechOnNewKey": false,
            "position": "top-left",
            "topOffset": "0",
            "bottomOffset": "0",
            "leftOffset": "0",
            "filter": false,
            "filterProcessName": ["App1.exe", "app2.exe"],
            "filterCheckEverySecond": "0.5"
        }"##;
        let config: Config = serde_json::from_str(legacy).expect("legacy config must parse");
        assert_eq!(config.show_on_monitor, 0);
        assert_eq!(config.popup_opacity, 0.9);
        assert_eq!(config.popup_font_size, 20.0);
        assert!(config.text_to_symbols);
        assert_eq!(config.filter_process_name.len(), 2);
        // Fields absent from legacy files fall back to defaults.
        assert_eq!(config.right_offset, 0.0);
        assert_eq!(config.toggle_capture_hotkey, "Ctrl+Alt+Y");
    }

    #[test]
    fn parses_modern_numeric_config_roundtrip() {
        let config = Config::default();
        let json = serde_json::to_string(&config).unwrap();
        let back: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(back.popup_opacity, config.popup_opacity);
        assert_eq!(back.position, config.position);
    }
}
