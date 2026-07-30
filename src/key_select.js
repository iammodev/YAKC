/**
 * YAKC "pick which keys to show" page: a clickable mock-up of the on-screen
 * keyboard. Each cap toggles whether that physical key appears on the live
 * overlay keyboard. Saved as `keyboardVisibleKeys` (W3C codes); an empty list
 * means "show the whole keyboard", so selecting every key stores [] to stay
 * future-proof as the layout grows.
 */

const { core } = window.__TAURI__;

const ROWS = window.YAKC_KEYBOARD_ROWS || [];
const ALL_CODES = ROWS.flat().map(([code]) => code);

const selected = new Set(); // physical codes currently included
const capById = new Map(); // code -> cap element

function build() {
  const preview = document.getElementById("preview");
  preview.textContent = "";
  for (const row of ROWS) {
    const rowEl = document.createElement("div");
    rowEl.className = "kb-row";
    for (const [code, label, width] of row) {
      const key = document.createElement("button");
      key.type = "button";
      key.className = "kb-key";
      key.textContent = label;
      key.style.flexGrow = String(width || 1);
      key.addEventListener("click", () => toggle(code));
      rowEl.appendChild(key);
      capById.set(code, key);
    }
    preview.appendChild(rowEl);
  }
  render();
}

function toggle(code) {
  if (selected.has(code)) selected.delete(code);
  else selected.add(code);
  render();
}

function render() {
  for (const [code, el] of capById) {
    el.classList.toggle("selected", selected.has(code));
  }
  document.getElementById("count").textContent =
    `${selected.size} of ${ALL_CODES.length} keys shown`;
}

// Relabel caps to the user's real OS layout, matching the live keyboard.
async function applyLayout() {
  try {
    const labels = await core.invoke("get_key_labels");
    for (const [code, label] of Object.entries(labels || {})) {
      const el = capById.get(code);
      if (el && label && label !== " ") {
        el.textContent = label.length === 1 ? label.toUpperCase() : label;
      }
    }
  } catch {
    // defaults are fine if the OS layout can't be read
  }
}

async function save() {
  const status = document.getElementById("status");
  try {
    // Every key selected → store [] ("show all"); otherwise the explicit set,
    // in layout order for a stable, readable config file.
    const list =
      selected.size === ALL_CODES.length ? [] : ALL_CODES.filter((c) => selected.has(c));

    const fullConfig = await core.invoke("get_config");
    fullConfig.keyboardVisibleKeys = list;
    await core.invoke("save_config", { config: fullConfig });
    status.textContent = "Saved ✓";
  } catch (err) {
    status.textContent = `Failed to save: ${err}`;
  }
  clearTimeout(save._timer);
  save._timer = setTimeout(() => (status.textContent = ""), 4000);
}

async function init() {
  let list = [];
  try {
    const config = await core.invoke("get_config");
    list = Array.isArray(config.keyboardVisibleKeys) ? config.keyboardVisibleKeys : [];
  } catch {
    // treat as "show all" if config can't load
  }
  // Empty selection means "show everything" — reflect that as all-selected.
  const initial = list.length ? list : ALL_CODES;
  for (const code of initial) {
    if (ALL_CODES.includes(code)) selected.add(code);
  }

  build();
  await applyLayout();

  document.getElementById("saveBtn").addEventListener("click", save);
  document.getElementById("selectAll").addEventListener("click", () => {
    for (const code of ALL_CODES) selected.add(code);
    render();
  });
  document.getElementById("clearAll").addEventListener("click", () => {
    selected.clear();
    render();
  });
  document.getElementById("invert").addEventListener("click", () => {
    for (const code of ALL_CODES) {
      if (selected.has(code)) selected.delete(code);
      else selected.add(code);
    }
    render();
  });
}

window.addEventListener("DOMContentLoaded", init);
