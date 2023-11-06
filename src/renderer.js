/**
 * Title: Renderer API
 * Credits: [Mo Devecioglu](https://github.com/iammodev/)
 * Date: 01/11/2023
 */

// define custom YAKC API
const getYAKCAPI = window.YAKCAPI;

let config;

// get the config data from main.js
getYAKCAPI.onConfigData((getConfig) => {
  config = getConfig;
});

/**
 * This function is called when the window loads.
 * It calls the onRendererLoaded function from the YAKCAPI.
 * The reason for this is to ensure config never gets called before the window loads.
 */
window.onload = () => {
  // Call the onRendererLoaded function from the YAKCAPI
  getYAKCAPI.onRendererLoaded();
};

let debug = false; // Set to false to disable debugging

let lastKeyTime = 0; // Store the time of the last key press
let popupContainers = []; // Store all popup containers
let popupContainer; // Store the current popup container
let popups = []; // Store the current active popups
let popupTimers = []; // Store timers for clearing popups

/**
 * Function to apply initial styles
 */
function applyInitialStyles() {
  // Create a style element
  const style = document.createElement("style");

  // Set the type of the style element
  style.type = "text/css";

  // Define the CSS rules
  style.innerHTML = `
    .popup {
      font-family: ${config.popupFontFamily} !important;
      font-weight: ${config.popupFontWeight} !important;
      background-color: ${config.popupBackgroundColor} !important;
      color: ${config.popupFontColor} !important;
      font-size: ${config.popupFontSize}px !important;
      transition: opacity ${config.popupFadeInSeconds}s ease-in-out !important;
      max-width: ${config.popupTextMaxWidthInPercentage}% !important;
      border-radius: ${config.popupBorderRadius}px !important;
    }
    .popup.active {
      opacity: ${config.popupOpacity} !important;
    }
  `;

  // Append the style element to the head of the document
  document.getElementsByTagName("head")[0].appendChild(style);
}

/**
 * Handles the "keydown" event from.
 * getYAKCAPI gets called from preload.js.
 * @param {string} keyLabel - The label of the pressed key.
 */
getYAKCAPI.onClickEvent((keyLabel) => {
  const currentTime = Date.now();
  if (!config) {
    alert("failed to load config, please restart YAKC");
    return;
  }

  // If the key label is empty, do nothing
  if (keyLabel.length == 0) {
    return;
  }

  // Check if enough time has passed since the last key press
  if (currentTime - lastKeyTime > config.popupInactiveAfterSeconds * 1000) {
    // Remove old popups if there are any
    if (popupContainer) {
      removeOldPopups();
    }

    // Create a new popup container
    popupContainer = createPopupContainer();

    // Add the new popup container to the list
    popupContainers.push(popupContainer);

    // Create a new popup and add it to the list
    popups.push(createPopup(keyLabel));

    // Append the new popup to the popup container
    popupContainer.appendChild(popups[popups.length - 1]);
  } else if (popupContainer) {
    // If there is an existing popup, update its content
    popups[popups.length - 1].textContent += keyLabel;

    // Reset the popup timer
    resetPopupTimer(popups.length - 1);
  }

  // Update the last key press time
  lastKeyTime = currentTime;

  // Perform text-to-speech if enabled
  if (config.textToSpeech) {
    if ("speechSynthesis" in window) {
      const synth = window.speechSynthesis;
      const utterance = new SpeechSynthesisUtterance(keyLabel);

      // Cancel the current speech if enabled
      if (config.textToSpeechCancelSpeechOnNewKey) {
        if (utterance && synth.speaking) {
          synth.cancel();
        }
      }

      // Speak the key label
      synth.speak(utterance);
    } else {
      console.log("Text-to-speech is not supported in this browser.");
    }
  }
});

/**
 * Creates a new popup container and appends it to the body.
 * If there are more than 5 popup containers, the oldest one is removed.
 * @returns {HTMLElement} The newly created popup container.
 */
function createPopupContainer() {
  // Create a new div element for the popup container
  const popupContainer = document.createElement("div");
  popupContainer.classList.add("popupContainer");

  // Append the new container to the body
  document.body.appendChild(popupContainer);

  // Check if there are more than 5 containers
  if (popupContainer.length > 5) {
    // Remove the oldest container
    const removedContainer = popupContainers.shift();
    document.body.removeChild(removedContainer);

    // Log the removal action if debug mode is enabled
    if (debug) {
      logAction("Removed the oldest popup container", "red");
    }
  }

  // Log the creation action if debug mode is enabled
  if (debug) {
    logAction("Created a new popup container", "green");
  }

  // Return the newly created container
  return popupContainer;
}

/**
 * Creates a popup element with the given text.
 * @param {string} text - The text to display in the popup.
 * @returns {HTMLElement} - The created popup element.
 */
function createPopup(text) {
  // Create a new div element for the popup
  const popup = document.createElement("div");

  // Add the 'popup' and 'active' classes to the popup element
  popup.classList.add("popup", "active");

  // Set the text content of the popup element to the given text
  popup.textContent = text;

  // Get the current length of the popups array
  const popupIndex = popups.length;

  // Set a timer to remove the popup after X seconds of inactivity
  popupTimers.push(
    setTimeout(() => {
      removePopup(popupIndex);
    }, config.popupRemoveAfterSeconds * 1000)
  );

  // Return the created popup element
  return popup;
}

/**
 * Resets the popup timer for a given index.
 * If the index is valid, it clears the previous timer and sets a new one.
 * If the index is invalid, it does nothing.
 *
 * @param {number} index - The index of the popup timer to reset.
 */
function resetPopupTimer(index) {
  // Check if the index is within the bounds of the popupTimers array
  if (index >= 0 && index < popupTimers.length) {
    // Clear the previous timer for this index
    clearTimeout(popupTimers[index]);

    // Set a new timer for this index
    popupTimers[index] = setTimeout(() => {
      // After the specified time, remove the popup
      removePopup(index);
    }, config.popupRemoveAfterSeconds * 1000);
  }
}

/**
 * Removes a popup from the DOM and clears its associated timer.
 * @param {number} index - The index of the popup to remove.
 */
function removePopup(index) {
  // Check if index is within bounds and a popup exists at that index
  if (index >= 0 && index < popups.length && popups[index]) {
    const popup = popups[index];
    popup.classList.remove("active");

    // Add event listener for transition end
    popup.addEventListener("transitionend", function () {
      // Remove the popup from the DOM if it exists
      if (popup && popup.parentNode) {
        popup.parentNode.remove();
        if (debug) {
          logAction("Removed parent of transitioned (fade out) popup", "red");
        }
      }
    });

    // Clear the timer associated with this popup
    clearTimeout(popupTimers[index]);
  }
}

/**
 * Removes old popups from the DOM.
 * It iterates over the popupTimers array and removes any popup that is not active.
 */
function removeOldPopups() {
  // Iterate over the popupTimers array
  for (let i = 0; i < popupTimers.length; i++) {
    // Check if the popup at index i is not active
    if (!popups[i].classList.contains("active")) {
      // Remove the popup at index i
      removePopup(i);
    }
  }
}

/**
 * Logs a debug message to the console.
 *
 * @param {string} message - The message to log.
 * @param {string} color - The color of the message.
 */
function logAction(message, color) {
  // Check if debug mode is enabled
  if (debug) {
    // Log the message with the specified color
    console.log(`%c${message}`, `color: ${color}`);
  }
}
