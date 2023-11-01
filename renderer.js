const ipcRenderer = require("electron").ipcRenderer;
const fs = require("fs");

// load config
let config = JSON.parse(fs.readFileSync("./config.json", "utf-8"));
const {
  popupTextMaxWidthInPercentage,
  popupOpacity,
  popupFadeInSeconds,
  popupFontSize,
  popupBorderRadius,
  popupFontColor,
  popupBackgroundColor,
  popupFontFamily,
  popupFontWeight,
  textToSpeech,
  textToSpeechCancelSpeechOnNewKey,
  position,
  topOffset,
  bottomOffset,
  leftOffset,
  rightOffset,
} = config;

let debug = false; // Set to false to disable debugging

let lastKeyTime = 0;
let popupContainers = []; // Store all popup containers
let popupContainer; // Store the current popup container
let popups = []; // Store the current active popups
let popupTimers = []; // Store timers for clearing popups

window.onload = () => {
  applyInitialStyles();
};

// Function to apply initial styles
function applyInitialStyles() {
  const style = document.createElement("style");
  style.type = "text/css";

  style.innerHTML = `
    .popup {
      font-family: ${popupFontFamily} !important;
      font-weight: ${popupFontWeight} !important;
      background-color: ${popupBackgroundColor} !important;
      color: ${popupFontColor} !important;
      font-size: ${popupFontSize}px !important;
      transition: opacity ${popupFadeInSeconds}s ease-in-out !important;
      max-width: ${popupTextMaxWidthInPercentage}% !important;
      border-radius: ${popupBorderRadius}px !important;
    }
    .popup.active {
      opacity: ${popupOpacity} !important;
    }
  `;

  document.getElementsByTagName("head")[0].appendChild(style);
}

ipcRenderer.on("keydown", (event, keyLabel) => {
  const currentTime = Date.now();
  if (currentTime - lastKeyTime > config.popupInactiveAfterSeconds * 1000) {
    if (popupContainer) {
      removeOldPopups();
    }
    popupContainer = createPopupContainer();
    popupContainers.push(popupContainer);
    popups.push(createPopup(keyLabel));
    popupContainer.appendChild(popups[popups.length - 1]);
  } else if (popupContainer) {
    popups[popups.length - 1].textContent += keyLabel;
    resetPopupTimer(popups.length - 1);
  }
  lastKeyTime = currentTime;

  if (textToSpeech) {
    if ("speechSynthesis" in window) {
      const synth = window.speechSynthesis;
      const utterance = new SpeechSynthesisUtterance(keyLabel);
      if (textToSpeechCancelSpeechOnNewKey) {
        if (utterance && synth.speaking) {
          synth.cancel();
        }
      }
      synth.speak(utterance);
    } else {
      console.log("Text-to-speech is not supported in this browser.");
    }
  }
});

function createPopupContainer() {
  const popupContainer = document.createElement("div");
  popupContainer.classList.add("popupContainer");
  document.body.appendChild(popupContainer);
  if (debug) {
    logAction("Created a new popup container", "green");
  }
  return popupContainer;
}

function createPopup(text) {
  const popup = document.createElement("div");
  popup.classList.add("popup", "active");
  popup.textContent = text;

  // Set a timer to remove the popup after X seconds of inactivity
  const popupIndex = popups.length;
  popupTimers.push(
    setTimeout(() => {
      removePopup(popupIndex);
    }, config.popupRemoveAfterSeconds * 1000)
  );

  return popup;
}

function resetPopupTimer(index) {
  if (index >= 0 && index < popupTimers.length) {
    clearTimeout(popupTimers[index]);
    popupTimers[index] = setTimeout(() => {
      removePopup(index);
    }, config.popupRemoveAfterSeconds * 1000);
  }
}

function removePopup(index) {
  if (index >= 0 && index < popups.length) {
    const popup = popups[index];
    if (popup) {
      popup.classList.remove("active");

      popup.addEventListener("transitionend", function () {
        if (popup && popup.parentNode) {
          popupContainers.shift();
          popup.parentNode.remove();
          if (debug) {
            logAction("Removed parent of transitioned (fade out) popup", "red");
          }
        }
      });
    }
  }
  if (popupContainers.length > 5) {
    const removedContainer = popupContainers.shift();
    document.body.removeChild(removedContainer);
    if (debug) {
      logAction("Removed the oldest popup container", "red");
    }
  }
}

function removeOldPopups() {
  for (let i = 0; i < popupTimers.length; i++) {
    if (!popups[i].classList.contains("active")) {
      removePopup(i);
    }
  }
}

function logAction(message, color) {
  if (debug) {
    console.log(`%c${message}`, `color: ${color}`);
  }
}
