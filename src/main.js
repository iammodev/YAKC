const { app, Tray, Menu, BrowserWindow, dialog, screen } = require("electron");
const iohook = require("@mechakeys/iohook");
const fs = require("fs");
const keycode = require("keycodes");
const path = require("path");
let config;
let isCapturing = true;

// Event handler when the application is ready
app.on("ready", () => {
  // Check if config.json exists
  if (fs.existsSync("./config.json")) {
    config = JSON.parse(fs.readFileSync("./config.json", "utf-8"));
    // Create a default config.json if it doesn't exist
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
      textToSpeech: false,
      textToSpeechCancelSpeechOnNewKey: false,
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
      devTools: true,
      nodeIntegration: true,
      contextIsolation: false,
    },
  });

  // useful for debugging
  //mainWindow.webContents.openDevTools();

  // Disable mouse events (useful for transparent windows)
  mainWindow.setIgnoreMouseEvents(true);
  // Load the index.html file
  mainWindow.loadFile("./index.html");

  // Start listening to events by using iohook
  iohook.start();

  // Event handler for keydown events
  if (showKeyboardClick) {
    iohook.on("keydown", (event) => {
      mainWindow.webContents.send("keydown", generateKeyLabel(event));
      // if (generateKeyLabel(event, config).length > 0) {
      //   mainWindow.webContents.send("keydown", generateKeyLabel(event, config));
      // }
    });
    // iohook.on("keydown", (event) => {
    //   if (generateKeyLabel(event, config).length > 0) {
    //     mainWindow.webContents.send("keydown", generateKeyLabel(event, config));
    //   }
    // });
  }

  // Event handler for mouse events
  if (showMouseClick) {
    // Show mouse coordinates
    if (!showMouseCoordinates) {
      iohook.on("mousedown", (event) => {
        mainWindow.webContents.send("keydown", ` MOUSE${event.button} `);
      });
      // Show mouse button
    } else {
      iohook.on("mousedown", (event) => {
        mainWindow.webContents.send(
          "keydown",
          ` MOUSE${event.button} X: ${event.x} Y: ${event.y} `
        );
      });
    }
  }

  // Create a tray icon
  let trayIcon;

  // Set the tray icon based on the platform
  switch (process.platform) {
    case "win32":
      trayIcon = path.join(
        __dirname,
        "..",
        "assets",
        "icons",
        "windows",
        "icon.ico"
      );
      console.log(trayIcon);
      break;
    case "darwin":
      trayIcon = path.join(
        __dirname,
        "..",
        "assets",
        "icons",
        "mac",
        "icon.icns"
      );
      break;
    default:
      trayIcon = path.join(
        __dirname,
        "..",
        "assets",
        "icons",
        "linux",
        "icon.png"
      );
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

  // Function to get the modifier key
  const getModifier = () => {
    if (ctrlKey) return " CTRL ";
    if (shiftKey) return " SHIFT ";
    if (altKey) return " ALT ";
    if (metaKey) return " META ";
    return "";
  };

  // Get the modifier key
  const modifier = getModifier();

  // If only keys with modifiers are required and no modifier key is pressed, return empty string
  if (onlyKeysWithModifiers && !modifier) {
    return "";
  }

  // If a modifier key is pressed, append it to the key label
  if (modifier) {
    return `${modifier} + ${keyLabel.toUpperCase()}`;
  }

  // If the key label is "space", return the appropriate space character
  if (keyLabel === "space") {
    return showSpaceAsUnicode ? "␣" : " ";
  }

  // If no special case applies, return the key label
  return keyLabel || "";
}
