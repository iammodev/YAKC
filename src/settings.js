/**
 * YAKC settings window: a plain form over every config field.
 * Saving persists config.json and live-applies to the overlay.
 */

const { core } = window.__TAURI__;

// Field schema: key must match the config JSON keys (camelCase).
const SECTIONS = [
  {
    title: "Appearance",
    fields: [
      { key: "popupFontSize", type: "number", label: "Font size (px)", min: 6 },
      { key: "popupFontFamily", type: "text", label: "Font family" },
      { key: "popupFontWeight", type: "select", label: "Font weight", options: ["normal", "bold", "bolder", "lighter"] },
      { key: "popupFontColor", type: "color", label: "Font color" },
      { key: "popupBackgroundColor", type: "color", label: "Background color" },
      { key: "popupOpacity", type: "number", label: "Opacity (0–1)", min: 0, max: 1, step: 0.05 },
      { key: "popupBorderRadius", type: "number", label: "Corner radius (px)", min: 0 },
      { key: "popupTextMaxWidthInPercentage", type: "number", label: "Max width (% of screen)", min: 5, max: 100 },
    ],
  },
  {
    title: "Position",
    fields: [
      { key: "showOnMonitor", type: "number", label: "Monitor index", hint: "0 = first monitor", min: 0, step: 1 },
      { key: "position", type: "select", label: "Screen corner", options: ["top-left", "top-right", "bottom-left", "bottom-right"] },
      { key: "topOffset", type: "number", label: "Top offset (px)" },
      { key: "bottomOffset", type: "number", label: "Bottom offset (px)" },
      { key: "leftOffset", type: "number", label: "Left offset (px)" },
      { key: "rightOffset", type: "number", label: "Right offset (px)" },
    ],
  },
  {
    title: "Timing",
    fields: [
      { key: "popupFadeInSeconds", type: "number", label: "Fade duration (s)", min: 0, step: 0.1 },
      { key: "popupRemoveAfterSeconds", type: "number", label: "Remove popup after (s)", min: 0.1, step: 0.1 },
      { key: "popupInactiveAfterSeconds", type: "number", label: "New popup after inactivity (s)", min: 0, step: 0.1 },
    ],
  },
  {
    title: "Input",
    fields: [
      { key: "displayMode", type: "select", label: "Display mode", options: ["text", "raw"], hint: "text: like a text editor — only typed characters, Backspace deletes. raw: every key (modifiers, ⌫, arrows, …)" },
      { key: "showKeyboardClick", type: "bool", label: "Show keyboard clicks" },
      { key: "showMouseClick", type: "bool", label: "Show mouse clicks" },
      { key: "showMouseCoordinates", type: "bool", label: "Show mouse coordinates", hint: "Not available on Wayland (the compositor hides the cursor position)" },
      { key: "onlyKeysWithModifiers", type: "bool", label: "Only keys with modifiers", hint: "Raw mode only: show a key only when Ctrl/Alt/Meta is held" },
      { key: "showSpaceAsUnicode", type: "bool", label: "Show space as ␣" },
      { key: "textToSymbols", type: "bool", label: "Special keys as symbols", hint: "e.g. Tab → ↹, Backspace → ⌫" },
      { key: "toggleCaptureHotkey", type: "text", label: "Toggle-capture hotkey", hint: "e.g. Ctrl+Alt+Y" },
      { key: "keyboardLayout", type: "text", label: "Keyboard layout override", hint: "Linux only, empty = auto-detect (e.g. us, de, tr); needs restart" },
    ],
  },
  {
    title: "Text-to-speech",
    fields: [
      { key: "textToSpeech", type: "bool", label: "Speak every keystroke" },
      { key: "textToSpeechCancelSpeechOnNewKey", type: "bool", label: "Cancel speech on new key" },
    ],
  },
  {
    title: "Process filter",
    fields: [
      { key: "filter", type: "bool", label: "Enable process filter", hint: "Capture only while a listed app is focused" },
      { key: "filterProcessName", type: "list", label: "Process names", hint: "Comma-separated, e.g. obs.exe, code" },
      { key: "filterCheckEverySecond", type: "number", label: "Check interval (s)", min: 0.1, step: 0.1 },
    ],
  },
];

let config;

function fieldId(key) {
  return `field-${key}`;
}

function buildForm() {
  const form = document.getElementById("form");
  form.textContent = "";

  for (const section of SECTIONS) {
    const heading = document.createElement("h2");
    heading.textContent = section.title;
    form.appendChild(heading);

    for (const field of section.fields) {
      const row = document.createElement("div");
      row.className = "field";

      const label = document.createElement("label");
      label.htmlFor = fieldId(field.key);
      label.textContent = field.label;
      if (field.hint) {
        const hint = document.createElement("span");
        hint.className = "hint";
        hint.textContent = field.hint;
        label.appendChild(hint);
      }
      row.appendChild(label);

      let input;
      if (field.type === "select") {
        input = document.createElement("select");
        for (const option of field.options) {
          const el = document.createElement("option");
          el.value = option;
          el.textContent = option;
          input.appendChild(el);
        }
        input.value = config[field.key];
      } else {
        input = document.createElement("input");
        switch (field.type) {
          case "bool":
            input.type = "checkbox";
            input.checked = Boolean(config[field.key]);
            break;
          case "number":
            input.type = "number";
            if (field.min !== undefined) input.min = field.min;
            if (field.max !== undefined) input.max = field.max;
            input.step = field.step ?? "any";
            input.value = config[field.key];
            break;
          case "color":
            input.type = "color";
            input.value = config[field.key];
            break;
          case "list":
            input.type = "text";
            input.value = (config[field.key] || []).join(", ");
            break;
          default:
            input.type = "text";
            input.value = config[field.key];
        }
      }
      input.id = fieldId(field.key);
      input.dataset.type = field.type;
      input.dataset.key = field.key;
      row.appendChild(input);
      form.appendChild(row);
    }
  }
}

function collectForm() {
  const updated = { ...config };
  for (const input of document.querySelectorAll("[data-key]")) {
    const key = input.dataset.key;
    switch (input.dataset.type) {
      case "bool":
        updated[key] = input.checked;
        break;
      case "number":
        updated[key] = Number(input.value) || 0;
        break;
      case "list":
        updated[key] = input.value
          .split(",")
          .map((s) => s.trim())
          .filter((s) => s.length > 0);
        break;
      default:
        updated[key] = input.value;
    }
  }
  return updated;
}

async function save() {
  const status = document.getElementById("status");
  try {
    config = collectForm();
    await core.invoke("save_config", { config });
    status.textContent = "Saved ✓";
  } catch (err) {
    status.textContent = `Failed to save: ${err}`;
  }
  clearTimeout(save._timer);
  save._timer = setTimeout(() => (status.textContent = ""), 4000);
}

async function init() {
  config = await core.invoke("get_config");
  try {
    document.getElementById("configPath").textContent = await core.invoke("get_config_path");
  } catch {
    document.getElementById("configPath").textContent = "config.json";
  }
  buildForm();
  document.getElementById("saveBtn").addEventListener("click", save);
}

window.addEventListener("DOMContentLoaded", init);
