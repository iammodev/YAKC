/**
 * Title: Communication from main.js to renderer.js
 * Credits: [Mo Devecioglu](https://github.com/iammodev/)
 * Date: 01/11/2023
 */

const { contextBridge, ipcRenderer } = require("electron");

// create custom YAKC API
contextBridge.exposeInMainWorld("YAKCAPI", {
  // create Listener(s)
  onConfigData: makeListener("configData"),
  onClickEvent: makeListener("clickEvent"),
  onRendererLoaded: rendererLoaded,
});

/**
 * Sends a message to the main process to indicate that the renderer has loaded.
 */
function rendererLoaded() {
  ipcRenderer.send("rendererLoaded");
}

/**
 * Creates a listener for a given channel.
 * @param {string} channel - The channel to listen on.
 * @returns {Function} - The listener function.
 */
function makeListener(channel) {
  /**
   * Listens for events on the given channel.
   * @param {Function} listener - The function to call when an event occurs.
   * @returns {Function} - The function to remove the listener.
   */
  return function (listener) {
    // Attach the listener to the channel
    ipcRenderer.on(channel, (_evt, ...args) => listener(...args));

    // Return a function to remove the listener
    return () => ipcRenderer.removeListener(channel, listener);
  };
}
