# YAKC - Yet Another Key Caster

![yakc-logo](https://github.com/iammodev/YAKC/assets/89686923/d776922e-ebb8-42b0-b49f-c516d52957ae)

**YAKC** is an **open-source** **cross-platform** **Key** & **Mouse** Click visualizer useful for **content creators, developers or presentations**!.

# Big News!

I decided to try to **migrate** from [Electron](https://www.electronjs.org/) to [NW.JS](https://nwjs.io). So I'll try to get this done **quickly**. I don't even know if it's **possible** yet, we'll see, what **problems** I will have to **solve**. **Wish me luck**! I am **LIVE** right now **[Twitch](https://www.twitch.tv/iammodev)** (Date: 11/8/2023).

## Features

![YAKC-edited](https://github.com/iammodev/YAKC/assets/89686923/1b650c0b-bf86-47f6-afad-cfc072eb59c9)

### Appereance

- Display **Key** & **Mouse** **clicks** and **coordinates**
- Choose between Keyboard layouts
- Highly customizable (size, opacity, text/background color, font, boldness, cornered/pointy corners)
- Position and Offset (**top-left**, **top-right**, **bottom-left**, **bottom-right**)
- Smooth fade **transition** upon **remove**

### Behaviour

- Display on **any** selected **Monitor**
- Keystroke display (**Modifiers only**, **Keystroke2Unicode**)
- **Text-to-speech** for each keystroke (perfect for blind people)

### User Security

- **Automatic**: **Start** capturing keystrokes when **selected process** is focused.
- **Manual**: **Right-click** the tray icon to **toggle** keystroke capturing.

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

4. `cd ./node_modules/@mechakeys/iohook`

5. `npm install`

6. `node install.js`

7. Go to [Usage](#usage)

## Usage

1. Make sure you are in folder ./YAKC/
2. Start the application: `npm start`
3. The application will launch and display a system tray icon.
4. Upon pressing on any key, a popup will appear.

## Build

1. `npm install --save-dev electron-packager`

2. `npm run package:windows` (Options: package:windows, package:mac, package:linux, package:all)

3. run executable

## Buy me a coffee

<a href="https://www.buymeacoffee.com/iammodev" target="_blank"><img src="https://www.buymeacoffee.com/assets/img/custom_images/orange_img.png" alt="Buy Me A Coffee" style="height: 37px !important;width: 170px !important;box-shadow: 0px 3px 2px 0px rgba(190, 190, 190, 0.5) !important;-webkit-box-shadow: 0px 3px 2px 0px rgba(190, 190, 190, 0.5) !important;" ></a>

[![ko-fi](https://ko-fi.com/img/githubbutton_sm.svg)](https://ko-fi.com/iammodev)

## Configuration

The configuration file `./resources/app/config.json` allows you to customize the behavior of YAKC. Here are the available options:

- `keyboardLayout`: Select Keyboard Layout. (options: english, german)

- `showOnMonitor`: Specifies the monitor on which the pop-up text should be displayed. The value should be the index of the monitor (starting from 0)

- `popupTextMaxWidthInPercentage`: Specifies the maximum width of the pop-up text as a percentage of the screen width

- `popupOpacity`: Opacity of the popup text. Range: "0.0" - "1"

- `popupFadeInSeconds`: Duration of Fade effect for the popup text

- `popupRemoveAfterSeconds`: Remove inactive popup after X seconds

- `popupInactiveAfterSeconds`: After X seconds, a new popup will be created upon key/mouse press

- `popupFontSize`: Font size of popup text

- `popupFontColor`: Font color of popup text

- `popupFontFamily`: Font Family for the text

- `popupFontWeight`: Font weight of popup text

- `popupBorderRadius`: Corner border radius. Use "0" for sharp corners

- `popupBackgroundColor`: Background color of popup text

- `showMouseClick`: Show mouse clicks. Options: true, false

- `showMouseCoordinates`: Show mouse coordinates. Options: true, false

- `showKeyboardClick`: Show Keyboard clicks. Options: true, false

- `onlyKeysWithModifiers`: Show only keys with modifiers. Options: true, false

- `showSpaceAsUnicode`: Show space as Unicode character (␣). Options: true, false

- `textToSymbols`: Change some Keystrokes to Unicode Characters (example: Tab to ↹). Options: true, false

- `textToSpeech`: Speak out loud every Keystroke. (perfect for blind people!). Options: true, false

- `textToSpeechCancelSpeechOnNewKey`: Cancels current SpeechToText if new Keystroke pressed. Options: true, false

- `position`: Position of popup on screen. Options: "top-left", "top-right", "bottom-left", and "bottom-right"

- `topOffset`: Top offset of the popup text in pixels

- `bottomOffset`: Bottom offset of the pop-up text in pixels

- `leftOffset`: Left offset of the pop-up text in pixels

- `rightOffset`: Right offset of the pop-up text in pixels

- `filter`: Whether to enable the filter function. Options: true, false

- `filterProcessName`: Process Name(s) to start listening to any keystrokes when process is focused. Options: ["app1.exe, app2.exe"]

- `filterCheckEverySecond`: How often to check if active window includes the `filter` Process(es)

## TODO:

- [x] Reliable solution for all/common keyboard layouts
- [x] Improve **user security** to **prevent** the display of **sensitive information**.
- [ ] position (top-left, top-right, bottom-left, bottom-right)
- [ ] topOffset, bottomOffset, leftOffset, rightOffset
- [ ] GUI to easily configure at runtime
- [ ] Drag and Drop the popup to desired position
- [ ] Add hotkey for start/stop listening to keystrokes
- [ ] Add unit tests

## Related

- [iohook](https://github.com/mechakeys/iohook) for capturing keyboard & mouse input.
- [active-win](https://github.com/sindresorhus/active-win) to get metadata about the focused process.

## Contributing

Contributions are welcome! If you find any issues or have suggestions for improvements, please open an issue or submit a pull request.

### Everyone can contribute!

We require an extensive collection of [Keyboard Layouts](./src/keyboardLayouts/) to accommodate a wide range of languages and input methods.

## Security Assurance

YAKC is free and open source. YAKC operates independently without any network interactions. Your private information, including passwords, is never stored or shared by YAKC, guaranteeing your safety and privacy.

**Please Exercise Caution**: When using YAKC for activities like presentation, recording or streaming, be mindful not to inadvertently share sensitive information. Always ensure your privacy and the security of any confidential data.

## License

This project is licensed under the `MIT` License. See the [LICENSE](LICENSE) file for details.
