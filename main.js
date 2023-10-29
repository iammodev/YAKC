const { app, Tray, Menu, BrowserWindow, ipcMain, screen } = require("electron");
const iohook = require("@mechakeys/iohook");
const fs = require("fs");
const keycodemap = require("./keycode/index");

app.on("ready", () => {
  // Load Config
  const config = JSON.parse(fs.readFileSync("./config.json", "utf-8"));
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

  console.log(displays[showOnMonitor]);

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

  if (config && config.selectMap) {
    keycodemap.setMap(config.selectMap);
  } else {
    keycodemap.setMap("us");
  }

  iohook.start();

  iohook.on("keydown", (event) => {
    // TODO: onlyKeysWithModifiers

    let keyLabel = keycodemap.map(event.rawcode);

    if (keyLabel == "SPACE") {
      if (config && config.showSpaceAsUnicode) {
        keyLabel = "␣";
      } else {
        keyLabel = " ";
      }
    }

    if (keyLabel) {
      window.webContents.send("keydown", keyLabel);
    }
  });

  // Create a tray icon
  let menuTray = new Tray("./assets/testIcon.png");

  // Set the context menu
  const contextMenu = Menu.buildFromTemplate([
    { label: "Quit", click: () => app.quit() },
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
