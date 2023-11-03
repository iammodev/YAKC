const { app, Tray, Menu, ipcMain, BrowserWindow, screen } = require("electron");
const iohook = require("@mechakeys/iohook");
const fs = require("fs");
const keycode = require("keycodes");
const path = require("path");
let config;
let isCapturing = true;

// Gets called when the app is ready
app.on("ready", () => {
  // Check if config.json exists
  if (fs.existsSync("./config.json")) {
    // Load config.json
    config = JSON.parse(fs.readFileSync("./config.json", "utf-8"));
  } else {
    // if config.json doesn't exist, Create a default ./config.json based on ./src/defaultConfig.json
    const getDefaultConfig = fs.readFileSync(
      "./src/defaultConfig.json",
      "utf-8"
    );
    // Create config.json (using data from defaultConfig.json)
    config = JSON.parse(getDefaultConfig);
    fs.writeFileSync("./config.json", JSON.stringify(config, null, 2));
  }

  // Get all available monitors
  const displays = screen.getAllDisplays();

  // Select the monitor in the config if available
  const selectedMonitor =
    displays[config.showOnMonitor] || screen.getPrimaryDisplay();

  // Create the main window
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
    frame: false,
    webPreferences: {
      devTools: false,
      nodeIntegration: false,
      contextIsolation: true,
      preload: path.join(__dirname, "preload.js"),
    },
  });

  // Disable mouse events (useful for transparent windows)
  mainWindow.setIgnoreMouseEvents(true);

  // Load the index.html file
  mainWindow.loadFile("./index.html");

  // Send the config.json to the renderer after renderer onload event is triggered
  ipcMain.on("rendererLoaded", () => {
    mainWindow.webContents.send("configData", config);
  });

  // Start listening to events by using iohook
  iohook.start();
  // Event handler for keydown events
  if (config.showKeyboardClick) {
    iohook.on("keydown", (event) => {
      // Send the keyLabel to the renderer (using preload.js)
      mainWindow.webContents.send("clickEvent", generateKeyLabel(event));
    });
  }

  // Event handler for mouse events
  if (config.showMouseClick) {
    // Show mouse coordinates
    if (!config.showMouseCoordinates) {
      iohook.on("mousedown", (event) => {
        mainWindow.webContents.send("clickEvent", ` MOUSE${event.button} `);
      });
      // Show mouse button
    } else {
      iohook.on("mousedown", (event) => {
        mainWindow.webContents.send(
          "clickEvent",
          ` MOUSE${event.button} X: ${event.x} Y: ${event.y} `
        );
      });
    }
  }

  // Set the tray icon based on the platform
  const iconPath = path.join(__dirname, "..", "assets", "icons");
  const platform = process.platform;

  // Create a tray icon
  let trayIcon;

  if (platform === "win32") {
    trayIcon = path.join(iconPath, "windows", "icon.ico");
  } else if (platform === "darwin") {
    trayIcon = path.join(iconPath, "mac", "icon.icns");
  } else {
    trayIcon = path.join(iconPath, "linux", "icon.png");
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

/**
 * Toggles the state of keystroke capturing.
 * If capturing is on, it starts capturing.
 * If capturing is off, it stops capturing.
 */
function toggleCapturing() {
  // Toggle the state of capturing
  isCapturing = !isCapturing;

  // If capturing is on, start capturing
  if (isCapturing) {
    iohook.start();
  }
  // If capturing is off, stop capturing
  else {
    iohook.stop();
  }
}

/**
 * Generate a key label based on the given event.
 * @param {Object} event - The event object.
 * @returns {string} - The generated key label.
 */
function generateKeyLabel(event) {
  // Extract the key label from the event
  const keyLabel = keycode(event.rawcode);

  // Extract the config options
  const { onlyKeysWithModifiers, showSpaceAsUnicode } = config || {};

  // Extract the modifier keys from the event
  const { shiftKey, ctrlKey, altKey, metaKey } = event;

  return keyLabel;
  // let getKeyMapper;

  // if (fs.existsSync("./src/keyMapper.json")) {
  //   getKeyMapper = fs.readFileSync("./src/keyMapper.json");
  // }

  // // Function to get the modifier key
  // for (keyLabel in getKeyMapper) {
  //   if (getKeyMapper[mappedKey] === keyLabel) {
  //     console.log(mappedKey);
  //     return mappedKey;
  //   }
  // }

  // Get the modifier key

  // console.log(mappedKey);

  // console.log(mappedKey[keyLabel]);

  // If only keys with modifiers are required and no modifier key is pressed, return empty string
  if (onlyKeysWithModifiers && !modifier) {
    return "";
  }

  // If a modifier is pressed and keyLabel has alphabetic and numeric characters
  // if (modifier && /^[a-zA-Z0-9]+$/.test(keyLabel)) {
  //   // Check if keyLabel is already equal to the modifier key
  //   if (keyLabel.toUpperCase() === modifier.trim()) {
  //     return modifier.trim();
  //   }
  //   return `${modifier} + ${keyLabel.toUpperCase()}`;
  // }

  // // If the key label is "space", return the appropriate space character
  // if (keyLabel === "space") {
  //   return showSpaceAsUnicode ? "␣" : " ";
  // }

  // // If no special case applies, return the key label
  // if (/^[a-zA-Z0-9]+$/.test(keyLabel)) {
  //   return keyLabel;
  // }

  // return "";
}
