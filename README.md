# YAKC - Yet Another Key Caster

![yakc-logo](https://github.com/iammodev/YAKC/assets/89686923/d776922e-ebb8-42b0-b49f-c516d52957ae)

YAKC is an open-source Platform-independent Electron-based keystroke visualizer.

## Features

- Highly customizable style options
- Flexible Positioning
- Show Modifiers only
- Toggle Keyboard/Mouse Click Visualization
- Force popup next line on screen percentage
- Smooth Transition (Fade In & Fade Out)
- Timing Control
- Spacebar as Unicode? ("␣")
- Text-To-Speech (perfect for blind people)

https://github.com/iammodev/YAKC/assets/89686923/61ebf7db-5171-4d5d-997b-59efd30218ec

## Prerequisites

- [Node.js](https://nodejs.org/) installed on your system.
- [Electron](https://www.electronjs.org/) for building and running the application.

## Installation

1. Clone the repository to your local machine:

```bash
git clone https://github.com/iammodev/YAKC.git
```

2. `cd YAKC`

3. `npm install`

## Usage

1. Start the application: `npm start`
2. The application will launch and display a system tray icon.
3. Upon pressing on any key, a popup will appear.

## Build

1. `npm install --save-dev electron-packager`

2. `npm run package:windows` (Options: package:windows, package:mac, package:linux, package:all)

3. run executable

## Configuration

The configuration file `config.json` allows you to customize the behavior of YAKC. Here are the available options:

- `showOnMonitor`: Specifies the monitor on which the pop-up text should be displayed. The value should be the index of the monitor (starting from 0).

- `popupTextMaxWidthInPercentage`: Specifies the maximum width of the pop-up text as a percentage of the screen width.

- `popupOpacity`: Opacity of the popup text. Range: ("0.0" - "1").

- `popupFadeInSeconds`: Duration of Fade effect for the popup text.

- `popupRemoveAfterSeconds`: Remove inactive popup after X seconds.

- `popupInactiveAfterSeconds`: After X seconds, a new popup will be created upon key/mouse press.

- `popupFontSize`: Font size of popup text.

- `popupFontColor`: Font color of popup text.

- `popupFontFamily`: Font Family for the text.

- `popupFontWeight`: Font weight of popup text.

- `popupBorderRadius`: Corner border radius. Use "0" for sharp corners.

- `popupBackgroundColor`: Background color of popup text.

- `showMouseClick`: Show mouse clicks. Options: true, false.

- `showMouseCoordinates`: Show mouse coordinates. Options: true, false.

- `showKeyboardClick`: Show Keyboard clicks. Options: true, false.

- `onlyKeysWithModifiers`: Show only keys with modifiers. Options: (true, false).

- `showSpaceAsUnicode`: Show space as Unicode character (␣). Options: (true, false).

- `textToSpeech`: Speak out loud every Keystroke. (perfect for blind people!). Options: (true, false).

- `textToSpeechCancelSpeechOnNewKey`: Cancels current SpeechToText if new Keystroke pressed. Options: (true, false).

- `position`: Position of popup on screen. Options: "top-left", "top-right", "bottom-left", and "bottom-right".

- `topOffset`: Top offset of the popup text in pixels.

- `bottomOffset`: Bottom offset of the pop-up text in pixels.

- `leftOffset`: Left offset of the pop-up text in pixels.

- `rightOffset`: Right offset of the pop-up text in pixels.

## TODO:

- [ ] position (top-left, top-right, bottom-left, bottom-right)
- [ ] topOffset, bottomOffset, leftOffset, rightOffset
- [ ] GUI to easily configure at runtime
- [ ] Drag and Drop the popup to desired position
- [ ] Add hotkey for start/stop listening to keystrokes
- [ ] Reliable solution for all/common keyboard layout's (please contact me if you have knowledge about this)
- [ ] Add unit tests

## Related

- [iohook](https://github.com/mechakeys/iohook) for capturing keyboard & mouse input.
- [keycode](https://github.com/timoxley/keycode) for converting keycodes.

## Contributing

Contributions are welcome! If you find any issues or have suggestions for improvements, please open an issue or submit a pull request.

## Security Assurance

YAKC is free and open source. YAKC operates independently without any network interactions. Your private information, including passwords, is never stored or shared by YAKC, guaranteeing your safety and privacy.

**Please Exercise Caution**: When using YAKC for activities like presentation, recording or streaming, be mindful not to inadvertently share sensitive information. Always ensure your privacy and the security of any confidential data.

## License

This project is licensed under the `MIT` License. See the [LICENSE](LICENSE) file for details.
