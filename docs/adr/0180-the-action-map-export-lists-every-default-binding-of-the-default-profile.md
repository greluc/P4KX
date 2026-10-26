# The action map export lists every default binding of the default profile

Decided for issue #314, on top of ADR-0159 (CSV cells) and ADR-0003 (layers).

## Decision

- `p4kx actionmap export` reads `Data\Libs\Config\defaultProfile.xml` (text XML or `CryXml`) in
  `p4kx-export` and writes JSON or CSV. The reader and the writer are library API
  (`read_action_maps`, `write_action_maps`), so the desktop app and the MCP server can use them.
- Only `action` elements inside an `actionmap` count. The `actiongroup` element lists action names
  without bindings or labels and is ignored.
- A binding is a device, an input and an activation mode. It comes from the `keyboard`, `mouse`,
  `gamepad` and `joystick` attributes of the action, from a device child element's `input`
  attribute, and from the `inputdata` elements inside a device child. The mode of a child is its
  own `activationMode`, else the action's. A blank input is no binding; the same device, input and
  mode appear once. Child elements add to the attributes: the profile has one action that states a
  binding both ways, and no action whose child must replace its attribute.
- The label is the text of the `UILabel` key in `--language` (English by default). An empty
  `UILabel` is no key. The one action map that misspells the attribute as `UILable` is read as
  `UILabel`. There is no fallback language: a key without a text leaves the label empty, so an
  export in one language never mixes two.
- The export states the label key and the label, never the `UIDescription` text. Descriptions are
  lore and stay out of data meant for third parties.
- JSON is one document with the language and the action maps, in document order. CSV has one row
  per action and a column per device; its text cells use `write_csv_cell` with the neutralisation of
  ADR-0159. There is no `--csv-raw`: an input or label that starts with `=`, `+`, `-` or `@` is
  rare, and a script that wants exact values reads the JSON.
- The command line text is English like every other command; this issue adds no string to the
  Fluent files of the desktop app.

## Considered Options

- One row per binding in the CSV: easier to filter by device, but it repeats the label and breaks
  the one-row-per-action shape that scdatatools and sc-keybind-extract use.
- Resolve labels with an English fallback: hides which language lacks a text.
