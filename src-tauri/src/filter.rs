//! Process filter: when enabled, capture runs only while one of the configured
//! processes owns the focused window. Port of the Electron checkActiveProcess().

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::config::SharedConfig;

pub fn spawn(config: SharedConfig, capturing: Arc<AtomicBool>) {
    let mut was_enabled = false;
    std::thread::spawn(move || loop {
        let (enabled, filters, interval) = match config.read() {
            Ok(cfg) => (
                cfg.filter,
                cfg.filter_process_name
                    .iter()
                    .map(|name| name.trim().to_lowercase())
                    .filter(|name| !name.is_empty())
                    .collect::<Vec<_>>(),
                cfg.filter_check_every_second.max(0.1),
            ),
            Err(_) => (false, Vec::new(), 1.0),
        };

        // Turning the filter off must not leave capture stuck in whatever
        // state the last focus check chose.
        if was_enabled && !enabled {
            capturing.store(true, Ordering::Relaxed);
        }
        was_enabled = enabled;

        if enabled && !filters.is_empty() {
            if let Ok(window) = active_win_pos_rs::get_active_window() {
                let process_name = window
                    .process_path
                    .file_name()
                    .map(|name| name.to_string_lossy().to_lowercase())
                    .unwrap_or_default();
                let app_name = window.app_name.to_lowercase();

                let matches = filters
                    .iter()
                    .any(|filter| filter == &process_name || filter == &app_name);
                capturing.store(matches, Ordering::Relaxed);
            }
        }

        std::thread::sleep(Duration::from_secs_f64(interval));
    });
}
