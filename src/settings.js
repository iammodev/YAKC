/**
 * YAKC settings window: a tabbed form over every config field.
 * Saving persists config.json and live-applies to the overlay.
 */

const { core, event: tauriEvent } = window.__TAURI__;

const PROFILES_TAB = "Profiles";

// Cross-platform-safe font stacks offered in the font picker (free text still
// allowed via the combobox).
const FONT_CHOICES = [
  "Tahoma, sans-serif",
  "Arial, sans-serif",
  "Helvetica, sans-serif",
  "Verdana, sans-serif",
  "Segoe UI, sans-serif",
  "system-ui, sans-serif",
  "Georgia, serif",
  "Times New Roman, serif",
  "Courier New, monospace",
  "Consolas, monospace",
  "Menlo, monospace",
  "Impact, sans-serif",
];

// Quick color presets shown as swatches beside each color input.
const COLOR_SWATCHES = [
  "#ffffff", "#000000", "#ff5555", "#ffb86c", "#f1fa8c",
  "#50fa7b", "#8be9fd", "#bd93f9", "#ff79c6", "#6272a4",
];

// Field schema: key must match the config JSON keys (camelCase). `tab` groups
// sections into the tab bar (several sections can share one tab).
const SECTIONS = [
  {
    tab: "Appearance",
    title: "Appearance",
    fields: [
      { key: "popupFontSize", type: "number", label: "Font size (px)", min: 6 },
      { key: "popupFontFamily", type: "combobox", label: "Font family", options: FONT_CHOICES, hint: "Pick one or type any CSS font stack" },
      { key: "popupFontWeight", type: "select", label: "Font weight", options: ["normal", "bold", "bolder", "lighter"] },
      { key: "popupFontColor", type: "color", label: "Font color" },
      { key: "popupBackgroundColor", type: "color", label: "Background color" },
      { key: "popupOpacity", type: "number", label: "Opacity (0–1)", min: 0, max: 1, step: 0.05 },
      { key: "popupBorderRadius", type: "number", label: "Corner radius (px)", min: 0 },
      { key: "popupTextMaxWidthInPercentage", type: "number", label: "Max width (% of screen)", min: 5, max: 100 },
    ],
  },
  {
    tab: "Appearance",
    title: "Timing",
    fields: [
      { key: "popupFadeInSeconds", type: "number", label: "Fade duration (s)", min: 0, step: 0.1 },
      { key: "popupRemoveAfterSeconds", type: "number", label: "Remove popup after (s)", min: 0.1, step: 0.1 },
      { key: "popupInactiveAfterSeconds", type: "number", label: "New popup after inactivity (s)", min: 0, step: 0.1 },
    ],
  },
  {
    tab: "Position",
    title: "Position",
    fields: [
      { key: "showOnMonitor", type: "monitor", label: "Monitor", hint: "Which screen to show the overlay on" },
      { key: "position", type: "select", label: "Screen position", options: ["top-left", "top-center", "top-right", "center", "bottom-left", "bottom-center", "bottom-right"] },
      { key: "moveOverlay", type: "action", label: "Drag to position", buttonLabel: "Drag on screen…", command: "begin_overlay_move", hint: "Opens a draggable handle on the overlay; drop it where you want, then Save." },
      { key: "topOffset", type: "number", label: "Top offset (px)" },
      { key: "bottomOffset", type: "number", label: "Bottom offset (px)" },
      { key: "leftOffset", type: "number", label: "Left offset (px)" },
      { key: "rightOffset", type: "number", label: "Right offset (px)" },
    ],
  },
  {
    tab: "Input",
    title: "Input",
    fields: [
      { key: "displayStyle", type: "select", label: "Overlay style", options: ["popups", "keyboard"], hint: "popups: fading key popups. keyboard: an on-screen keyboard that lights up as you type (great for tutorials/streams)." },
      { key: "keyboardVisibleKeys", type: "link", href: "key_select.html", buttonLabel: "Pick keys…", label: "Keys to show on keyboard", hint: "Keyboard style only: pick exactly which keys appear (e.g. just WASD + binds). Default shows the whole keyboard." },
      { key: "displayMode", type: "select", label: "Display mode", options: ["text", "raw"], hint: "Popups only. text: like a text editor — only typed characters, Backspace deletes. raw: every key (modifiers, ⌫, arrows, …)" },
      { key: "showKeyboardClick", type: "bool", label: "Show keyboard clicks" },
      { key: "showMouseClick", type: "bool", label: "Show mouse clicks" },
      { key: "showMouseCoordinates", type: "bool", label: "Show mouse coordinates", hint: "Not available on Wayland (the compositor hides the cursor position)" },
      { key: "onlyKeysWithModifiers", type: "bool", label: "Only keys with modifiers", hint: "Raw mode only: show a key only when Ctrl/Alt/Meta is held" },
      { key: "showSpaceAsUnicode", type: "bool", label: "Show space as ␣" },
      { key: "textToSymbols", type: "bool", label: "Special keys as symbols", hint: "e.g. Tab → ↹, Backspace → ⌫" },
      { key: "toggleCaptureHotkey", type: "text", label: "Toggle-capture hotkey", hint: "e.g. Ctrl+Alt+Y" },
      { key: "keyboardLayout", type: "text", label: "Keyboard layout override", hint: "Linux only, empty = auto-detect (e.g. us, de, tr); needs restart" },
      { key: "keyLabelOverrides", type: "link", label: "Key label overrides", hint: "Customize display text for any key (modifiers, arrows, F-keys, …)" },
    ],
  },
  {
    tab: "Devices",
    title: "Mouse (movement & scroll)",
    fields: [
      { key: "showMouseMovement", type: "bool", label: "Show mouse movement", hint: "A dot-in-a-ring widget that reacts to how you move the mouse (works on Wayland too)" },
      { key: "showMouseScroll", type: "bool", label: "Show scroll wheel", hint: "Scroll ticks appear as popup tokens (Scroll↑ / Scroll↓)" },
      { key: "mouseMovementSensitivity", type: "number", label: "Movement sensitivity", min: 0.1, step: 0.1 },
      { key: "mouseMovementDecaySeconds", type: "number", label: "Spring-back time (s)", min: 0.05, step: 0.05, hint: "How long the dot takes to return to center once the mouse stops" },
      { key: "deviceWidgetScale", type: "number", label: "Widget size (scale)", min: 0.3, max: 5, step: 0.1, hint: "Size of the mouse & gamepad widgets" },
    ],
  },
  {
    tab: "Devices",
    title: "Gamepad / controller",
    fields: [
      { key: "showGamepad", type: "bool", label: "Show gamepad input", hint: "Buttons as popups; sticks & triggers in a widget. Works with any XInput / DualShock-style controller." },
    ],
  },
  {
    tab: "Streaming",
    title: "OBS / browser source",
    fields: [
      { key: "obsServerEnabled", type: "bool", label: "Enable OBS browser source", hint: "Serves the overlay at http://localhost:<port>/overlay for an OBS Browser source (transparent, live). Enabling starts it immediately; changing the port needs a restart." },
      { key: "obsServerPort", type: "number", label: "Server port", min: 1, max: 65535, step: 1, hint: "Default 7238" },
      { key: "showOverlayOnScreen", type: "bool", label: "Show overlay on this screen", hint: "Turn off to display only in the OBS browser source (avoids showing twice or seeing it locally)." },
    ],
  },
  {
    tab: "Streaming",
    title: "Text-to-speech",
    fields: [
      { key: "textToSpeech", type: "bool", label: "Speak every keystroke" },
      { key: "textToSpeechCancelSpeechOnNewKey", type: "bool", label: "Cancel speech on new key" },
    ],
  },
  {
    tab: "Filter",
    title: "Process filter",
    fields: [
      { key: "filter", type: "bool", label: "Enable process filter", hint: "Capture only while a listed app is focused" },
      { key: "filterProcessName", type: "processlist", label: "Apps to capture in", hint: "Tick running apps, search to narrow the list, or type a name + Enter to add an app that isn't open yet" },
    ],
  },
];

const TAB_STORAGE_KEY = "yakc.settings.tab";

let config;
const procPickers = []; // { load } for each process picker, filled by buildForm

function fieldId(key) {
  return `field-${key}`;
}

// WebKitGTK (the Linux webview) sometimes doesn't paint DOM added after an async
// hop until the next reflow, so a freshly-loaded list looks empty until you
// interact with it. Toggling display forces an immediate re-layout + repaint.
function forceRepaint(el) {
  el.style.display = "none";
  void el.offsetHeight; // read forces synchronous layout
  el.style.display = "";
}

function makeLabel(field) {
  const label = document.createElement("label");
  label.htmlFor = fieldId(field.key);
  label.textContent = field.label;
  if (field.hint) {
    const hint = document.createElement("span");
    hint.className = "hint";
    hint.textContent = field.hint;
    label.appendChild(hint);
  }
  return label;
}

function buildForm() {
  const form = document.getElementById("form");
  form.textContent = "";
  procPickers.length = 0;

  const tabs = [...new Set(SECTIONS.map((s) => s.tab)), PROFILES_TAB];
  const nav = document.createElement("nav");
  nav.className = "tabs";
  const panels = document.createElement("div");
  panels.className = "panels";

  const panelByTab = new Map();
  for (const tab of tabs) {
    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = "tab-btn";
    btn.textContent = tab;
    btn.dataset.tab = tab;
    btn.addEventListener("click", () => activateTab(tab));
    nav.appendChild(btn);

    const panel = document.createElement("section");
    panel.className = "tab-panel";
    panel.dataset.tab = tab;
    panels.appendChild(panel);
    panelByTab.set(tab, panel);
  }

  for (const section of SECTIONS) {
    const panel = panelByTab.get(section.tab);
    const heading = document.createElement("h2");
    heading.textContent = section.title;
    panel.appendChild(heading);
    for (const field of section.fields) buildField(field, panel);
  }

  buildProfilesPanel(panelByTab.get(PROFILES_TAB));

  form.append(nav, panels);

  const saved = localStorage.getItem(TAB_STORAGE_KEY);
  activateTab(tabs.includes(saved) ? saved : tabs[0]);
}

function activateTab(tab) {
  for (const btn of document.querySelectorAll(".tab-btn")) {
    btn.classList.toggle("active", btn.dataset.tab === tab);
  }
  for (const panel of document.querySelectorAll(".tab-panel")) {
    panel.hidden = panel.dataset.tab !== tab;
  }
  // Load any process pickers on the now-visible tab. Rendering the checklist
  // into a visible panel (not a display:none one) lets its grid lay out with a
  // real width — otherwise the list only appears after the next reflow.
  for (const picker of procPickers) {
    if (picker.tab === tab) picker.load();
  }
  try {
    localStorage.setItem(TAB_STORAGE_KEY, tab);
  } catch {
    // private mode / storage disabled — non-fatal, just don't remember the tab
  }
}

// Builds one field's row(s) and appends them to `panel`.
function buildField(field, panel) {
  if (field.type === "processlist") {
    buildProcessPicker(field, panel);
    return;
  }

  const row = document.createElement("div");
  row.className = "field";
  row.appendChild(makeLabel(field));

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
  } else if (field.type === "combobox") {
    input = document.createElement("input");
    input.type = "text";
    input.value = config[field.key];
    input.setAttribute("list", `dl-${field.key}`);
    const dl = document.createElement("datalist");
    dl.id = `dl-${field.key}`;
    for (const option of field.options) {
      const el = document.createElement("option");
      el.value = option;
      dl.appendChild(el);
    }
    row.appendChild(dl);
  } else if (field.type === "monitor") {
    // Populated live from get_monitors(); starts with the saved index so it is
    // correct even before hydration and if enumeration fails.
    input = document.createElement("select");
    const current = document.createElement("option");
    current.value = String(config[field.key] ?? 0);
    current.textContent = `Monitor ${config[field.key] ?? 0}`;
    input.appendChild(current);
    input.value = current.value;
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
      case "color": {
        input.type = "color";
        input.value = config[field.key];
        const swatches = document.createElement("div");
        swatches.className = "swatches";
        for (const color of COLOR_SWATCHES) {
          const sw = document.createElement("button");
          sw.type = "button";
          sw.className = "swatch";
          sw.style.background = color;
          sw.title = color;
          sw.addEventListener("click", () => (input.value = color));
          swatches.appendChild(sw);
        }
        const wrap = document.createElement("div");
        wrap.className = "color-field";
        tagInput(input, field);
        wrap.append(input, swatches);
        row.appendChild(wrap);
        panel.appendChild(row);
        return;
      }
      case "list":
        input.type = "text";
        input.value = (config[field.key] || []).join(", ");
        break;
      case "link":
        input = document.createElement("button");
        input.className = "link-btn";
        input.textContent = field.buttonLabel || "Edit";
        input.addEventListener("click", () => {
          window.location.href = field.href || "key_labels.html";
        });
        break;
      case "action":
        input = document.createElement("button");
        input.className = "link-btn";
        input.textContent = field.buttonLabel || "Run";
        input.addEventListener("click", () => core.invoke(field.command));
        break;
      default:
        input.type = "text";
        input.value = config[field.key];
    }
  }

  tagInput(input, field);
  row.appendChild(input);
  panel.appendChild(row);
}

function tagInput(input, field) {
  input.id = fieldId(field.key);
  input.dataset.type = field.type;
  input.dataset.key = field.key;
}

// A searchable process picker: selected apps as removable chips (the saved
// value), a search box that both filters the running-app list AND adds a typed
// name on Enter, and a checklist of running apps. All three stay in sync.
function buildProcessPicker(field, panel) {
  const labelRow = document.createElement("div");
  labelRow.className = "field field--wide";
  labelRow.appendChild(makeLabel(field));
  panel.appendChild(labelRow);

  const wrap = document.createElement("div");
  wrap.className = "field field--wide proc-picker";

  // Hidden element is the source of truth read by collectForm.
  const hidden = document.createElement("input");
  hidden.type = "hidden";
  tagInput(hidden, field);

  const selected = new Set((config[field.key] || []).map(String));
  const lowerSelected = () => new Set([...selected].map((s) => s.toLowerCase()));
  const hasName = (name) => [...selected].some((s) => s.toLowerCase() === name.toLowerCase());

  const chips = document.createElement("div");
  chips.className = "chips";

  const search = document.createElement("input");
  search.type = "text";
  search.className = "proc-search";
  search.placeholder = "Search running apps, or type a name + Enter to add…";

  const refresh = document.createElement("button");
  refresh.type = "button";
  refresh.className = "link-btn";
  refresh.textContent = "↻ Refresh";

  const controls = document.createElement("div");
  controls.className = "proc-controls";
  controls.append(search, refresh);

  const list = document.createElement("div");
  list.className = "proc-list";
  list.textContent = "Loading running apps…";

  wrap.append(hidden, chips, controls, list);
  panel.appendChild(wrap);

  let running = [];

  const sync = () => (hidden.value = [...selected].join(", "));

  function renderChips() {
    chips.textContent = "";
    if (selected.size === 0) {
      const empty = document.createElement("span");
      empty.className = "chips-empty";
      empty.textContent = "No apps selected yet.";
      chips.appendChild(empty);
      return;
    }
    for (const name of selected) {
      const chip = document.createElement("span");
      chip.className = "chip";
      chip.appendChild(document.createTextNode(name));
      const x = document.createElement("button");
      x.type = "button";
      x.className = "chip-x";
      x.textContent = "×";
      x.title = `Remove ${name}`;
      x.addEventListener("click", () => removeName(name));
      chip.appendChild(x);
      chips.appendChild(chip);
    }
  }

  function addName(name) {
    const n = name.trim();
    if (n && !hasName(n)) selected.add(n);
    sync();
    renderChips();
    syncListChecks();
  }

  function removeName(name) {
    for (const s of [...selected]) {
      if (s.toLowerCase() === name.toLowerCase()) selected.delete(s);
    }
    sync();
    renderChips();
    syncListChecks();
  }

  function renderList() {
    list.textContent = "";
    if (running.length === 0) {
      list.textContent = "No running apps found.";
      return;
    }
    const sel = lowerSelected();
    const q = search.value.trim().toLowerCase();
    let shown = 0;
    for (const name of running) {
      if (q && !name.toLowerCase().includes(q)) continue;
      const item = document.createElement("label");
      item.className = "proc-item";
      item.dataset.name = name.toLowerCase();
      const box = document.createElement("input");
      box.type = "checkbox";
      box.checked = sel.has(name.toLowerCase());
      box.addEventListener("change", () => (box.checked ? addName(name) : removeName(name)));
      item.append(box, document.createTextNode(name));
      list.appendChild(item);
      shown++;
    }
    if (shown === 0) {
      const none = document.createElement("div");
      none.className = "proc-none";
      none.textContent = q
        ? `No running app matches “${search.value.trim()}”. Press Enter to add it anyway.`
        : "";
      list.appendChild(none);
    }
    forceRepaint(list);
  }

  // Re-check the visible list against the selection without rebuilding it.
  function syncListChecks() {
    const sel = lowerSelected();
    for (const item of list.querySelectorAll(".proc-item")) {
      item.querySelector("input").checked = sel.has(item.dataset.name);
    }
  }

  search.addEventListener("input", renderList);
  search.addEventListener("keydown", (e) => {
    if (e.key === "Enter") {
      e.preventDefault();
      const value = search.value;
      search.value = "";
      addName(value);
      renderList();
    }
  });
  refresh.addEventListener("click", () => load());

  async function load() {
    list.textContent = "Loading running apps…";
    try {
      running = await core.invoke("get_running_processes");
    } catch {
      running = [];
    }
    renderList();
  }

  sync();
  renderChips();
  procPickers.push({ tab: panel.dataset.tab, load });
}

// Rebuild the whole form after the config changed underneath us (profile load,
// preset, import), staying on the given tab.
async function reloadForm(activeTab) {
  config = await core.invoke("get_config");
  buildForm();
  hydrateMonitors();
  if (activeTab) activateTab(activeTab);
}

// The Profiles tab: named snapshots + bundled presets + import/export. Custom
// UI (not schema-driven), all wired to the profile backend commands.
function buildProfilesPanel(panel) {
  const setStatus = (msg) => {
    const status = document.getElementById("status");
    if (!status) return;
    status.textContent = msg;
    clearTimeout(setStatus._t);
    setStatus._t = setTimeout(() => (status.textContent = ""), 4000);
  };

  const guard = async (fn) => {
    try {
      await fn();
    } catch (err) {
      setStatus(`Error: ${err}`);
    }
  };

  const heading = (text) => {
    const h = document.createElement("h2");
    h.textContent = text;
    return h;
  };

  const button = (label, cls, onClick) => {
    const b = document.createElement("button");
    b.type = "button";
    b.className = cls;
    b.textContent = label;
    b.addEventListener("click", onClick);
    return b;
  };

  panel.appendChild(heading("Profiles"));

  const active = document.createElement("p");
  active.className = "profiles-active";
  panel.appendChild(active);

  // Row: profile dropdown + Load + Delete.
  const select = document.createElement("select");
  select.className = "profiles-select";
  const pickRow = document.createElement("div");
  pickRow.className = "profiles-row";
  pickRow.append(
    select,
    button("Load", "profiles-btn", () =>
      guard(async () => {
        const name = select.value;
        if (!name) return;
        await core.invoke("load_profile", { name });
        await reloadForm(PROFILES_TAB);
        setStatus(`Loaded “${name}” ✓`);
      })
    ),
    button("Delete", "profiles-btn", () =>
      guard(async () => {
        const name = select.value;
        if (!name) return;
        await core.invoke("delete_profile", { name });
        await refreshProfiles();
        setStatus(`Deleted “${name}”`);
      })
    )
  );
  panel.appendChild(pickRow);

  // Row: name box + Save-as + Rename.
  const nameInput = document.createElement("input");
  nameInput.type = "text";
  nameInput.className = "profiles-name";
  nameInput.placeholder = "Profile name";
  const nameRow = document.createElement("div");
  nameRow.className = "profiles-row";
  nameRow.append(
    nameInput,
    button("Save current as", "profiles-btn", () =>
      guard(async () => {
        const name = nameInput.value.trim();
        if (!name) return setStatus("Enter a profile name first");
        await core.invoke("save_profile", { name });
        nameInput.value = "";
        await refreshProfiles();
        setStatus(`Saved “${name}” ✓`);
      })
    ),
    button("Rename selected", "profiles-btn", () =>
      guard(async () => {
        const old = select.value;
        const next = nameInput.value.trim();
        if (!old || !next) return setStatus("Pick a profile and type a new name");
        await core.invoke("rename_profile", { old, new: next });
        nameInput.value = "";
        await refreshProfiles();
        setStatus(`Renamed to “${next}”`);
      })
    )
  );
  panel.appendChild(nameRow);

  // Row: import / export.
  const ioRow = document.createElement("div");
  ioRow.className = "profiles-row";
  ioRow.append(
    button("Import JSON…", "profiles-btn", () =>
      guard(async () => {
        const imported = await core.invoke("import_config");
        if (imported) {
          await reloadForm(PROFILES_TAB);
          setStatus("Imported config ✓");
        }
      })
    ),
    button("Export JSON…", "profiles-btn", () =>
      guard(async () => {
        const ok = await core.invoke("export_config");
        setStatus(ok ? "Exported config ✓" : "Export cancelled");
      })
    )
  );
  panel.appendChild(ioRow);

  // Bundled presets.
  panel.appendChild(heading("Starter presets"));
  const presetHint = document.createElement("p");
  presetHint.className = "hint";
  presetHint.textContent =
    "Apply a starter look on top of your current settings, then tweak and save it as a profile.";
  panel.appendChild(presetHint);
  const presetRow = document.createElement("div");
  presetRow.className = "profiles-row";
  panel.appendChild(presetRow);
  guard(async () => {
    const presets = await core.invoke("list_presets");
    for (const name of presets) {
      presetRow.appendChild(
        button(name, "profiles-btn preset-btn", () =>
          guard(async () => {
            await core.invoke("apply_preset", { name });
            await reloadForm(PROFILES_TAB);
            setStatus(`Applied “${name}” preset ✓`);
          })
        )
      );
    }
  });

  // Populate (and re-populate) the dropdown + active label.
  async function refreshProfiles() {
    const [names, current] = await Promise.all([
      core.invoke("list_profiles"),
      core.invoke("get_active_profile"),
    ]);
    select.textContent = "";
    if (names.length === 0) {
      const opt = document.createElement("option");
      opt.value = "";
      opt.textContent = "(no saved profiles)";
      select.appendChild(opt);
      select.disabled = true;
    } else {
      select.disabled = false;
      for (const name of names) {
        const opt = document.createElement("option");
        opt.value = name;
        opt.textContent = name;
        select.appendChild(opt);
      }
      if (current) select.value = current;
    }
    active.textContent = current ? `Active profile: ${current}` : "No profile selected (custom settings)";
  }

  // Expose so external events (tray, profiles-updated) can refresh it.
  buildProfilesPanel._refresh = refreshProfiles;
  guard(refreshProfiles);
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
      case "monitor":
        updated[key] = Number(input.value) || 0;
        break;
      case "list":
      case "processlist":
        updated[key] = input.value
          .split(",")
          .map((s) => s.trim())
          .filter((s) => s.length > 0);
        break;
      case "link":
      case "action":
        // Not inputs — a page link / a command button. Leave the config value
        // untouched (spread above); collecting the button's empty string here
        // would clobber real fields (e.g. keyLabelOverrides, a map).
        break;
      default:
        updated[key] = input.value;
    }
  }
  return updated;
}

// Fill each monitor dropdown with the real connected monitors, keeping the
// saved index selected.
async function hydrateMonitors() {
  let monitors;
  try {
    monitors = await core.invoke("get_monitors");
  } catch {
    return; // leave the saved-index fallback option in place
  }
  if (!Array.isArray(monitors) || monitors.length === 0) return;
  for (const select of document.querySelectorAll('[data-type="monitor"]')) {
    const chosen = select.value;
    select.textContent = "";
    monitors.forEach((label, index) => {
      const el = document.createElement("option");
      el.value = String(index);
      el.textContent = label;
      select.appendChild(el);
    });
    select.value = Number(chosen) < monitors.length ? chosen : "0";
  }
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
  buildForm(); // activateTab() lazily loads the process picker when its tab shows
  hydrateMonitors();
  document.getElementById("saveBtn").addEventListener("click", save);
  // Ctrl/Cmd+S saves from anywhere in the window.
  window.addEventListener("keydown", (e) => {
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "s") {
      e.preventDefault();
      save();
    }
  });
  // The tray "Manage profiles…" opens settings on the Profiles tab.
  tauriEvent.listen("open-profiles-tab", () => activateTab(PROFILES_TAB));
  // Keep the Profiles list fresh when a profile is switched from the tray.
  tauriEvent.listen("profiles-updated", () => {
    if (buildProfilesPanel._refresh) buildProfilesPanel._refresh();
  });
}

window.addEventListener("DOMContentLoaded", init);
