/**
 * Title: YAKC - Yet Another Key Caster
 * Credits: [Mo Devecioglu](https://github.com/iammodev/)
 * Date: 01/11/2023
 */

const {
  app,
  Tray,
  Menu,
  dialog,
  ipcMain,
  BrowserWindow,
  screen,
} = require("electron");
const iohook = require("@mechakeys/iohook");
const fs = require("fs");
const path = require("path");
const isSingleInstance = app.requestSingleInstanceLock();
const activeWindow = require("active-win");

let config;
let isCapturing = true;

// Check if the app is already running
if (!isSingleInstance) {
  // If the app is already running, quit the app
  app.quit();
}

// Gets called when the app is ready
app.on("ready", () => {
  // Check if config.json exists
  if (fs.existsSync(path.join(__dirname, "..", "config.json"))) {
    // Load config.json
    config = JSON.parse(
      fs.readFileSync(path.join(__dirname, "..", "config.json"), "utf-8")
    );
  } else {
    // If config.json doesn't exist, create it based on defaultConfig.json
    if (fs.existsSync(path.join(__dirname, "defaultConfig.json"))) {
      const getDefaultConfig = fs.readFileSync(
        path.join(__dirname, "defaultConfig.json"),
        "utf-8"
      );
      // Create ./config.json (using data from ./src/defaultConfig.json)
      config = JSON.parse(getDefaultConfig);
      fs.writeFileSync(
        path.join(__dirname, "..", "config.json"),
        JSON.stringify(config, null, 2)
      );
    }
  }

  // If config.json is empty, show an error message and exit the app
  if (!config) {
    dialog.showErrorBox(
      "Error",
      "config.json not found, please contact the developer."
    );
    app.quit();
    return;
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

  // Disable mouse events (necessary for transparent window)
  mainWindow.setIgnoreMouseEvents(true);

  // Load the index.html file
  mainWindow.loadFile(path.join(__dirname, "index.html"));

  // only for debugging
  //mainWindow.webContents.openDevTools();

  // Send the config.json to the renderer after window.onload event is triggered
  ipcMain.on("rendererLoaded", () => {
    mainWindow.webContents.send("configData", config);

    // Make the window always on top at the 'screen-saver' level
    mainWindow.setAlwaysOnTop(true, "screen-saver");

    // Check if filter is enabled
    if (config.filter) {
      checkActiveProcess();
    }
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
    // Show mouse with x, y coordinates
    if (config.showMouseCoordinates) {
      iohook.on("mousedown", (event) => {
        mainWindow.webContents.send(
          "clickEvent",
          ` MOUSE${event.button} X: ${event.x} Y: ${event.y} `
        );
      });
      // Show mouse button
    } else {
      iohook.on("mousedown", (event) => {
        mainWindow.webContents.send("clickEvent", ` MOUSE${event.button} `);
      });
    }
  }

  // Create a tray icon
  const iconPath = path.join(__dirname, "..", "assets", "icons");
  // Get the OS
  const platform = process.platform;

  // Create a tray icon
  let trayIcon;

  // Set the tray icon based on the platform
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
 * Generate a key label based on the given event and keyboard layout.
 * @param {Object} event - The event object.
 * @returns {string} The generated key label.
 */
function generateKeyLabel(event) {
  // Load the keyboard layout based on the config.json
  const currentLayout = loadKeyboardLayout(config.keyboardLayout.toLowerCase());

  // Extract necessary properties from the event object
  const { shiftKey, ctrlKey, altKey, metaKey, rawcode } = event;

  // Initialize an array to store modifiers and a string for key label
  const modifiers = [];
  let keyLabel = "";

  // Add modifiers to the array based on key states
  if (ctrlKey) modifiers.push("CTRL");
  if (altKey) modifiers.push("ALT");
  if (shiftKey) modifiers.push("SHIFT");
  if (metaKey) modifiers.push("META");

  // if shiftKey is true and rawcode is in currentLayout.shift
  if (shiftKey && rawcode in currentLayout.shift) {
    keyLabel = shiftKey ? currentLayout.shift[rawcode] : "";
    // TODO: altKey for some keyboardLayouts necessary
  } else if (rawcode in currentLayout.default) {
    // push modifier(s) if pressed (as example: to return CTRL + ALT + H)
    if (modifiers.length > 0) {
      if (keyLabel !== currentLayout.default)
        keyLabel = ` ${modifiers.join(" + ")} + ${currentLayout.default[
          rawcode
        ].toUpperCase()} `;
    }
    // if shiftKey is false and rawcode is in currentLayout.default
    if (!shiftKey && !ctrlKey && !altKey && !metaKey) {
      keyLabel = currentLayout.default[rawcode];
    }
    // if shiftKey is true and rawcode is in currentLayout.default return upper case
    if (shiftKey) {
      keyLabel = currentLayout.default[rawcode].toUpperCase();
    }
    // if Space is pressed, check config settings
    if (keyLabel.toLocaleLowerCase() == "space") {
      keyLabel = config.showSpaceAsUnicode ? "␣" : " ";
    }

    // Convert keyLabel to unicode
    if (config.textToSymbols) {
      const mapper = charToUnicodeMapper();
      if (keyLabel.toLocaleLowerCase() in mapper) {
        keyLabel = mapper[keyLabel.toLocaleLowerCase()];
      }
    }
  }

  return keyLabel;
}
/**
 * Loads the keyboard layout for a given language.
 *
 * @param {string} language - The language for which to load the keyboard layout.
 * @return {Object} The keyboard layout object for the specified language.
 */
function loadKeyboardLayout(language) {
  const languageFilePath = path.join(
    __dirname,
    "keyboardLayouts",
    `${language}.js`
  );

  if (fs.existsSync(languageFilePath)) {
    const keyboardLayout = require(languageFilePath);
    return keyboardLayout;
  }
  return {};
}

/**
 * Retrieves the character to Unicode mapping from a file or returns an empty object.
 *
 * @return {Object} The character to Unicode mapping or an empty object if the file does not exist.
 */
function charToUnicodeMapper() {
  const charToUnicodeFilePath = path.join(__dirname, "charToUnicode.js");

  if (fs.existsSync(charToUnicodeFilePath)) {
    const charToUnicode = require(charToUnicodeFilePath);
    return charToUnicode;
  }
  return {};
}

/**
 * This function checks the active process and starts or stops listening for keystrokes
 * based on whether the process is in the filter list.
 */
async function checkActiveProcess() {
  let filters = [];

  try {
    // Load the filter list from config.json
    if (config.filterProcessName && Array.isArray(config.filterProcessName)) {
      filters = config.filterProcessName.map((filter) => filter.toLowerCase());
    }
  } catch (error) {
    console.error("Error loading config:", error);
  }

  while (true) {
    try {
      const currentWindow = await activeWindow();

      if (currentWindow && currentWindow.owner && currentWindow.owner.path) {
        const processPath = currentWindow.owner.path;
        const currentProcess =
          getProcessNameFromPath(processPath).toLowerCase();

        // Check if the active process is in the filter list
        if (filters.includes(currentProcess)) {
          // Start listening for keystrokes
          iohook.start();
        } else {
          // Stop listening for keystrokes
          iohook.stop();
        }
      }
    } catch (error) {
      console.error("Error:", error);
    }

    // Wait for X seconds before checking again
    await new Promise((resolve) =>
      setTimeout(resolve, config.filterCheckEverySecond * 1000)
    );
  }
}

/**
 * This function extracts the process name from a given path.
 *
 * @param {string} path - The path from which to extract the process name.
 * @returns {string} - The process name extracted from the path.
 */
function getProcessNameFromPath(path) {
  // Split the path into parts using the path separator
  const pathParts = path.split(getPathSeparator());

  // The process name is the last part of the path
  const processName = pathParts[pathParts.length - 1];

  // Return the process name
  return processName;
}

/**
 * Function to get the path separator based on the platform
 * @returns {string} - The path separator ('\\' for win32, '/' for others)
 */
function getPathSeparator() {
  // Check the platform
  return process.platform === "win32" ? "\\" : "/";
}
