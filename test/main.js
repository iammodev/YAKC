const { generateKeyLabel } = require("../src/main.js");

const { expect } = require("chai");

// Testing for modifier key
it("generateKeyLabel - modifier key", () => {
  const event = {
    rawcode: 16,
    shiftKey: true,
    ctrlKey: false,
    altKey: false,
    metaKey: false,
  };
  const config = { onlyKeysWithModifiers: true };
  expect(generateKeyLabel(event, config)).to.be.empty;
});

// Testing for Ctrl key
it("generateKeyLabel - Ctrl key", () => {
  const event = {
    rawcode: 65,
    shiftKey: false,
    ctrlKey: true,
    altKey: false,
    metaKey: false,
  };
  expect(generateKeyLabel(event)).to.equal(" CTRL + A ");
});

// Testing for Shift key
it("generateKeyLabel - Shift key", () => {
  const event = {
    rawcode: 65,
    shiftKey: true,
    ctrlKey: false,
    altKey: false,
    metaKey: false,
  };
  expect(generateKeyLabel(event)).to.equal(" A ");
});

// Testing for Alt key
it("generateKeyLabel - Alt key", () => {
  const event = {
    rawcode: 65,
    shiftKey: false,
    ctrlKey: false,
    altKey: true,
    metaKey: false,
  };
  expect(generateKeyLabel(event)).to.equal(" ALT + A ");
});

// Testing for Meta key
it("generateKeyLabel - Meta key", () => {
  const event = {
    rawcode: 65,
    shiftKey: false,
    ctrlKey: false,
    altKey: false,
    metaKey: true,
  };
  expect(generateKeyLabel(event)).to.equal(" META + A ");
});

// Testing for Space key
it("generateKeyLabel - Space key", () => {
  const event = {
    rawcode: 32,
    shiftKey: false,
    ctrlKey: false,
    altKey: false,
    metaKey: false,
  };
  expect(generateKeyLabel(event)).to.equal(" ");
});

// Testing for other keys
it("generateKeyLabel - other keys", () => {
  const event = {
    rawcode: 49,
    shiftKey: false,
    ctrlKey: false,
    altKey: false,
    metaKey: false,
  };
  expect(generateKeyLabel(event)).to.equal("1");
});
