const { app, Tray, Menu, BrowserWindow, dialog, screen } = require("electron");
const iohook = require("@mechakeys/iohook");
const fs = require("fs");
const keycode = require("keycode");
const path = require("path");
let config;
let isCapturing = true;

app.on("ready", () => {
  if (fs.existsSync("./config.json")) {
    config = JSON.parse(fs.readFileSync("./config.json", "utf-8"));
  } else {
    config = {
      showOnMonitor: "0",
      popupTextMaxWidthInPercentage: "60",
      popupOpacity: ".9",
      popupFadeInSeconds: "0.5",
      popupRemoveAfterSeconds: "3",
      popupInactiveAfterSeconds: ".5",
      popupFontSize: "20",
      popupFontFamily: "Tahoma, sans-serif",
      popupFontWeight: "bold",
      popupBorderRadius: "10",
      popupFontColor: "#ffffff",
      popupBackgroundColor: "#000000",
      showKeyboardClick: true,
      showMouseClick: true,
      showMouseCoordinates: false,
      onlyKeysWithModifiers: false,
      showSpaceAsUnicode: false,
      textToSpeech: true,
      position: "top-left",
      topOffset: "0",
      bottomOffset: "0",
      leftOffset: "0",
      rightOffset: "0",
    };
    fs.writeFileSync("./config.json", JSON.stringify(config, null, 2));
    dialog.showMessageBox({
      type: "info",
      message:
        "YAKC does not have a configuration file. A default configuration file is being created. Please edit the 'config.json' file and restart the application for your changes to take effect.",
      buttons: ["Okay"],
    });
  }
  const {
    showOnMonitor,
    showKeyboardClick,
    showMouseClick,
    showMouseCoordinates,
    textToSpeech,
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

  const mainWindow = new BrowserWindow({
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
    kiosk: true,
    frame: false,
    webPreferences: {
      nodeIntegration: true,
      contextIsolation: false,
    },
  });

  //window.webContents.openDevTools();

  mainWindow.setIgnoreMouseEvents(true);
  mainWindow.loadFile("./index.html");

  iohook.start();

  if (showKeyboardClick) {
    iohook.on("keydown", (event) => {
      if (convertSpecialKeys(event, config).length > 0) {
        mainWindow.webContents.send(
          "keydown",
          convertSpecialKeys(event, config)
        );
      }
    });
  }

  if (showMouseClick) {
    if (!showMouseCoordinates) {
      iohook.on("mousedown", (event) => {
        mainWindow.webContents.send("keydown", ` MOUSE${event.button} `);
      });
    } else {
      iohook.on("mousedown", (event) => {
        mainWindow.webContents.send(
          "keydown",
          ` MOUSE${event.button} X: ${event.x} Y: ${event.y} `
        );
      });
    }
  }

  let trayIcon;

  switch (process.platform) {
    case "win32":
      trayIcon = path.join(__dirname, "assets", "icons", "windows", "icon.ico");
      break;
    case "darwin":
      trayIcon = path.join(__dirname, "assets", "icons", "mac", "icon.icns");
      break;
    default:
      trayIcon = path.join(__dirname, "assets", "icons", "linux", "icon.png");
  }

  // Create a tray icon
  const menuTray = new Tray(trayIcon);

  // Set the context menu
  const contextMenu = Menu.buildFromTemplate([
    { label: "Toggle Capturing", click: () => toggleCapturing() },
    { label: "Quit", click: () => app.exit() },
  ]);

  // Set Tool Tip
  menuTray.setToolTip("YAKC - Yet Another Key Caster");

  // Set ContextMenu
  menuTray.setContextMenu(contextMenu);
});

function toggleCapturing() {
  isCapturing = !isCapturing;

  if (isCapturing) {
    iohook.start();
  } else {
    iohook.stop();
  }
}

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
