//! KDE Wayland active-window source.
//!
//! Wayland deliberately hides the focused window from normal apps, so the
//! X11-based `active-win-pos-rs` returns nothing under a Wayland session. KDE
//! (KWin) still exposes it through its scripting D-Bus interface, so we own a
//! tiny D-Bus service and load a KWin script that pushes the active window's
//! class to us on every focus change. Push-based (no polling), and entirely
//! best-effort: if anything is missing (not KDE, no session bus, …) we return
//! `None` and the process filter simply falls back / fails open.

use crate::filter::Engine;
use std::sync::Arc;

const SERVICE: &str = "org.yakc.KwinFocus";
const PATH: &str = "/Focus";
const PLUGIN: &str = "yakcfocus";

// Runs inside KWin: reports the active window's class whenever focus changes.
// `resourceClass` is the Wayland app id (e.g. "org.kde.konsole", "firefox").
const SCRIPT: &str = r#"
function report() {
  var c = workspace.activeWindow || workspace.activeClient;
  var name = c ? (c.resourceClass || c.resourceName || "") : "";
  callDBus("org.yakc.KwinFocus", "/Focus", "org.yakc.KwinFocus", "Report", String(name));
}
if (workspace.windowActivated) workspace.windowActivated.connect(report);
if (workspace.clientActivated) workspace.clientActivated.connect(report);
report();
"#;

struct Focus {
    engine: Arc<Engine>,
}

#[zbus::interface(name = "org.yakc.KwinFocus")]
impl Focus {
    fn report(&self, app: String) {
        // Push the new focus straight into the filter — event-driven, no polling.
        self.engine.set_active(app);
    }
}

/// Starts the KDE Wayland focus watcher when on a Wayland session (best-effort;
/// a no-op on X11, where the caller's poll thread queries the focused window).
pub fn start(engine: Arc<Engine>) {
    if std::env::var("WAYLAND_DISPLAY").is_err() {
        return;
    }
    std::thread::spawn(move || {
        if let Err(err) = run(engine) {
            // Not fatal — the filter falls back to fail-open without this.
            eprintln!("YAKC: KDE Wayland focus watcher unavailable: {err}");
        }
    });
}

fn run(engine: Arc<Engine>) -> Result<(), Box<dyn std::error::Error>> {
    // loadScript takes a file path, so drop the script somewhere stable.
    let path = std::env::temp_dir().join("yakc-kwin-focus.js");
    std::fs::write(&path, SCRIPT)?;

    // Own our service so the script's callDBus reaches us, then keep the
    // connection alive so it keeps dispatching Report calls.
    let conn = zbus::blocking::connection::Builder::session()?
        .name(SERVICE)?
        .serve_at(PATH, Focus { engine })?
        .build()?;

    let scripting = zbus::blocking::Proxy::new(
        &conn,
        "org.kde.KWin",
        "/Scripting",
        "org.kde.kwin.Scripting",
    )?;
    // Drop any leftover instance from a previous run so they don't stack up.
    let _: zbus::Result<bool> = scripting.call("unloadScript", &(PLUGIN,));
    let id: i32 = scripting.call("loadScript", &(path.to_string_lossy().as_ref(), PLUGIN))?;
    let script = zbus::blocking::Proxy::new(
        &conn,
        "org.kde.KWin",
        format!("/Scripting/Script{id}"),
        "org.kde.kwin.Script",
    )?;
    let _: zbus::Result<()> = script.call("run", &());

    loop {
        std::thread::park();
    }
}
