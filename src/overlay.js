/**
 * YAKC overlay renderer: applies popup operations from the Rust side
 * ({op:"append"|"delete"|"repeat"}) to fading popups anchored to the
 * configured screen corner. Popup content is a list of tokens so held keys
 * render as "a (x13)" and Backspace can really delete.
 */

const { event: tauriEvent, core } = window.__TAURI__;

let config;
let lastKeyTime = 0;
let popupArea;
let currentPopup = null;

const MAX_POPUPS = 5;

function num(value, fallback) {
  const n = Number(value);
  return Number.isFinite(n) ? n : fallback;
}

/** Apply config-driven styles to the popup area and (re)build the popup CSS. */
function applyConfigStyles() {
  let style = document.getElementById("configStyle");
  if (!style) {
    style = document.createElement("style");
    style.id = "configStyle";
    document.head.appendChild(style);
  }
  style.textContent = `
    .popup {
      font-family: ${config.popupFontFamily};
      font-weight: ${config.popupFontWeight};
      background-color: ${config.popupBackgroundColor};
      color: ${config.popupFontColor};
      font-size: ${num(config.popupFontSize, 20)}px;
      transition: opacity ${num(config.popupFadeInSeconds, 0.5)}s ease-in-out;
      max-width: ${num(config.popupTextMaxWidthInPercentage, 60)}vw;
      border-radius: ${num(config.popupBorderRadius, 10)}px;
    }
    .popup.active {
      opacity: ${num(config.popupOpacity, 0.9)};
    }
  `;

  // Anchor the popup stack to the configured position with the configured offsets.
  const position = config.position || "top-left";
  const [vertical, horizontal] = position.split("-");

  // Vertical anchoring
  popupArea.style.bottom = "auto";
  if (vertical === "top") {
    popupArea.style.top = `${num(config.topOffset, 0)}px`;
  } else if (vertical === "bottom") {
    popupArea.style.top = "auto";
    popupArea.style.bottom = `${num(config.bottomOffset, 0)}px`;
  } else {
    // center — vertically centered
    popupArea.style.top = "50%";
  }

  // Horizontal anchoring
  popupArea.style.right = "auto";
  if (horizontal === "left") {
    popupArea.style.left = `${num(config.leftOffset, 0)}px`;
    popupArea.style.alignItems = "flex-start";
  } else if (horizontal === "right") {
    popupArea.style.left = "auto";
    popupArea.style.right = `${num(config.rightOffset, 0)}px`;
    popupArea.style.alignItems = "flex-end";
  } else {
    // center (or full center with no horizontal component)
    popupArea.style.left = "50%";
    popupArea.style.alignItems = "center";
  }

  // Transform: full center offsets both axes, edge-center offsets only X
  if (horizontal === "center") {
    popupArea.style.transform = "translateX(-50%)";
  } else if (!horizontal) {
    popupArea.style.transform = "translate(-50%, -50%)";
  } else {
    popupArea.style.transform = "none";
  }

  // Keyboard-skin mode hides the popup stack (the keyboard renders instead).
  popupArea.style.display = config.displayStyle === "keyboard" ? "none" : "flex";
}

function renderPopup(popup) {
  popup.textContent = popup._tokens
    .map((t) => (t.count > 1 ? `${t.text} (x${t.count})` : t.text))
    .join("");
}

function createPopup() {
  const popup = document.createElement("div");
  popup.classList.add("popup");
  popup._tokens = [];
  popupArea.appendChild(popup);

  // Add .active on the next frame so the opacity transition (fade-in) runs.
  requestAnimationFrame(() => popup.classList.add("active"));
  armRemoveTimer(popup);

  const popups = popupArea.querySelectorAll(".popup");
  if (popups.length > MAX_POPUPS) {
    removePopup(popups[0]);
  }
  return popup;
}

function armRemoveTimer(popup) {
  clearTimeout(popup._removeTimer);
  popup._removeTimer = setTimeout(
    () => removePopup(popup),
    num(config.popupRemoveAfterSeconds, 3) * 1000
  );
}

function removePopup(popup) {
  if (popup._removing) return;
  popup._removing = true;
  clearTimeout(popup._removeTimer);
  popup.classList.remove("active");
  popup.addEventListener("transitionend", () => popup.remove(), { once: true });
  // Safety net in case the transition never fires (e.g. popup was never painted).
  setTimeout(() => popup.remove(), (num(config.popupFadeInSeconds, 0.5) + 0.5) * 1000);
  if (currentPopup === popup) currentPopup = null;
}

function onPopupOp(payload) {
  if (!config || !payload || !payload.op) return;
  // In keyboard-skin mode the on-screen keyboard (keyboard.js) renders instead.
  if (config.displayStyle === "keyboard") return;

  const now = Date.now();
  const inactiveMs = num(config.popupInactiveAfterSeconds, 0.5) * 1000;
  const haveCurrent =
    currentPopup && !currentPopup._removing && now - lastKeyTime <= inactiveMs;

  switch (payload.op) {
    case "append": {
      if (!haveCurrent) {
        currentPopup = createPopup();
      }
      currentPopup._tokens.push({ text: payload.text, count: 1 });
      break;
    }
    case "delete": {
      // Text-editor behavior: remove the last token from the current popup.
      if (!haveCurrent || currentPopup._tokens.length === 0) return;
      const last = currentPopup._tokens[currentPopup._tokens.length - 1];
      if (last.count > 1) {
        last.count -= 1;
      } else {
        currentPopup._tokens.pop();
      }
      if (currentPopup._tokens.length === 0) {
        lastKeyTime = now;
        removePopup(currentPopup);
        return;
      }
      break;
    }
    case "repeat": {
      // A held key: bump the "(xN)" counter of the last token.
      if (!haveCurrent || currentPopup._tokens.length === 0) return;
      currentPopup._tokens[currentPopup._tokens.length - 1].count += 1;
      break;
    }
    default:
      return;
  }

  renderPopup(currentPopup);
  armRemoveTimer(currentPopup);
  lastKeyTime = now;
}

function showNotice(message) {
  const notice = document.getElementById("notice");
  notice.textContent = message;
  notice.hidden = false;
  clearTimeout(notice._timer);
  notice._timer = setTimeout(() => (notice.hidden = true), 30000);
}

// Drag-to-position: the overlay becomes interactive (Rust drops click-through)
// and EVERY visible overlay object — the keys display (on-screen keyboard or a
// popup handle), the mouse widget and the gamepad widget — becomes an
// independently draggable target with a name tag and a hover highlight, so
// overlapping widgets can be told apart and moved apart. Save persists each
// object's position; the keys display uses position=top-left + offsets, the
// device widgets use config.widgetPositions[id]. Only ever runs in the native
// overlay — the OBS browser page never receives the "overlay-move" event.
let moveState = null;

// Makes one element a draggable move-target: adds the outline/label, converts it
// to left/top so it can be moved freely, and wires drag + hover handlers.
function makeMoveTarget(el, kind, label, state) {
  const rect = el.getBoundingClientRect();
  el.classList.add("move-target");
  el.dataset.moveLabel = label;
  el.style.pointerEvents = "auto";
  el.style.cursor = "move";
  el.style.zIndex = "13";
  el.style.right = "auto";
  el.style.bottom = "auto";
  el.style.left = `${rect.left}px`;
  el.style.top = `${rect.top}px`;

  const onDown = (e) => {
    const r = el.getBoundingClientRect();
    state.active = el;
    state.offX = e.clientX - r.left;
    state.offY = e.clientY - r.top;
    e.preventDefault();
    e.stopPropagation();
  };
  const onEnter = () => el.classList.add("move-hover");
  const onLeave = () => el.classList.remove("move-hover");
  el.addEventListener("mousedown", onDown);
  el.addEventListener("mouseenter", onEnter);
  el.addEventListener("mouseleave", onLeave);
  state.targets.push({ el, kind, onDown, onEnter, onLeave });
}

function enterMoveMode() {
  if (moveState) return;
  const state = { targets: [], active: null, offX: 0, offY: 0 };

  const backdrop = document.createElement("div");
  backdrop.id = "moveBackdrop";
  document.body.appendChild(backdrop);

  // The keys display: the real keyboard, or a labeled handle for the popup stack.
  const keyboardEl = document.getElementById("keyboard");
  let handle = null;
  if (config.displayStyle === "keyboard" && keyboardEl) {
    keyboardEl.style.transformOrigin = "top left";
    keyboardEl.style.transform = `scale(${config.deviceWidgetScale || 1})`;
    makeMoveTarget(keyboardEl, "keys", "Keyboard", state);
  } else {
    handle = document.createElement("div");
    handle.id = "moveHandle";
    handle.textContent = "Keys / popups appear here";
    const rect = popupArea.getBoundingClientRect();
    handle.style.left = `${Math.min(Math.max(rect.left || 40, 0), window.innerWidth - 240)}px`;
    handle.style.top = `${Math.min(Math.max(rect.top || 40, 0), window.innerHeight - 80)}px`;
    document.body.appendChild(handle);
    makeMoveTarget(handle, "keys", "Keys / popups", state);
  }

  // Device widgets as separate targets. Force them visible (devices.js also does
  // this on the overlay-move event, but not depending on listener order lets us
  // read their real position) before grabbing them.
  const deviceLayer = document.getElementById("deviceLayer");
  if (config.showMouseMovement) {
    const el = document.getElementById("mouseWidget");
    if (el) {
      deviceLayer.hidden = false;
      el.hidden = false;
      makeMoveTarget(el, "mouse", "Mouse", state);
    }
  }
  if (config.showGamepad) {
    const el = document.getElementById("gamepadWidget");
    if (el) {
      deviceLayer.hidden = false;
      el.hidden = false;
      makeMoveTarget(el, "gamepad", "Gamepad", state);
    }
  }

  // Alignment guides: snap the dragged object's edges/center to the other
  // objects' edges/centers and the screen center, drawing a guide line on snap.
  state.snap = config.snapToGuides !== false;
  const guideV = document.createElement("div");
  guideV.className = "move-guide v";
  guideV.hidden = true;
  const guideH = document.createElement("div");
  guideH.className = "move-guide h";
  guideH.hidden = true;
  document.body.append(guideV, guideH);
  state.guideV = guideV;
  state.guideH = guideH;

  const onMove = (e) => {
    if (!state.active) return;
    const rect = state.active.getBoundingClientRect();
    let x = Math.max(e.clientX - state.offX, 0);
    let y = Math.max(e.clientY - state.offY, 0);

    let lineX = null;
    let lineY = null;
    if (state.snap) {
      const SNAP = 8;
      // Candidate lines from every other object + the screen center.
      const vLines = [window.innerWidth / 2];
      const hLines = [window.innerHeight / 2];
      for (const t of state.targets) {
        if (t.el === state.active) continue;
        const r = t.el.getBoundingClientRect();
        vLines.push(r.left, r.left + r.width / 2, r.right);
        hLines.push(r.top, r.top + r.height / 2, r.bottom);
      }
      // Snap the nearest of {left, center, right} to the nearest vertical line.
      let bestX = SNAP + 1;
      for (const line of vLines) {
        for (const anchor of [x, x + rect.width / 2, x + rect.width]) {
          const d = Math.abs(anchor - line);
          if (d < bestX) {
            bestX = d;
            x += line - anchor;
            lineX = line;
          }
        }
      }
      let bestY = SNAP + 1;
      for (const line of hLines) {
        for (const anchor of [y, y + rect.height / 2, y + rect.height]) {
          const d = Math.abs(anchor - line);
          if (d < bestY) {
            bestY = d;
            y += line - anchor;
            lineY = line;
          }
        }
      }
    }

    state.active.style.left = `${x}px`;
    state.active.style.top = `${y}px`;
    guideV.hidden = lineX === null;
    if (lineX !== null) guideV.style.left = `${lineX}px`;
    guideH.hidden = lineY === null;
    if (lineY !== null) guideH.style.top = `${lineY}px`;
  };
  const onUp = () => {
    state.active = null;
    guideV.hidden = true;
    guideH.hidden = true;
  };
  state.onMove = onMove;
  state.onUp = onUp;
  window.addEventListener("mousemove", onMove);
  window.addEventListener("mouseup", onUp);

  const toolbar = document.createElement("div");
  toolbar.id = "moveToolbar";
  const snapBtn = document.createElement("button");
  snapBtn.className = "move-snap";
  const updateSnapBtn = () => {
    snapBtn.textContent = `Snapping: ${state.snap ? "On" : "Off"}`;
    snapBtn.classList.toggle("off", !state.snap);
  };
  updateSnapBtn();
  snapBtn.addEventListener("click", () => {
    state.snap = !state.snap;
    updateSnapBtn();
    if (!state.snap) {
      guideV.hidden = true;
      guideH.hidden = true;
    }
  });
  const saveBtn = document.createElement("button");
  saveBtn.textContent = "Save positions";
  saveBtn.className = "move-save";
  const cancelBtn = document.createElement("button");
  cancelBtn.textContent = "Cancel";
  cancelBtn.className = "move-cancel";
  toolbar.append(snapBtn, saveBtn, cancelBtn);
  document.body.appendChild(toolbar);

  saveBtn.addEventListener("click", async () => {
    const positions = { ...(config.widgetPositions || {}) };
    for (const t of state.targets) {
      const rect = t.el.getBoundingClientRect();
      if (t.kind === "keys") {
        config.position = "top-left";
        config.leftOffset = Math.round(rect.left);
        config.topOffset = Math.round(rect.top);
        config.rightOffset = 0;
        config.bottomOffset = 0;
      } else {
        positions[t.kind] = { x: Math.round(rect.left), y: Math.round(rect.top) };
      }
    }
    config.widgetPositions = positions;
    config.snapToGuides = state.snap;
    try {
      await core.invoke("save_config", { config });
    } catch {
      // config-updated won't fire; leaving move mode still restores state.
    }
    await core.invoke("end_overlay_move");
  });
  cancelBtn.addEventListener("click", () => core.invoke("end_overlay_move"));

  moveState = { backdrop, handle, toolbar, state };
}

function exitMoveMode() {
  if (!moveState) return;
  const { backdrop, handle, toolbar, state } = moveState;
  window.removeEventListener("mousemove", state.onMove);
  window.removeEventListener("mouseup", state.onUp);
  for (const t of state.targets) {
    t.el.removeEventListener("mousedown", t.onDown);
    t.el.removeEventListener("mouseenter", t.onEnter);
    t.el.removeEventListener("mouseleave", t.onLeave);
    t.el.classList.remove("move-target", "move-hover");
    delete t.el.dataset.moveLabel;
    // Reset interaction styles; keyboard.js / devices.js re-apply real positions
    // on the config-updated (save) or overlay-move (cancel) events.
    t.el.style.pointerEvents = "";
    t.el.style.cursor = "";
    t.el.style.zIndex = "";
  }
  if (state.guideV) state.guideV.remove();
  if (state.guideH) state.guideH.remove();
  backdrop.remove();
  if (handle) handle.remove();
  toolbar.remove();
  moveState = null;
}

async function init() {
  popupArea = document.getElementById("popupArea");
  config = await core.invoke("get_config");
  applyConfigStyles();

  await tauriEvent.listen("click-event", (e) => onPopupOp(e.payload));
  await tauriEvent.listen("config-updated", (e) => {
    config = e.payload;
    applyConfigStyles();
  });
  await tauriEvent.listen("yakc-error", (e) => showNotice(e.payload));
  await tauriEvent.listen("overlay-move", (e) =>
    e.payload ? enterMoveMode() : exitMoveMode()
  );

  // Errors raised before this page was listening (e.g. missing input-device
  // permission detected during the first device scan).
  const pending = await core.invoke("get_pending_errors");
  if (pending.length > 0) showNotice(pending[pending.length - 1]);
}

window.addEventListener("DOMContentLoaded", init);
