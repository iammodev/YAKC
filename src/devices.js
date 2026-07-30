/**
 * YAKC device widgets: renders mouse-movement (a dot inside a ring) and
 * gamepad sticks/triggers from the `device-state` events the Rust side emits
 * at ~60 Hz. Discrete inputs (scroll, gamepad buttons, mouse buttons) are shown
 * as popups by overlay.js; this file only handles the analog widgets.
 *
 * Wrapped in an IIFE: overlay.js and devices.js are classic scripts sharing one
 * global scope, so top-level `const core` / `tauriEvent` would collide and this
 * file would fail to load.
 */

(() => {
const { event: tauriEvent, core } = window.__TAURI__;

// How much the mouse dot deflects per pixel moved, and the ring/stick radii.
const MOUSE_GAIN = 0.02;
const MOUSE_MAX_OFFSET = 33; // px, matches #mouseRing radius minus dot size
const STICK_MAX_OFFSET = 26; // px, matches .stick radius minus dot size

const flags = { mouse: false, gamepad: false, connected: false };
const params = { sensitivity: 1, decay: 0.4, scale: 1 };

let moveMode = false;
let lastConfig = null;

// Default anchors (opposite bottom corners) when a widget has no saved position,
// so the mouse and gamepad widgets don't stack on each other or the keyboard.
const DEFAULT_POS = {
  mouse: { right: "90px", bottom: "90px" },
  gamepad: { left: "90px", bottom: "90px" },
};

// Mouse dot offset, normalized to -1..1; springs back to center when idle.
let mx = 0;
let my = 0;
// Latest gamepad analog values.
const gp = { lsx: 0, lsy: 0, rsx: 0, rsy: 0, lt: 0, rt: 0 };

let deviceLayer, mouseWidget, gamepadWidget, mouseDot, leftDot, rightDot, ltFill, rtFill;

function clamp01(v) {
  return Math.max(0, Math.min(1, v));
}

function applyVisibility() {
  // In drag-to-position mode, show every enabled widget so it can be placed,
  // regardless of live activity or controller connection.
  const showMouse = flags.mouse;
  const showGamepad = moveMode ? flags.gamepad : flags.gamepad && flags.connected;
  mouseWidget.hidden = !showMouse;
  gamepadWidget.hidden = !showGamepad;
  deviceLayer.hidden = !(showMouse || showGamepad);
}

// Place a widget at its saved position (or a default corner) and apply scale.
function placeWidget(el, id) {
  const pos = lastConfig && lastConfig.widgetPositions && lastConfig.widgetPositions[id];
  el.style.left = el.style.top = el.style.right = el.style.bottom = "auto";
  if (pos && Number.isFinite(pos.x) && Number.isFinite(pos.y)) {
    el.style.left = `${pos.x}px`;
    el.style.top = `${pos.y}px`;
  } else {
    const def = DEFAULT_POS[id];
    for (const k in def) el.style[k] = def[k];
  }
  el.style.transform = `scale(${params.scale})`;
}

function applyWidgetPositions() {
  placeWidget(mouseWidget, "mouse");
  placeWidget(gamepadWidget, "gamepad");
}

function onDeviceState(s) {
  if (!s) return;
  flags.mouse = s.showMouseMovement;
  flags.gamepad = s.showGamepad;
  flags.connected = s.gamepadConnected;
  params.sensitivity = s.sensitivity || 1;
  params.decay = s.decaySeconds > 0 ? s.decaySeconds : 0.0001;

  // Accumulate mouse motion as an impulse toward the movement direction.
  mx = Math.max(-1, Math.min(1, mx + s.mouseDx * MOUSE_GAIN * params.sensitivity));
  my = Math.max(-1, Math.min(1, my + s.mouseDy * MOUSE_GAIN * params.sensitivity));

  gp.lsx = s.lsX;
  gp.lsy = s.lsY;
  gp.rsx = s.rsX;
  gp.rsy = s.rsY;
  gp.lt = s.lt;
  gp.rt = s.rt;

  applyVisibility();
}

/** Config drives initial + toggle-off visibility (device-state only fires on activity). */
function onConfig(c) {
  if (!c) return;
  lastConfig = c;
  flags.mouse = c.showMouseMovement;
  flags.gamepad = c.showGamepad;
  params.scale = c.deviceWidgetScale || 1;
  deviceLayer.style.setProperty("--dev-color", c.popupFontColor || "#ffffff");
  applyWidgetPositions();
  applyVisibility();
}

let lastTs = 0;
function frame(ts) {
  const dt = lastTs ? (ts - lastTs) / 1000 : 0;
  lastTs = ts;

  // Spring the mouse dot back to center with the configured time constant.
  const factor = Math.exp(-dt / params.decay);
  mx *= factor;
  my *= factor;
  if (Math.abs(mx) < 0.001) mx = 0;
  if (Math.abs(my) < 0.001) my = 0;
  mouseDot.style.transform = `translate(${mx * MOUSE_MAX_OFFSET}px, ${my * MOUSE_MAX_OFFSET}px)`;

  // Gamepad: invert Y so pushing the stick up moves the dot up.
  leftDot.style.transform = `translate(${gp.lsx * STICK_MAX_OFFSET}px, ${-gp.lsy * STICK_MAX_OFFSET}px)`;
  rightDot.style.transform = `translate(${gp.rsx * STICK_MAX_OFFSET}px, ${-gp.rsy * STICK_MAX_OFFSET}px)`;
  ltFill.style.height = `${clamp01(gp.lt) * 100}%`;
  rtFill.style.height = `${clamp01(gp.rt) * 100}%`;

  requestAnimationFrame(frame);
}

async function init() {
  deviceLayer = document.getElementById("deviceLayer");
  mouseWidget = document.getElementById("mouseWidget");
  gamepadWidget = document.getElementById("gamepadWidget");
  mouseDot = document.getElementById("mouseDot");
  leftDot = document.querySelector("#leftStick .stick-dot");
  rightDot = document.querySelector("#rightStick .stick-dot");
  ltFill = document.getElementById("ltFill");
  rtFill = document.getElementById("rtFill");

  try {
    onConfig(await core.invoke("get_config"));
  } catch {
    // overlay.js surfaces config errors; widgets just stay hidden.
  }

  await tauriEvent.listen("device-state", (e) => onDeviceState(e.payload));
  await tauriEvent.listen("config-updated", (e) => onConfig(e.payload));
  // Drag-to-position: reveal enabled widgets so they can be placed; on exit,
  // re-place them (undoing any unsaved drag) and restore normal visibility.
  await tauriEvent.listen("overlay-move", (e) => {
    moveMode = !!e.payload;
    applyWidgetPositions();
    applyVisibility();
  });
  requestAnimationFrame(frame);
}

window.addEventListener("DOMContentLoaded", init);
})();
