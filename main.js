const { app, Tray, Menu, BrowserWindow, ipcMain, screen } = require("electron");
const iohook = require("@mechakeys/iohook");
const fs = require("fs");
const keycode = require("keycode");
let config;

app.on("ready", () => {
  // Load Config
  config = JSON.parse(fs.readFileSync("./config.json", "utf-8"));
  const {
    showOnMonitor,
    position,
    topOffset,
    bottomOffset,
    leftOffset,
    rightOffset,
  } = config;

  // Get all available monitors
  const displays = screen.getAllDisplays();

  // Select the monitor in the config if available
  const selectedMonitor = displays[showOnMonitor] || screen.getPrimaryDisplay();

  const window = new BrowserWindow({
    x: selectedMonitor.bounds.x,
    y: selectedMonitor.bounds.y,
    width: selectedMonitor.bounds.width,
    height: selectedMonitor.bounds.height,
    fullscreen: true,
    transparent: true,
    focusable: false,
    alwaysOnTop: true,
    resizable: false,
    skipTaskbar: false,
    frame: false,
    webPreferences: {
      nodeIntegration: true,
      contextIsolation: false,
    },
  });

  //window.webContents.openDevTools();

  window.setIgnoreMouseEvents(true);
  window.loadFile("./index.html");
  window.webContents.send("config", config);

  iohook.start();

  iohook.on("keydown", (event) => {
    if (convertSpecialKeys(event, config).length > 0)
      window.webContents.send("keydown", convertSpecialKeys(event, config));
  });

  if (config && config.showMouseClick) {
    iohook.on("mousedown", (event) => {
      window.webContents.send("keydown", ` MOUSE${event.button} `);
    });
  }

  // Create a tray icon
  let menuTray = new Tray("./assets/yakc-logo.png");

  // Set the context menu
  const contextMenu = Menu.buildFromTemplate([
    { label: "Quit", click: () => app.exit() },
  ]);

  // Set Tool Tip
  menuTray.setToolTip("YAKC");

  // Set ContextMenu
  menuTray.setContextMenu(contextMenu);
});

ipcMain.on("config", (event, newConfig) => {
  if (newConfig) {
    window.webContents.send("config", newConfig);
  }
});

function convertSpecialKeys(event, config) {
  const keyLabel = keycode(event.rawcode);
  const isModifierKey = config && config.onlyKeysWithModifiers;
  const isShiftKey = event.shiftKey;
  const isCtrlKey = event.ctrlKey;
  const isAltKey = event.altKey;
  const isMetaKey = event.metaKey;
  const isSpaceKey = keyLabel === "space";

  if (isModifierKey && !isCtrlKey && !isShiftKey && !isAltKey && !isMetaKey) {
    return "";
  }

  if (isCtrlKey && keyLabel !== undefined) {
    return ` CTRL + ${keyLabel.toUpperCase()} `;
  }

  if (isShiftKey) {
    if (isModifierKey && keyLabel !== undefined) {
      return ` SHIFT + ${keyLabel.toUpperCase()} `;
    }

    if (/^[a-zA-Z]$/.test(keyLabel)) {
      return keyLabel.toUpperCase();
    }

    if (keyLabel !== undefined) {
      return keyLabel;
    }
  }

  if (isAltKey && keyLabel !== undefined) {
    return ` ALT + ${keyLabel.toUpperCase()} `;
  }

  if (isMetaKey && keyLabel !== undefined) {
    return ` META + ${keyLabel.toUpperCase()} `;
  }

  if (isSpaceKey) {
    return config && config.showSpaceAsUnicode ? "␣" : " ";
  }

  return keyLabel || "";
}
