/**
 * YAKC key label overrides page: lists every key that can be overridden,
 * grouped by category, with an input for each to set a custom label.
 */

const { core } = window.__TAURI__;

let knownKeys = [];
let currentOverrides = {};

function groupLabel(group) {
  switch (group) {
    case "modifier":
      return "Modifier keys (Ctrl / Alt / Shift / Meta)";
    case "editing":
      return "Editing (Backspace, Delete, Space, …)";
    case "navigation":
      return "Navigation (Arrow keys, Home, End, Page, …)";
    case "numpad":
      return "Numpad";
    case "function":
      return "Function keys (F1–F12)";
    case "system":
      return "System (Escape, Caps Lock, Print Screen, …)";
    default:
      return "Other";
  }
}

function buildForm() {
  const form = document.getElementById("form");
  form.textContent = "";

  // Group keys by category, preserving order.
  const groups = {};
  for (const key of knownKeys) {
    (groups[key.group] ??= []).push(key);
  }

  const groupOrder = ["modifier", "editing", "navigation", "numpad", "function", "system", "other"];

  for (const g of groupOrder) {
    const keys = groups[g];
    if (!keys) continue;

    const section = document.createElement("section");
    section.className = "key-group";

    const heading = document.createElement("h2");
    heading.textContent = groupLabel(g);
    section.appendChild(heading);

    for (const key of keys) {
      const row = document.createElement("div");
      row.className = "key-row";

      const idSpan = document.createElement("span");
      idSpan.className = "key-id";
      idSpan.textContent = key.id;
      row.appendChild(idSpan);

      const arrowSpan = document.createElement("span");
      arrowSpan.className = "key-arrow";
      arrowSpan.textContent = "→";
      row.appendChild(arrowSpan);

      const defaultSpan = document.createElement("span");
      defaultSpan.className = "key-default";
      defaultSpan.textContent = key.default_label;
      row.appendChild(defaultSpan);

      const input = document.createElement("input");
      input.type = "text";
      input.className = "key-override";
      input.placeholder = key.default_label;
      input.value = currentOverrides[key.id] || "";
      input.dataset.keyId = key.id;
      row.appendChild(input);

      section.appendChild(row);
    }

    form.appendChild(section);
  }
}

function collectOverrides() {
  const obj = {};
  for (const input of document.querySelectorAll(".key-override")) {
    const val = input.value.trim();
    if (val) {
      obj[input.dataset.keyId] = val;
    }
  }
  return obj;
}

async function save() {
  const status = document.getElementById("status");
  try {
    const overrides = collectOverrides();

    // Fetch current full config, update only keyLabelOverrides, send back.
    const fullConfig = await core.invoke("get_config");
    fullConfig.keyLabelOverrides = overrides;

    await core.invoke("save_config", { config: fullConfig });
    currentOverrides = overrides;
    status.textContent = "Saved ✓";
  } catch (err) {
    status.textContent = `Failed to save: ${err}`;
  }
  clearTimeout(save._timer);
  save._timer = setTimeout(() => (status.textContent = ""), 4000);
}

async function init() {
  try {
    const [keys, config] = await Promise.all([
      core.invoke("get_known_keys"),
      core.invoke("get_config"),
    ]);
    knownKeys = keys;
    currentOverrides = config.keyLabelOverrides || {};
    buildForm();
    document.getElementById("saveBtn").addEventListener("click", save);
  } catch (err) {
    document.getElementById("form").textContent = `Error loading data: ${err}`;
  }
}

window.addEventListener("DOMContentLoaded", init);
