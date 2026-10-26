### Fixed

- Interior lights: the colours the game stores are linear and were converted from sRGB a second
  time, which made the lights too dark; they are written as stored. A scene node that holds a light
  and a model or children is no longer rotated by the light's quarter turn: the light sits on a
  child node.
