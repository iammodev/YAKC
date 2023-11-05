/**
 * Title: Keyboard Layout: English
 * Credits: [Mo Devecioglu](https://github.com/iammodev/)
 * Date: 05/11/2023
 */

const keyboardLayout = {
  default: {
    // Alphabetical keys Start
    65: "a",
    66: "b",
    67: "c",
    68: "d",
    69: "e",
    70: "f",
    71: "g",
    72: "h",
    73: "i",
    74: "j",
    75: "k",
    76: "l",
    77: "m",
    78: "n",
    79: "o",
    80: "p",
    81: "q",
    82: "r",
    83: "s",
    84: "t",
    85: "u",
    86: "v",
    87: "w",
    88: "x",
    89: "y",
    90: "z",
    186: ";",
    187: "=",
    188: ",",
    189: "-",
    190: ".",
    191: "/",
    219: "[",
    220: "\\",
    221: "]",
    222: "'",
    226: "\\",
    // Alphabetical keys End

    // Modifiers keys Start
    16: "Shift", // for some odd reason, iohook returns rawcode 160 instead of 16
    17: "Ctrl", // for some odd reason, iohook returns rawcode 162 instead of 17
    18: "Alt", // for some odd reason, iohook returns rawcode 164 instead of 18
    91: "Meta",
    // 92: "Meta", // MetaRight
    // Modifiers keys End

    // F-keys Start
    112: "F1",
    113: "F2",
    114: "F3",
    115: "F4",
    116: "F5",
    117: "F6",
    118: "F7",
    119: "F8",
    120: "F9",
    121: "F10",
    122: "F11",
    123: "F12",
    124: "F13",
    125: "F14",
    126: "F15",
    127: "F16",
    128: "F17",
    129: "F18",
    130: "F19",
    131: "F20",
    132: "F21",
    133: "F22",
    134: "F23",
    135: "F24",
    // F-keys End

    // Numeric keys Start
    48: "0",
    49: "1",
    50: "2",
    51: "3",
    52: "4",
    53: "5",
    54: "6",
    55: "7",
    56: "8",
    57: "9",
    // Numeric keys End

    // Numpad keys Start
    96: "Numpad0",
    97: "Numpad1",
    98: "Numpad2",
    99: "Numpad3",
    100: "Numpad4",
    101: "Numpad5",
    102: "Numpad6",
    103: "Numpad7",
    104: "Numpad8",
    105: "Numpad9",
    144: "NumLock",
    111: "NumpadDivide",
    106: "NumpadMultiply",
    109: "NumpadSubtract",
    107: "NumpadAdd",
    // 13: "NumpadEnter", // Commented out because it conflicts with the Enter key
    110: "NumpadDecimal",
    // Numpad keys End

    // Navigation keys Start
    33: "PageUp",
    34: "PageDown",
    35: "End",
    36: "Home",
    37: "ArrowLeft", // ArrowLeft
    38: "ArrowUp", // ArrowUp
    39: "ArrowRight", // ArrowRight
    40: "ArrowDown", // ArrowDown

    // Navigation keys End

    // Miscellaneous keys Start
    8: "Backspace",
    9: "Tab",
    13: "Enter",
    19: "Pause",
    20: "CapsLock",
    27: "Escape",
    32: "Space",
    44: "PrintScreen",
    45: "Insert",
    46: "Delete",
    93: "ContextMenu",
    // Miscellaneous keys End
  },
  shift: {
    48: ")", // Digit0
    49: "!", // Digit1
    50: "@", // Digit2
    51: "#", // Digit3
    52: "$", // Digit4
    53: "%", // Digit5
    54: "^", // Digit6
    55: "&", // Digit7
    56: "*", // Digit8
    57: "(", // Digit9
    186: ":",
    187: "+",
    188: "<",
    189: "_",
    190: ">",
    191: "?",
    192: "~",
    219: "{",
    220: "|",
    221: "}",
    222: '"',
    226: "|",
    // 187: "*",
    // 191: "'",
    // 192: "~",
    // 219: "?", // Minus
    // 220: "°", // Backquote
    // 221: "`", // Equal
    // 226: ">", // IntlBackslash
  },
};

module.exports = keyboardLayout;
