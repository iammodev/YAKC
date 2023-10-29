const maps = require("./maps.js");

let defaultMapName = "us";

module.exports = {
  setMap: (name) => {
    defaultMapName = name.toLowerCase();
    return this;
  },
  map: (value) => {
    const map = maps[defaultMapName];
    if (!map) return "";

    const key =
      typeof value === "number"
        ? value
        : value
        ? value.which
        : this.event.which;
    return map && map[key] ? map[key] : "";
  },
};
