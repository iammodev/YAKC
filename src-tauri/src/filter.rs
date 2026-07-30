//! Process filter: when enabled, capture runs only while one of the configured
//! processes owns the focused window.
//!
//! Two ways focus reaches us: on Wayland the compositor *pushes* focus changes
//! (via [`Engine::set_active`]), so evaluation is fully event-driven — no
//! polling, no delay. Elsewhere (X11/Windows/macOS) there's no focus event, so a
//! background thread polls the focused window on an interval.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::config::SharedConfig;

/// Poll cadence where focus changes aren't pushed to us (X11/Windows/macOS).
/// Tight enough to feel instant; cheap because a disabled filter skips the
/// focus query entirely, so we only do real work while gating is active.
const POLL_INTERVAL: Duration = Duration::from_millis(100);

/// Shared holder for the focused window's class as reported by a push-based
/// source (e.g. the KDE Wayland watcher). `None` = not yet known.
pub type ActiveApp = Arc<Mutex<Option<String>>>;

/// Evaluates the process filter and drives the shared `capturing` flag. Shared
/// (via `Arc`) between the push source (KWin), the settings-save path, and — on
/// non-Wayland — the poll thread, so any of them can trigger a re-evaluation.
pub struct Engine {
    config: SharedConfig,
    capturing: Arc<AtomicBool>,
    /// Whether *we* (the filter) are the reason capture is off, so we can always
    /// restore it when we stop gating — regardless of how we got here.
    forced_off: AtomicBool,
    /// Push-based focus source (Wayland). `None` on X11/Windows/macOS, where the
    /// poll thread queries the focused window instead.
    wayland: Option<ActiveApp>,
}

impl Engine {
    pub fn new(
        config: SharedConfig,
        capturing: Arc<AtomicBool>,
        wayland: Option<ActiveApp>,
    ) -> Arc<Self> {
        Arc::new(Self {
            config,
            capturing,
            forced_off: AtomicBool::new(false),
            wayland,
        })
    }

    /// True when a push source is present, so no polling is needed.
    pub fn is_push(&self) -> bool {
        self.wayland.is_some()
    }

    /// Records a newly-focused window class (Wayland push) and re-evaluates now.
    pub fn set_active(&self, class: String) {
        if let Some(active) = &self.wayland {
            if let Ok(mut guard) = active.lock() {
                *guard = Some(class);
            }
        }
        self.reevaluate();
    }

    /// Restore capture if the filter isn't gating, otherwise gate on the current
    /// focus. Cheap and idempotent — safe to call from any trigger.
    pub fn reevaluate(&self) {
        let (enabled, filters) = match self.config.read() {
            Ok(cfg) => (
                cfg.filter,
                cfg.filter_process_name
                    .iter()
                    .map(|name| name.trim().to_lowercase())
                    .filter(|name| !name.is_empty())
                    .collect::<Vec<_>>(),
            ),
            Err(_) => (false, Vec::new()),
        };

        if !enabled || filters.is_empty() {
            self.restore();
            return;
        }
        match focused_names(&self.wayland) {
            Some(names) if names.iter().any(|n| !n.is_empty()) => {
                let matches = filters
                    .iter()
                    .any(|filter| names.iter().any(|name| name_matches(filter, name)));
                self.capturing.store(matches, Ordering::Relaxed);
                self.forced_off.store(!matches, Ordering::Relaxed);
            }
            // Focus unknown (not reported yet, error, or unreadable): never hold
            // capture hostage — fail open.
            _ => self.restore(),
        }
    }

    /// Undo any suppression we caused, without touching a manual (hotkey/tray) pause.
    fn restore(&self) {
        if self.forced_off.swap(false, Ordering::Relaxed) {
            self.capturing.store(true, Ordering::Relaxed);
        }
    }
}

/// Starts the filter. On Wayland it's driven by pushes (KWin) + settings saves,
/// so no thread is spawned; elsewhere a thread polls the focused window at a
/// snappy fixed cadence (see [`POLL_INTERVAL`]).
pub fn spawn(engine: Arc<Engine>) {
    if engine.is_push() {
        engine.reevaluate(); // apply the initial state
        return;
    }
    std::thread::spawn(move || loop {
        engine.reevaluate();
        std::thread::sleep(POLL_INTERVAL);
    });
}

/// Lowercased candidate names for the currently-focused window, or `None` when
/// the focus can't be determined. On Wayland this is the compositor-reported
/// class; otherwise it's the focused window's process file name and app name.
fn focused_names(wayland: &Option<ActiveApp>) -> Option<Vec<String>> {
    if let Some(active) = wayland {
        return active
            .lock()
            .ok()
            .and_then(|guard| guard.clone())
            .map(|class| vec![class.to_lowercase()]);
    }
    // `get_active_window` can panic on some platforms/compositors; a panic here
    // would kill the thread and strand capture off forever, so contain it.
    match std::panic::catch_unwind(active_win_pos_rs::get_active_window) {
        Ok(Ok(window)) => {
            let process_name = window
                .process_path
                .file_name()
                .map(|name| name.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            let app_name = window.app_name.to_lowercase();
            Some(vec![process_name, app_name])
        }
        _ => None,
    }
}

/// Drops a trailing `.exe`/`.app` so `chrome` and `chrome.exe` compare equal.
/// Only these known suffixes — not any dot — so reverse-DNS Wayland classes like
/// `org.kde.konsole` stay intact (and match `konsole` via containment).
fn strip_ext(name: &str) -> &str {
    name.strip_suffix(".exe")
        .or_else(|| name.strip_suffix(".app"))
        .unwrap_or(name)
}

/// Lenient, case-insensitive match between a configured filter name and an
/// actual window's process/app name. The name a user picks from the running-app
/// list (via `sysinfo`) doesn't always match what the focus API reports — it may
/// carry a `.exe`, be `google-chrome` vs `chrome`, or be truncated — so besides
/// an exact match we accept either name containing the other (guarded by length
/// so short names don't over-match). Both inputs are already lowercased.
fn name_matches(filter: &str, actual: &str) -> bool {
    let f = strip_ext(filter.trim());
    let a = strip_ext(actual.trim());
    if f.is_empty() || a.is_empty() {
        return false;
    }
    f == a || (f.len() >= 3 && a.len() >= 3 && (a.contains(f) || f.contains(a)))
}

#[cfg(test)]
mod tests {
    use super::name_matches;

    #[test]
    fn matches_exact_and_extension() {
        assert!(name_matches("code", "code"));
        assert!(name_matches("chrome", "chrome.exe"));
        assert!(name_matches("obs.exe", "obs"));
    }

    #[test]
    fn matches_differing_report_styles() {
        // sysinfo vs focus API can disagree on the exact string.
        assert!(name_matches("chrome", "google-chrome"));
        assert!(name_matches("code", "code - insiders")); // app_name style
    }

    #[test]
    fn matches_wayland_reverse_dns_class() {
        // KWin reports resourceClass like "org.kde.konsole"; the user picks
        // "konsole" from the process list — these must still match.
        assert!(name_matches("konsole", "org.kde.konsole"));
        assert!(name_matches("dolphin", "org.kde.dolphin"));
    }

    #[test]
    fn rejects_unrelated_and_too_short() {
        assert!(!name_matches("code", "firefox"));
        assert!(!name_matches("x", "xterm")); // too short to contain-match
        assert!(!name_matches("", "anything"));
    }
}
