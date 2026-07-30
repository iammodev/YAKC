/**
 * YAKC on-screen keyboard ("keyboard" display style).
 *
 * Driven by the structured `key-flash` event from Rust, keyed by PHYSICAL key
 * position (W3C KeyboardEvent.code style, e.g. "KeyY", "ShiftLeft"), so the
 * correct cap lights up on any layout (QWERTY/QWERTZ/AZERTY/…) and left/right
 * modifiers are distinct. Cap labels default to a US layout but are relabeled
 * live from the character the OS actually produced, so the displayed keyboard
 * matches the user's real layout as they type.
 *
 * Payloads: { code, label? }  → a regular key press (flash + relabel)
 *           { code, pressed } → a modifier down(true)/up(false) (hold-highlight)
 *
 * Wrapped in an IIFE (classic script sharing global scope with overlay.js).
 */

(() => {
  const { event: tauriEvent, core } = window.__TAURI__;

  // [code, default label, widthUnits?]
  const ROWS = [
    [["Escape", "Esc", 1.5], ["F1", "F1"], ["F2", "F2"], ["F3", "F3"], ["F4", "F4"], ["F5", "F5"], ["F6", "F6"], ["F7", "F7"], ["F8", "F8"], ["F9", "F9"], ["F10", "F10"], ["F11", "F11"], ["F12", "F12"]],
    [["Backquote", "`"], ["Digit1", "1"], ["Digit2", "2"], ["Digit3", "3"], ["Digit4", "4"], ["Digit5", "5"], ["Digit6", "6"], ["Digit7", "7"], ["Digit8", "8"], ["Digit9", "9"], ["Digit0", "0"], ["Minus", "-"], ["Equal", "="], ["Backspace", "⌫", 2]],
    [["Tab", "Tab", 1.5], ["KeyQ", "Q"], ["KeyW", "W"], ["KeyE", "E"], ["KeyR", "R"], ["KeyT", "T"], ["KeyY", "Y"], ["KeyU", "U"], ["KeyI", "I"], ["KeyO", "O"], ["KeyP", "P"], ["BracketLeft", "["], ["BracketRight", "]"], ["Backslash", "\\", 1.5]],
    [["CapsLock", "Caps", 1.75], ["KeyA", "A"], ["KeyS", "S"], ["KeyD", "D"], ["KeyF", "F"], ["KeyG", "G"], ["KeyH", "H"], ["KeyJ", "J"], ["KeyK", "K"], ["KeyL", "L"], ["Semicolon", ";"], ["Quote", "'"], ["Enter", "Enter", 2.25]],
    [["ShiftLeft", "Shift", 2.25], ["KeyZ", "Z"], ["KeyX", "X"], ["KeyC", "C"], ["KeyV", "V"], ["KeyB", "B"], ["KeyN", "N"], ["KeyM", "M"], ["Comma", ","], ["Period", "."], ["Slash", "/"], ["ShiftRight", "Shift", 2.25]],
    [["ControlLeft", "Ctrl", 1.5], ["MetaLeft", "Super", 1.25], ["AltLeft", "Alt", 1.25], ["Space", "", 6.25], ["AltRight", "Alt", 1.25], ["MetaRight", "Super", 1.25], ["ControlRight", "Ctrl", 1.5]],
    [["ArrowLeft", "←"], ["ArrowUp", "↑"], ["ArrowDown", "↓"], ["ArrowRight", "→"]],
  ];

  const FLASH_MS = 180;
  const capById = new Map(); // code -> element
  let keyboard;

  function num(value, fallback) {
    const n = Number(value);
    return Number.isFinite(n) ? n : fallback;
  }

  function build() {
    keyboard.textContent = "";
    capById.clear();
    for (const row of ROWS) {
      const rowEl = document.createElement("div");
      rowEl.className = "kb-row";
      for (const [code, label, width] of row) {
        const key = document.createElement("div");
        key.className = "kb-key";
        key.textContent = label;
        key.style.flexGrow = String(width || 1);
        rowEl.appendChild(key);
        capById.set(code, key);
      }
      keyboard.appendChild(rowEl);
    }
  }

  function relabel(code, label) {
    const el = capById.get(code);
    if (!el || label == null || label === " ") return;
    el.textContent = label.length === 1 ? label.toUpperCase() : label;
  }

  function onKeyFlash(s) {
    if (!s || !s.code) return;
    if (s.label != null) relabel(s.code, s.label);
    const el = capById.get(s.code);
    if (!el) return;

    if (s.pressed === true) {
      clearTimeout(el._flashTimer);
      el.classList.add("active"); // held: stays lit until release
      return;
    }
    if (s.pressed === false) {
      el.classList.remove("active");
      return;
    }
    // Regular key: brief flash.
    el.classList.add("active");
    clearTimeout(el._flashTimer);
    el._flashTimer = setTimeout(() => el.classList.remove("active"), FLASH_MS);
  }

  // Anchor the keyboard per the shared position/offsets (so drag-to-position
  // works), mirroring overlay.js, plus the widget scale.
  function applyPosition(config) {
    const scale = config.deviceWidgetScale || 1;
    const [vertical, horizontal] = (config.position || "top-left").split("-");

    keyboard.style.bottom = "auto";
    if (vertical === "top") {
      keyboard.style.top = `${num(config.topOffset, 0)}px`;
    } else if (vertical === "bottom") {
      keyboard.style.top = "auto";
      keyboard.style.bottom = `${num(config.bottomOffset, 0)}px`;
    } else {
      keyboard.style.top = "50%";
    }

    keyboard.style.right = "auto";
    if (horizontal === "left") {
      keyboard.style.left = `${num(config.leftOffset, 0)}px`;
    } else if (horizontal === "right") {
      keyboard.style.left = "auto";
      keyboard.style.right = `${num(config.rightOffset, 0)}px`;
    } else {
      keyboard.style.left = "50%";
    }

    let anchor = "";
    if (horizontal === "center") anchor = "translateX(-50%)";
    else if (!horizontal) anchor = "translate(-50%, -50%)";
    keyboard.style.transform = `${anchor} scale(${scale})`.trim();
    const vpart = vertical === "bottom" ? "bottom" : vertical === "top" ? "top" : "center";
    const hpart = horizontal === "right" ? "right" : horizontal === "left" ? "left" : "center";
    keyboard.style.transformOrigin = `${vpart} ${hpart}`;
  }

  let lastConfig = null;

  function applyConfig(config) {
    if (!config) return;
    lastConfig = config;
    keyboard.hidden = config.displayStyle !== "keyboard";
    keyboard.style.setProperty("--kb-color", config.popupFontColor || "#ffffff");
    keyboard.style.setProperty("--kb-bg", config.popupBackgroundColor || "#000000");
    applyPosition(config);
  }

  // Pre-label caps from the OS layout so QWERTZ/AZERTY/etc. render correctly
  // right away (no relabel-on-press flicker). Falls back silently if empty.
  async function applyLayout() {
    try {
      const labels = await core.invoke("get_key_labels");
      for (const [code, label] of Object.entries(labels || {})) {
        relabel(code, label);
      }
    } catch {
      // live relabeling still handles it as keys are pressed
    }
  }

  async function init() {
    keyboard = document.getElementById("keyboard");
    build();
    try {
      applyConfig(await core.invoke("get_config"));
    } catch {
      // stays hidden if config can't load
    }
    await applyLayout();
    await tauriEvent.listen("key-flash", (e) => onKeyFlash(e.payload));
    await tauriEvent.listen("config-updated", (e) => {
      applyConfig(e.payload);
      applyLayout(); // re-detect if the layout override changed
    });
    // Leaving move mode (e.g. after Cancel) re-applies the configured position,
    // since overlay.js may have moved the keyboard live during the drag.
    await tauriEvent.listen("overlay-move", (e) => {
      if (!e.payload && lastConfig) applyPosition(lastConfig);
    });
  }

  window.addEventListener("DOMContentLoaded", init);
})();
