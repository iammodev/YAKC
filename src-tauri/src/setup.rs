//! Frictionless privileged setup: instead of telling the user to type
//! commands, ask a yes/no question and run the fix through the system's
//! graphical authentication prompt (pkexec / System Settings).
//!
//! All dialogs here block and must never run on the main thread; they are
//! called from the input worker threads.

use tauri::AppHandle;
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons};

#[allow(dead_code)] // unused on Windows, where no setup is needed
fn ask(app: &AppHandle, message: &str) -> bool {
    app.dialog()
        .message(message)
        .title("YAKC Setup")
        .buttons(MessageDialogButtons::YesNo)
        .blocking_show()
}

#[allow(dead_code)]
fn inform(app: &AppHandle, message: &str) {
    app.dialog()
        .message(message)
        .title("YAKC Setup")
        .blocking_show();
}

/// Linux: grant read access to /dev/input via pkexec.
/// `usermod` makes it permanent (from the next login); `setfacl` makes it work
/// immediately — the device rescan loop picks the keyboards up seconds later.
/// Returns true when the fix was applied.
#[cfg(target_os = "linux")]
pub fn offer_input_access_fix(app: &AppHandle) -> bool {
    let confirmed = ask(
        app,
        "To show your keystrokes, YAKC needs permission to read your keyboard and mouse \
         devices (/dev/input).\n\nSet this up now? Your system will ask for your password.",
    );
    if !confirmed {
        return false;
    }

    let user = std::env::var("USER").unwrap_or_default();
    if user.is_empty() {
        return false;
    }
    let script = format!(
        "usermod -aG input '{user}' && \
         {{ setfacl -m 'u:{user}:r' /dev/input/event* 2>/dev/null || true; }}"
    );
    let ok = std::process::Command::new("pkexec")
        .args(["sh", "-c", &script])
        .status()
        .map(|status| status.success())
        .unwrap_or(false);

    if ok {
        inform(
            app,
            "All set! YAKC will start showing your keys within a few seconds.",
        );
    } else {
        inform(
            app,
            "Setup didn't complete (cancelled or failed).\n\nYou can retry by restarting YAKC, \
             or set it up manually:\n  sudo usermod -aG input $USER\nand log out and back in.",
        );
    }
    ok
}

/// Linux: install the text-to-speech engine (speech-dispatcher + espeak-ng)
/// via the distribution's package manager through pkexec.
/// Returns true when the install succeeded.
#[cfg(target_os = "linux")]
pub fn offer_tts_install(app: &AppHandle) -> bool {
    let Some(install) = detect_install_command() else {
        inform(
            app,
            "Text-to-speech needs the speech-dispatcher engine, but YAKC couldn't detect your \
             package manager. Please install 'speech-dispatcher' and 'espeak-ng' with your \
             distribution's tools.",
        );
        return false;
    };

    let confirmed = ask(
        app,
        "Text-to-speech needs the speech-dispatcher engine, which isn't installed.\n\n\
         Install it now? Your system will ask for your password.",
    );
    if !confirmed {
        return false;
    }

    let ok = std::process::Command::new("pkexec")
        .args(["sh", "-c", install])
        .status()
        .map(|status| status.success())
        .unwrap_or(false);

    if ok {
        inform(app, "Text-to-speech is ready.");
    } else {
        inform(
            app,
            "The installation didn't complete. You can retry by toggling text-to-speech in the \
             settings, or install 'speech-dispatcher' and 'espeak-ng' manually.",
        );
    }
    ok
}

#[cfg(target_os = "linux")]
fn detect_install_command() -> Option<&'static str> {
    const CANDIDATES: &[(&str, &str)] = &[
        ("pacman", "pacman -S --needed --noconfirm speech-dispatcher espeak-ng"),
        ("apt-get", "apt-get install -y speech-dispatcher espeak-ng"),
        ("dnf", "dnf install -y speech-dispatcher espeak-ng"),
        ("zypper", "zypper --non-interactive install speech-dispatcher espeak-ng"),
    ];
    CANDIDATES.iter().find_map(|(bin, cmd)| {
        let found = std::process::Command::new("sh")
            .args(["-c", &format!("command -v {bin}")])
            .output()
            .is_ok_and(|out| out.status.success());
        found.then_some(*cmd)
    })
}

/// macOS: the Accessibility permission cannot be granted programmatically —
/// offer to open the exact System Settings pane instead.
#[cfg(target_os = "macos")]
pub fn offer_accessibility_fix(app: &AppHandle) {
    let confirmed = ask(
        app,
        "To show your keystrokes, YAKC needs the Accessibility permission.\n\n\
         Open System Settings now? Enable YAKC under Privacy & Security → Accessibility, \
         then restart the app.",
    );
    if confirmed {
        let _ = std::process::Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
            .status();
    }
}
