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

  // Physical layout: [code, default label, widthUnits?]. Shared with the
  // key-selector via keyboard-layout.js so the two never drift.
  const ROWS = window.YAKC_KEYBOARD_ROWS || [];

  const FLASH_MS = 180;
  const capById = new Map(); // code -> element
  let keyboard;
  let builtSignature = null; // which visible-key set is currently rendered

  function num(value, fallback) {
    const n = Number(value);
    return Number.isFinite(n) ? n : fallback;
  }

  // Labels for otherwise-blank caps, shown only in compact mode where the wide
  // Space bar needs a glyph to read as a key.
  const COMPACT_LABEL = { Space: "␣" };

  // Keys at least this wide (only Space) stretch to fill to their row's right
  // edge instead of being a lone square.
  const STRETCH_MIN_UNITS = 3;

  // Build the keyboard. With no selection the whole keyboard renders (flex rows,
  // as designed). With a selection we switch to a compact grid of uniform square
  // caps, packed tightly, so a trimmed keyboard has no wide-key gaps and no
  // empty panel — just the keys you picked.
  function build(visible) {
    keyboard.textContent = "";
    capById.clear();
    keyboard.classList.toggle("kb-grid", Boolean(visible));
    if (visible) buildTrimmed(visible);
    else buildFull();
  }

  function buildFull() {
    for (const row of ROWS) {
      const rowEl = document.createElement("div");
      rowEl.className = "kb-row";
      if (row.some(([code]) => code.startsWith("Arrow"))) rowEl.classList.add("kb-arrows");
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

  function buildTrimmed(visible) {
    // Collect selected caps tagged with their column INDEX in the original row
    // (aligning by index, not pixel width, so Tab & Caps are both "column 0",
    // Q & A both "column 1", …).
    const selected = [];
    const usedCols = new Set();
    for (const row of ROWS) {
      row.forEach(([code, label, width], index) => {
        if (visible.has(code)) {
          selected.push({ code, label, index, wide: (width || 1) >= STRETCH_MIN_UNITS });
          usedCols.add(index);
        }
      });
    }
    if (!selected.length) return;

    // Horizontal: drop columns nobody uses so unselected keys leave no gap.
    const colMap = new Map([...usedCols].sort((a, b) => a - b).map((c, i) => [c, i]));
    const cols = colMap.size;

    // Vertical gravity: each column stacks its caps from the top, so an empty
    // slot above (e.g. an unselected Caps over Shift) is removed and rows merge
    // when they don't collide. A cap's visual row = how many selected caps are
    // above it in the same column. Preserves "A under Q" (same column) while
    // pulling everything as tight as it goes.
    const depth = new Map();
    const rows = [];
    for (const p of selected) {
      const r = depth.get(p.index) || 0;
      depth.set(p.index, r + 1);
      (rows[r] ||= []).push(p);
    }
    keyboard.style.setProperty("--kb-cols", String(cols));

    rows.forEach((picks, rowIndex) => {
      picks.sort((a, b) => colMap.get(a.index) - colMap.get(b.index));
      picks.forEach((p, i) => {
        const key = document.createElement("div");
        key.className = "kb-key";
        key.textContent = p.label || COMPACT_LABEL[p.code] || "";
        let start = colMap.get(p.index) + 1; // 1-based grid line
        let span = 1;
        // A trailing wide key (Space) packs against the previous cap and fills to
        // the right edge, reading as a spacebar rather than a lone square.
        if (p.wide && i === picks.length - 1) {
          const prev = i > 0 ? colMap.get(picks[i - 1].index) + 1 : 0;
          start = prev + 1;
          span = Math.max(1, cols - prev);
        }
        key.style.gridRow = String(rowIndex + 1);
        key.style.gridColumn = `${start} / span ${span}`;
        keyboard.appendChild(key);
        capById.set(p.code, key);
      });
    });
  }

  // A stable signature of the visible-key selection, so we only rebuild the DOM
  // (and re-fetch layout labels) when the selection actually changes.
  function visibleFrom(config) {
    const list = Array.isArray(config.keyboardVisibleKeys) ? config.keyboardVisibleKeys : [];
    return list.length ? new Set(list) : null; // null = show everything
  }

  function signatureOf(visible) {
    return visible ? [...visible].sort().join(",") : "*";
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

  // Returns true if the keyboard DOM was rebuilt (selection changed), so the
  // caller knows to re-fetch OS layout labels for the freshly created caps.
  function applyConfig(config) {
    if (!config) return false;
    lastConfig = config;
    keyboard.hidden = config.displayStyle !== "keyboard";
    keyboard.style.setProperty("--kb-color", config.popupFontColor || "#ffffff");
    keyboard.style.setProperty("--kb-bg", config.popupBackgroundColor || "#000000");

    const visible = visibleFrom(config);
    const signature = signatureOf(visible);
    let rebuilt = false;
    if (signature !== builtSignature) {
      build(visible);
      builtSignature = signature;
      rebuilt = true;
    }
    applyPosition(config);
    return rebuilt;
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
    try {
      applyConfig(await core.invoke("get_config"));
    } catch {
      build(null); // config unavailable: show the full keyboard, stays hidden
    }
    await applyLayout();
    await tauriEvent.listen("key-flash", (e) => onKeyFlash(e.payload));
    await tauriEvent.listen("config-updated", (e) => {
      applyConfig(e.payload);
      applyLayout(); // re-detect if the layout override / visible-key set changed
    });
    // Leaving move mode (e.g. after Cancel) re-applies the configured position,
    // since overlay.js may have moved the keyboard live during the drag.
    await tauriEvent.listen("overlay-move", (e) => {
      if (!e.payload && lastConfig) applyPosition(lastConfig);
    });
  }

  window.addEventListener("DOMContentLoaded", init);
})();
