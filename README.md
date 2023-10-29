# YAKC - Yet Another Key Caster

YAKC is a Platform-independent simple Electron-based application that displays key presses in real-time as popups.

## Features

- Display real-time on key press.
- Configurable styling options.
- Automatic removal of old popups.

## Prerequisites

- [Node.js](https://nodejs.org/) installed on your system.
- [Electron](https://www.electronjs.org/) for building and running the application.
- [iohook](https://github.com/mechakeys/iohook) for capturing keyboard & mouse input.

## Installation

1. Clone the repository to your local machine:

```bash
git clone https://github.com/iammodev/YAKC.git
```

## Usage

1. Start the application: `npm start`
2. The application will launch and display a system tray icon.
3. Start casting your keyboard keys as pop-up text on your screen.

## Build (TODO)

## Configuration

The configuration file `config.json` allows you to customize the behavior of YAKC. Here are the available options:

- `showOnMonitor`: Specifies the monitor on which the pop-up text should be displayed. The value should be the index of the monitor (starting from 0).
- `popupTextMaxWidthInPercentage`: Specifies the maximum width of the pop-up text as a percentage of the screen width.
- `popupOpacity`: Specifies the opacity of the pop-up text.
- `popupFadeInSeconds`: Specifies the duration of the fade-in effect for the pop-up text.
- `popupFontSize`: Specifies the font size of the pop-up text.
- `popupFontColor`: Specifies the font color of the pop-up text.
- `popupBackgroundColor`: Specifies the background color of the pop-up text.
- `position`: Specifies the position of the pop-up text on the screen. Valid values are "top-left", "top-right", "bottom-left", and "bottom-right".
- `topOffset`: Specifies the top offset of the pop-up text in pixels.
- `bottomOffset`: Specifies the bottom offset of the pop-up text in pixels.
- `leftOffset`: Specifies the left offset of the pop-up text in pixels.
- `rightOffset`: Specifies the right offset of the pop-up text in pixels.
- `showMouseClick`: Specifies whether to show mouse clicks as pop-up text.
- `onlyKeysWithModifiers`: Specifies whether to show only keys with modifiers as pop-up text.
- `showSpaceAsUnicode`: Specifies whether to show the space key as the Unicode character.

## Contributing

Contributions are welcome! If you find any issues or have suggestions for improvements, please open an issue or submit a pull request.

## License

This project is licensed under the `MIT` License. See the [LICENSE](LICENSE) file for details.
