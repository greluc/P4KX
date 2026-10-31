# Screens enter the canvas a style or binding picks and leave render targets undrawn

Decided for issue #912. Amends ADR-0110 (`CanvasReferenceRecord` is no longer out of scope) and
ADR-0259 (namespaces).

## Context

With `--var powerstate=1` the MFD of the Avenger Titan instantiated 48 widgets in 6 canvases and
drew nothing (ADR-0259). What an MFD shows is chosen by the field `CanvasReferenceRecord` of its
canvas widgets.

Found on LIVE with `p4kx dcb export` over the Records below `ui/buildingblocks`:

- **Style entries.** 511 modifiers `FieldModifierRecordRef` whose `field` is a
  `…RecordRefTypeCanvasReferenceRecord` with a `value` reference to a canvas, in default, brand and
  embedded styles (the Titan MFD: header types, comms overlays, the self, target and nav screens
  per manufacturer).
- **Bindings.** 87 field bindings set `CanvasReferenceRecord` (`BindingsStringField`). 13 of the
  graphs end in `BindingsStringFromBooleanCanvas` (`isTrue`, `isFalse` canvas references and a
  boolean input); the rest read a string variable that holds a Record path at run time.
- **Namespaces.** A variable operation has `binding`, `inheritsNamespace` and an optional
  structured `path`. No canvas, widget or binding Record holds the namespace that a relative
  binding resolves against: the game host sets it at run time. The only static unit that reads a
  variable is the canvas whose `operations` hold it.
- **Render targets.** The star map of the radar canvas is a `WidgetHoloVolume` (or `WidgetWindow`)
  inside `mapdisplaystarmap_holovolume` and `_window`; the game draws it into a texture. The
  Records hold the widget and its lights, not the picture.

## Decision

- **A canvas widget holds the canvas that a style entry picks, else the one a binding picks, else
  its own field.** The order is that of `IsActive` and `Alpha` (ADR-0110). A style entry puts
  `Modifiers::canvas` in force through the cascade, `StringFromBooleanCanvas` is evaluated with
  the boolean graph of ADR-0110, and the chosen canvas is entered through the usual nesting with its
  cycle and depth limits. A widget that holds a canvas by one of these is a widget with children
  for the layout.
- **A canvas that only run-time data names is not entered.** A `CanvasReferenceRecord` binding whose
  input is anything but `StringFromBooleanCanvas`, or whose chosen branch names no canvas, keeps the
  field of the widget and is listed as `Binding`: `the input of the bound field
  CanvasReferenceRecord is not evaluated: …`.
- **The survey reaches the swapped-in canvases.** Every canvas a style entry, a boolean binding or
  an override sheet can put in a widget counts as reachable, so that a variable behind it is
  known and the selectors that need the shape of the tree see it.
- **Render targets are not drawn and no placeholder is painted.** A screen that the game renders
  into a texture (`WidgetHoloVolume`, `WidgetWindow`, `GeometryEntityWidget`,
  `VehicleEntityWidget`, `WidgetDustVolume`, `WidgetRuntimeImage`) keeps its box as for every widget
  and is listed with its element and name (`3D content is drawn by the game at run time`). A
  placeholder would need a colour and a form that no Record holds, and a viewer would take it for
  game content; the report already says what is missing and the picture stays faithful to the
  Records. A viewer that wants the content opens the hologram or the model of the Entity.
- **A setting names a namespace by the canvas that reads the variable.** `CANVAS::NAME=VALUE`
  limits the setting to the bindings of the canvas whose Record path ends in `CANVAS`, case
  ignored (`mapdisplaymaster::isradar=1`). `NAME` follows the rules of ADR-0259 (last segment, or a
  whole path with `/`). A namespace has no `/`, is not empty and counts to the 128 bytes of the
  name. A setting without namespace reaches every canvas, as before. A setting whose namespace no
  canvas of the screen has, or whose canvas does not read the name, is `unknown` in the report
  with the namespace in its name. The canvas stands for the namespace the game sets at run time,
  which no Record holds; two canvases of the same name are not told apart. A setting holds at most
  one `::`: `a::b::c` and `a:::b` are refused.
- **The UI tab keeps the state and the typed variables.** `[ui] power` (`at-rest`, `powered-on`)
  and `variables` (the typed text) are stored in the settings file next to size, background and
  format. The text is stored when the field is left after a change, and again when the window
  closes. It is held to `MAX_VARIABLES_BYTES` (4096) bytes, in the field and when read back: a
  longer text is cut at the last blank or comma before the limit, so that no setting is cut in
  two, and becomes empty when it has none. An unknown state reads as at rest. The typed text is
  parsed as before, with the bounds of ADR-0259.

## What this makes of the Avenger Titan on LIVE (`--brand aegs`, `powerstate=1`)

`m_mfd_screen` enters 18 canvases with 206 widgets instead of 6 with 48. The picture is still
empty: the 206 widgets get their colours from state tags (`PrimaryStateTag`, bound, not evaluated:
ADR-0110) and their pictures from textures of `S42/MFDs/AEGS`, which LIVE does not hold. The radar
canvas is unchanged: the canvas of its star map is chosen by run-time data.

## Considered Options

- Evaluating state tags too: needed for colours and the next step to a rich MFD, but a feature of
  its own (3300 bound fields, the tag hierarchy, animations); filed as a follow-up.
- A hatched placeholder for render targets: invents a look; the report lists them already.
- Persisting the typed text on every key: writes the settings file per key press; leaving the field
  is enough.
- Naming the namespace by its path (`StarMapData/CommonData/IsRadar`): the Records name only the
  tail of a relative binding; ADR-0259 already takes whole paths with `/`.
