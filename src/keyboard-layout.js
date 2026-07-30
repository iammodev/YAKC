/**
 * Shared physical-keyboard layout for YAKC's on-screen keyboard.
 *
 * Single source of truth for both the live overlay (keyboard.js) and the
 * "pick which keys to show" selector (key_select.js), so the two can never
 * drift out of sync. Keyed by PHYSICAL key position (W3C KeyboardEvent.code),
 * so the right cap lights up on any layout (QWERTY/QWERTZ/AZERTY/…).
 *
 * Each entry: [code, default label, widthUnits?]
 */
window.YAKC_KEYBOARD_ROWS = [
  [["Escape", "Esc", 1.5], ["F1", "F1"], ["F2", "F2"], ["F3", "F3"], ["F4", "F4"], ["F5", "F5"], ["F6", "F6"], ["F7", "F7"], ["F8", "F8"], ["F9", "F9"], ["F10", "F10"], ["F11", "F11"], ["F12", "F12"]],
  [["Backquote", "`"], ["Digit1", "1"], ["Digit2", "2"], ["Digit3", "3"], ["Digit4", "4"], ["Digit5", "5"], ["Digit6", "6"], ["Digit7", "7"], ["Digit8", "8"], ["Digit9", "9"], ["Digit0", "0"], ["Minus", "-"], ["Equal", "="], ["Backspace", "⌫", 2]],
  [["Tab", "Tab", 1.5], ["KeyQ", "Q"], ["KeyW", "W"], ["KeyE", "E"], ["KeyR", "R"], ["KeyT", "T"], ["KeyY", "Y"], ["KeyU", "U"], ["KeyI", "I"], ["KeyO", "O"], ["KeyP", "P"], ["BracketLeft", "["], ["BracketRight", "]"], ["Backslash", "\\", 1.5]],
  [["CapsLock", "Caps", 1.75], ["KeyA", "A"], ["KeyS", "S"], ["KeyD", "D"], ["KeyF", "F"], ["KeyG", "G"], ["KeyH", "H"], ["KeyJ", "J"], ["KeyK", "K"], ["KeyL", "L"], ["Semicolon", ";"], ["Quote", "'"], ["Enter", "Enter", 2.25]],
  [["ShiftLeft", "Shift", 2.25], ["KeyZ", "Z"], ["KeyX", "X"], ["KeyC", "C"], ["KeyV", "V"], ["KeyB", "B"], ["KeyN", "N"], ["KeyM", "M"], ["Comma", ","], ["Period", "."], ["Slash", "/"], ["ShiftRight", "Shift", 2.25]],
  [["ControlLeft", "Ctrl", 1.5], ["MetaLeft", "Super", 1.25], ["AltLeft", "Alt", 1.25], ["Space", "", 6.25], ["AltRight", "Alt", 1.25], ["MetaRight", "Super", 1.25], ["ControlRight", "Ctrl", 1.5]],
  [["ArrowLeft", "←"], ["ArrowUp", "↑"], ["ArrowDown", "↓"], ["ArrowRight", "→"]],
];
