# YAKC - Yet Another Key Caster

![yakc-logo](https://github.com/iammodev/YAKC/assets/89686923/d776922e-ebb8-42b0-b49f-c516d52957ae)

**YAKC** is an **open-source**, **cross-platform** **key & mouse click visualizer** for **content creators, developers and presentations** — now rewritten in [Tauri](https://tauri.app) + Rust.

## Why the rewrite?

The Electron version needed Node.js, a native `iohook` build per platform, and shipped a full browser runtime. The Tauri rewrite is a single small binary (~10 MB) with:

- **Zero JavaScript dependencies** — the UI is plain HTML/CSS/JS, no npm, no bundler, no framework.
- **Any keyboard language, automatically** — keys are translated by the operating system itself (Windows `ToUnicode`, macOS `CGEventKeyboardGetUnicodeString`, Linux `xkbcommon`). The old hand-written per-language layout files are gone; QWERTZ, AZERTY, Turkish, Cyrillic, … all just work.
- **Full platform parity** — every feature works on Windows, macOS, and Linux on **both X11 and Wayland**.

## Features

- **Two display modes**:
  - `text` (default) — behaves like a text editor: only the characters you type appear, **Backspace really deletes**, shortcuts and navigation keys stay hidden
  - `raw` — every key shows: modifiers (`CTRL + ALT + H`), `⌫`, arrows, F-keys, …
- **Held keys don't spam** — they show a counter: `a (x13)`
- Display **key** & **mouse** clicks (optionally with coordinates)
- Highly customizable popups (size, opacity, colors, font, corner radius)
- Any screen corner + pixel offsets, on **any monitor**
- Smooth fade-out transition
- **Settings GUI** — configure everything at runtime, applies live
- **Global hotkey** to toggle capturing (default `Ctrl+Alt+Y`)
- Tray icon: toggle capturing, open settings, quit
- **Text-to-speech** for each keystroke (great for blind users)
- **Process filter**: only capture while selected apps are focused

## How it works per platform

| | Windows | macOS | Linux (X11) | Linux (Wayland) |
|---|---|---|---|---|
| Key/mouse capture | `WH_KEYBOARD_LL` hook | `CGEventTap` | `/dev/input` (evdev) | `/dev/input` (evdev) |
| Layout translation | OS (`ToUnicode`) | OS (`UCKeyTranslate`) | xkbcommon | xkbcommon, keymap fetched **from the compositor** — exactly the layout you configured in your desktop settings |
| Overlay | native | native (above fullscreen apps) | native | via XWayland¹ |
| Text-to-speech | SAPI (built in) | AVSpeechSynthesizer (built in) | speech-dispatcher (offered automatically²) | speech-dispatcher (offered automatically²) |

¹ Wayland compositors forbid regular apps from self-positioning always-on-top overlays, so YAKC renders its overlay through XWayland (available on effectively every compositor, including GNOME and KDE). Two sub-features are affected by Wayland's security model, which hides this information from *all* applications: global mouse coordinates (hidden on Wayland rather than showing wrong values) and the process filter for native-Wayland apps (off by default).

² If text-to-speech is enabled but no engine is installed, YAKC offers to install it for you — one click + your password.

## Installation

### Package managers

| Platform | Command |
|---|---|
| **Windows** (winget) | `winget install iammodev.YAKC` |
| **Arch Linux** (AUR) | `yay -S yakc` |

### Direct download

Grab a package from the [releases page](https://github.com/iammodev/YAKC/releases) and install it the usual way for your OS:

| Platform | Package | Install |
|---|---|---|
| Windows | `.msi` / `-setup.exe` | double-click, next-next-finish |
| macOS (Intel + Apple Silicon) | `_universal.dmg` | open, drag YAKC to Applications |
| Debian/Ubuntu | `.deb` | double-click, or `sudo apt install ./YAKC_*.deb` |
| Fedora/openSUSE | `.rpm` | double-click, or `sudo dnf install ./YAKC-*.rpm` |
| Any Linux | `.AppImage` | make executable, run |

Both x64 and ARM64 builds are provided for Windows and Linux; the macOS `.dmg` is a universal binary that runs on both Intel and Apple Silicon.

**That's the whole installation — everything else is prompted.** On first launch YAKC checks what it needs and simply asks:

- **Linux**: "YAKC needs permission to read your keyboard and mouse devices — set this up now?" → click *Yes*, type your password into the system prompt, done. Keys start appearing seconds later, no log-out, no terminal.
- **macOS**: "YAKC needs the Accessibility permission — open System Settings now?" → click *Yes*, flip the switch next to YAKC, restart the app. (Apple doesn't allow apps to grant this themselves.)
- **Windows**: no setup at all.
- **Text-to-speech** (any platform, only if you enable it): if an engine is missing, YAKC asks "Install it now?" → click *Yes*, type your password. Windows/macOS voices are built into the OS.

### Build from source

Prerequisites: [Rust](https://rustup.rs/) and the [Tauri system dependencies](https://v2.tauri.app/start/prerequisites/) for your OS (on Linux: `webkit2gtk-4.1`, `libxkbcommon`; TTS additionally wants `speech-dispatcher` headers at build time).

```bash
git clone https://github.com/iammodev/YAKC.git
cd YAKC
cargo install tauri-cli
cargo tauri build        # bundles in src-tauri/target/release/bundle/
# or, during development:
cargo tauri dev
```

No Node.js, npm, or any JavaScript toolchain required.

> Note: on rolling-release distros (e.g. Arch) the AppImage bundler's bundled `strip` chokes on modern system libraries — build with `NO_STRIP=true cargo tauri build` there. Ubuntu (and the CI runners) are unaffected.

## Usage

1. Start YAKC. A tray icon appears; pressing any key shows a popup.
2. **Right-click the tray icon** → *Toggle Capturing*, *Settings…*, or *Quit*.
3. Press the **toggle hotkey** (default `Ctrl+Alt+Y`) to start/stop capturing from anywhere.
4. Open **Settings…** to change anything at runtime — appearance, position, monitor, behavior. Saving applies live.

## Configuration

Settings live in `config.json` — edit them via the Settings window or by hand:

- Linux/macOS: `~/.config/dev.iammodev.yakc/config.json` (Linux), `~/Library/Application Support/dev.iammodev.yakc/config.json` (macOS)
- Windows: `%APPDATA%\dev.iammodev.yakc\config.json`
- Portable: a `config.json` next to the executable takes precedence (old Electron-style layouts keep working — stringly-typed numbers are accepted).

| Key | Description |
|---|---|
| `displayMode` | `text` (default): like a text editor — only typed characters, Backspace deletes, shortcuts hidden. `raw`: every key including modifiers and symbols. |
| `keyboardLayout` | **Linux only**: xkb layout override (`us`, `de`, `tr`, …). Empty = auto-detect from the compositor/session. Legacy values `english`/`german` still work. Other platforms always use the OS layout. |
| `showOnMonitor` | Monitor index to display popups on (0 = first) |
| `popupTextMaxWidthInPercentage` | Max popup width as % of screen width |
| `popupOpacity` | Popup opacity, `0.0`–`1.0` |
| `popupFadeInSeconds` | Fade transition duration |
| `popupRemoveAfterSeconds` | Remove inactive popup after X seconds |
| `popupInactiveAfterSeconds` | After X seconds of no input, the next key starts a new popup |
| `popupFontSize` / `popupFontFamily` / `popupFontWeight` | Popup text font |
| `popupFontColor` / `popupBackgroundColor` | Popup colors |
| `popupBorderRadius` | Corner radius (`0` = sharp) |
| `showKeyboardClick` / `showMouseClick` / `showMouseCoordinates` | What to display (mouse coordinates are unavailable on Wayland and hidden there) |
| `onlyKeysWithModifiers` | Only show keys pressed together with Ctrl/Alt/Meta |
| `showSpaceAsUnicode` | Show space as `␣` |
| `textToSymbols` | Special keys as symbols (Tab → `↹`, Backspace → `⌫`, …) |
| `textToSpeech` / `textToSpeechCancelSpeechOnNewKey` | Speak keystrokes aloud |
| `position` | `top-left`, `top-right`, `bottom-left`, `bottom-right` |
| `topOffset` / `bottomOffset` / `leftOffset` / `rightOffset` | Pixel offsets from the anchored corner |
| `filter` / `filterProcessName` / `filterCheckEverySecond` | Capture only while listed processes are focused |
| `toggleCaptureHotkey` | Global capture toggle, e.g. `Ctrl+Alt+Y` (needs ≥ 1 modifier) |

## TODO

- [x] Reliable solution for all/common keyboard layouts *(OS-native translation)*
- [x] position (top-left, top-right, bottom-left, bottom-right)
- [x] topOffset, bottomOffset, leftOffset, rightOffset
- [x] GUI to easily configure at runtime
- [x] Add hotkey for start/stop listening to keystrokes
- [x] Add unit tests
- [ ] Drag and drop the popup to the desired position (needs a temporary non-click-through mode)

## Buy me a coffee

<a href="https://www.buymeacoffee.com/iammodev" target="_blank"><img src="https://www.buymeacoffee.com/assets/img/custom_images/orange_img.png" alt="Buy Me A Coffee" style="height: 37px !important;width: 170px !important;box-shadow: 0px 3px 2px 0px rgba(190, 190, 190, 0.5) !important;-webkit-box-shadow: 0px 3px 2px 0px rgba(190, 190, 190, 0.5) !important;" ></a>

[![ko-fi](https://ko-fi.com/img/githubbutton_sm.svg)](https://ko-fi.com/iammodev)

## Contributing

Contributions are welcome! If you find any issues or have suggestions for improvements, please open an issue or submit a pull request. (Keyboard-layout files are no longer needed — the OS handles every language.)

## Security Assurance

YAKC is free and open source. YAKC operates independently without any network interactions. Your private information, including passwords, is never stored or shared by YAKC, guaranteeing your safety and privacy.

**Please Exercise Caution**: When using YAKC for activities like presentation, recording or streaming, be mindful not to inadvertently share sensitive information. Always ensure your privacy and the security of any confidential data.

## License

This project is licensed under the `MIT` License. See the [LICENSE](LICENSE) file for details.
