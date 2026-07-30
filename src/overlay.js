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

// Drag-to-position: the overlay becomes interactive (Rust drops click-through),
// the user drags the actual overlay content to the desired spot, then Saves
// (persists position=top-left + offsets) or Cancels. Only ever runs in the
// native overlay — the OBS browser page never receives the "overlay-move" event.
// In keyboard style the on-screen keyboard is the drag target; otherwise a
// labeled handle stands in for the (often empty) popup stack.
let moveState = null;

function enterMoveMode() {
  if (moveState) return;

  const keyboardEl = document.getElementById("keyboard");
  const useKeyboard = config.displayStyle === "keyboard" && keyboardEl;

  const backdrop = document.createElement("div");
  backdrop.id = "moveBackdrop";

  let handle = null;
  let target;
  if (useKeyboard) {
    // Drag the real keyboard: make it interactive and anchor by top-left.
    target = keyboardEl;
    const rect = target.getBoundingClientRect();
    target.style.pointerEvents = "auto";
    target.style.cursor = "move";
    target.style.zIndex = "12"; // above #moveBackdrop, so it's grabbable
    target.style.transformOrigin = "top left";
    target.style.transform = `scale(${config.deviceWidgetScale || 1})`;
    target.style.right = "auto";
    target.style.bottom = "auto";
    target.style.left = `${rect.left}px`;
    target.style.top = `${rect.top}px`;
  } else {
    handle = document.createElement("div");
    handle.id = "moveHandle";
    handle.textContent = "Drag me — this is where your keys will appear";
    const rect = popupArea.getBoundingClientRect();
    handle.style.left = `${Math.min(Math.max(rect.left || 40, 0), window.innerWidth - 260)}px`;
    handle.style.top = `${Math.min(Math.max(rect.top || 40, 0), window.innerHeight - 80)}px`;
    target = handle;
  }

  const toolbar = document.createElement("div");
  toolbar.id = "moveToolbar";
  const saveBtn = document.createElement("button");
  saveBtn.textContent = "Save position";
  saveBtn.className = "move-save";
  const cancelBtn = document.createElement("button");
  cancelBtn.textContent = "Cancel";
  cancelBtn.className = "move-cancel";
  toolbar.append(saveBtn, cancelBtn);

  let dragging = false;
  let offX = 0;
  let offY = 0;
  const onDown = (e) => {
    dragging = true;
    const rect = target.getBoundingClientRect();
    offX = e.clientX - rect.left;
    offY = e.clientY - rect.top;
    e.preventDefault();
  };
  const onMove = (e) => {
    if (!dragging) return;
    let x = Math.max(e.clientX - offX, 0);
    const y = Math.max(e.clientY - offY, 0);
    // QoL: snap to horizontal center when within a few px.
    const w = target.getBoundingClientRect().width;
    const centeredX = (window.innerWidth - w) / 2;
    if (Math.abs(x - centeredX) < 15) x = centeredX;
    target.style.left = `${x}px`;
    target.style.top = `${y}px`;
  };
  const onUp = () => {
    dragging = false;
  };

  target.addEventListener("mousedown", onDown);
  window.addEventListener("mousemove", onMove);
  window.addEventListener("mouseup", onUp);

  saveBtn.addEventListener("click", async () => {
    const rect = target.getBoundingClientRect();
    config.position = "top-left";
    config.leftOffset = Math.round(rect.left);
    config.topOffset = Math.round(rect.top);
    config.rightOffset = 0;
    config.bottomOffset = 0;
    try {
      await core.invoke("save_config", { config });
    } catch {
      // config-updated won't fire; leaving move mode still restores state.
    }
    await core.invoke("end_overlay_move");
  });
  cancelBtn.addEventListener("click", () => core.invoke("end_overlay_move"));

  document.body.append(backdrop, toolbar);
  if (handle) document.body.appendChild(handle);
  moveState = { backdrop, handle, toolbar, target, onDown, onMove, onUp, useKeyboard };
}

function exitMoveMode() {
  if (!moveState) return;
  window.removeEventListener("mousemove", moveState.onMove);
  window.removeEventListener("mouseup", moveState.onUp);
  moveState.target.removeEventListener("mousedown", moveState.onDown);
  moveState.backdrop.remove();
  if (moveState.handle) moveState.handle.remove();
  moveState.toolbar.remove();
  if (moveState.useKeyboard) {
    // Restore click-through; keyboard.js re-applies its configured position
    // (also handles the Cancel case, where nothing was saved).
    moveState.target.style.pointerEvents = "none";
    moveState.target.style.cursor = "";
    moveState.target.style.zIndex = "";
  }
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
