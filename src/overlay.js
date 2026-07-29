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

  // Errors raised before this page was listening (e.g. missing input-device
  // permission detected during the first device scan).
  const pending = await core.invoke("get_pending_errors");
  if (pending.length > 0) showNotice(pending[pending.length - 1]);
}

window.addEventListener("DOMContentLoaded", init);
