//! Named config profiles + bundled presets.
//!
//! A profile is a full [`Config`] snapshot saved as `<name>.json` under a
//! `profiles/` directory next to config.json. Presets are built-in starter
//! looks that tweak the *current* config (so infrastructure like the OBS port,
//! monitor and hotkey are preserved). Switching a profile / applying a preset
//! goes through [`crate::apply_active`], so it live-updates like a normal save.

use std::path::PathBuf;

use tauri::{AppHandle, Emitter, State};
use tauri_plugin_dialog::DialogExt;

use crate::config::{Config, SharedConfig};

/// Directory holding profile files, alongside the active config.json.
fn profiles_dir(app: &AppHandle) -> PathBuf {
    crate::config::config_path(app)
        .parent()
        .map(|dir| dir.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."))
        .join("profiles")
}

/// Sanitizes a display name into a safe file stem (no path traversal, no odd
/// characters). Returns `None` for names that reduce to nothing.
fn safe_name(name: &str) -> Option<String> {
    let cleaned: String = name
        .chars()
        .filter(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_'))
        .collect();
    let cleaned = cleaned.trim();
    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned.to_string())
    }
}

fn profile_path(app: &AppHandle, name: &str) -> Option<PathBuf> {
    safe_name(name).map(|stem| profiles_dir(app).join(format!("{stem}.json")))
}

fn active_file(app: &AppHandle) -> PathBuf {
    profiles_dir(app).join(".active")
}

fn set_active(app: &AppHandle, name: &str) {
    let _ = std::fs::create_dir_all(profiles_dir(app));
    let _ = std::fs::write(active_file(app), name);
    let _ = app.emit("profiles-updated", ());
}

/// The currently-selected profile name (empty if none / custom edits).
#[tauri::command]
pub fn get_active_profile(app: AppHandle) -> String {
    std::fs::read_to_string(active_file(&app))
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

/// Names of saved profiles, sorted case-insensitively.
#[tauri::command]
pub fn list_profiles(app: AppHandle) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(profiles_dir(&app))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                path.file_stem().map(|s| s.to_string_lossy().to_string())
            } else {
                None
            }
        })
        .collect();
    names.sort_by_key(|n| n.to_lowercase());
    names
}

/// Saves the current config as a profile (creating or overwriting it).
#[tauri::command]
pub fn save_profile(app: AppHandle, state: State<SharedConfig>, name: String) -> Result<(), String> {
    let path = profile_path(&app, &name).ok_or("Please enter a valid profile name")?;
    let config = state.read().map(|c| c.clone()).unwrap_or_default();
    let json = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(profiles_dir(&app)).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())?;
    set_active(&app, &safe_name(&name).unwrap_or_default());
    Ok(())
}

/// Loads a profile and makes it the live config. Returns it so the UI refreshes.
#[tauri::command]
pub fn load_profile(app: AppHandle, name: String) -> Result<Config, String> {
    let path = profile_path(&app, &name).ok_or("Invalid profile name")?;
    let raw = std::fs::read_to_string(&path).map_err(|e| format!("Couldn't read profile: {e}"))?;
    let config: Config = serde_json::from_str(&raw).map_err(|e| format!("Invalid profile: {e}"))?;
    crate::apply_active(&app, &config)?;
    set_active(&app, &safe_name(&name).unwrap_or_default());
    Ok(config)
}

#[tauri::command]
pub fn delete_profile(app: AppHandle, name: String) -> Result<(), String> {
    let path = profile_path(&app, &name).ok_or("Invalid profile name")?;
    std::fs::remove_file(&path).map_err(|e| e.to_string())?;
    if get_active_profile(app.clone()) == safe_name(&name).unwrap_or_default() {
        let _ = std::fs::remove_file(active_file(&app));
        let _ = app.emit("profiles-updated", ());
    }
    Ok(())
}

#[tauri::command]
pub fn rename_profile(app: AppHandle, old: String, new: String) -> Result<(), String> {
    let from = profile_path(&app, &old).ok_or("Invalid profile name")?;
    let to = profile_path(&app, &new).ok_or("Please enter a valid new name")?;
    std::fs::rename(&from, &to).map_err(|e| e.to_string())?;
    if get_active_profile(app.clone()) == safe_name(&old).unwrap_or_default() {
        set_active(&app, &safe_name(&new).unwrap_or_default());
    } else {
        let _ = app.emit("profiles-updated", ());
    }
    Ok(())
}

/// Names of the bundled presets, in display order.
#[tauri::command]
pub fn list_presets() -> Vec<String> {
    vec!["Minimal".into(), "Streamer".into(), "Gamer".into()]
}

/// Applies a bundled preset on top of the current config (keeping infrastructure
/// like OBS port, monitor, hotkey and layout), then makes it live.
#[tauri::command]
pub fn apply_preset(app: AppHandle, state: State<SharedConfig>, name: String) -> Result<Config, String> {
    let mut config = state.read().map(|c| c.clone()).unwrap_or_default();
    match name.to_lowercase().as_str() {
        "minimal" => {
            config.display_style = "popups".into();
            config.display_mode = "text".into();
            config.popup_font_size = 16.0;
            config.popup_opacity = 0.7;
            config.popup_font_color = "#ffffff".into();
            config.popup_background_color = "#000000".into();
            config.position = "bottom-center".into();
            config.show_mouse_click = false;
            config.show_mouse_movement = false;
            config.show_gamepad = false;
        }
        "streamer" => {
            config.display_style = "popups".into();
            config.display_mode = "raw".into();
            config.popup_font_size = 24.0;
            config.popup_opacity = 0.9;
            config.popup_font_color = "#ffffff".into();
            config.popup_background_color = "#1e1e2e".into();
            config.position = "bottom-center".into();
            config.show_keyboard_click = true;
            config.show_mouse_click = true;
            config.show_mouse_movement = true;
            config.show_mouse_scroll = true;
        }
        "gamer" => {
            config.display_style = "keyboard".into();
            config.keyboard_visible_keys = [
                "KeyQ", "KeyW", "KeyE", "KeyR", "KeyA", "KeyS", "KeyD", "KeyF", "ShiftLeft",
                "Space", "ControlLeft",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect();
            config.position = "bottom-center".into();
            config.show_mouse_movement = true;
            config.show_mouse_click = true;
            config.show_gamepad = true;
        }
        _ => return Err(format!("Unknown preset: {name}")),
    }
    crate::apply_active(&app, &config)?;
    // A preset is not a named profile.
    let _ = std::fs::remove_file(active_file(&app));
    let _ = app.emit("profiles-updated", ());
    Ok(config)
}

/// Exports the current config to a user-chosen JSON file. Returns false if the
/// save dialog was cancelled.
#[tauri::command]
pub fn export_config(app: AppHandle, state: State<SharedConfig>) -> Result<bool, String> {
    let config = state.read().map(|c| c.clone()).unwrap_or_default();
    let json = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
    let picked = app
        .dialog()
        .file()
        .add_filter("YAKC config", &["json"])
        .set_file_name("yakc-config.json")
        .blocking_save_file();
    let Some(path) = picked else { return Ok(false) };
    let path = path.into_path().map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())?;
    Ok(true)
}

/// Imports a config from a user-chosen JSON file and makes it live. Returns the
/// new config, or None if the dialog was cancelled.
#[tauri::command]
pub fn import_config(app: AppHandle) -> Result<Option<Config>, String> {
    let picked = app
        .dialog()
        .file()
        .add_filter("YAKC config", &["json"])
        .blocking_pick_file();
    let Some(path) = picked else { return Ok(None) };
    let path = path.into_path().map_err(|e| e.to_string())?;
    let raw = std::fs::read_to_string(&path).map_err(|e| format!("Couldn't read file: {e}"))?;
    let config: Config = serde_json::from_str(&raw).map_err(|e| format!("Invalid config file: {e}"))?;
    crate::apply_active(&app, &config)?;
    let _ = std::fs::remove_file(active_file(&app));
    let _ = app.emit("profiles-updated", ());
    Ok(Some(config))
}
