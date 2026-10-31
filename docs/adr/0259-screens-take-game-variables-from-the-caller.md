# Screens take game variables from the caller

Decided for issue #588. Amends ADR-0110 (the state at rest). ADR-0277 adds namespaces
(`CANVAS::NAME=VALUE`, a setting for the bindings of one canvas), the canvases that
`CanvasReferenceRecord` picks, and the persistence of the state and the typed variables in the UI
tab; the passages below that say otherwise are superseded by it.

## Context

ADR-0110 draws a screen with every game variable at zero. That is the faithful state of a cold
ship, and it makes the MFD of the Avenger Titan draw nothing: its content is instantiated only
when `/seatdashboard/powerstate` is not 0. A viewer needs to choose the state.

Found on LIVE with `p4kx dcb export` over the Records below `ui/buildingblocks`: a variable
operation (`BindingsBooleanVariable`, `…NumberVariable`, `…IntegerVariable` or
`…StringVariable`) has a `binding`, the path of the variable. Paths are absolute
(`/seatdashboard/powerstate`, `/MobiGlas/CursorScreenRelPos/x`) or relative to a namespace the
holding canvas widget sets (`ammocount`, `tempIndicator/maxTemp`,
`StarMapData/CommonData/IsRadar`).

## Decision

- **A caller sets variables.** `ScreenComposer::with_variables` and `StillRequest::variables` take
  `Variable`s; `p4kx ui render --screen --var NAME=VALUE` (repeatable) and the UI tab fill them.
  A variable that is not set keeps its zero, so nothing changes without `--var`.
- **A name is the last segment of the path.** `powerstate` matches `/seatdashboard/powerstate`
  and every other binding that ends in `powerstate`; a name with a `/` is the whole path, with or
  without the leading `/`. Case is ignored. Namespaces are not resolved, since the Records name
  them only at run time: one setting reaches every canvas that reads a variable of that name. The
  last setting of a name wins.
- **The value is read as the kind of variable.** A boolean takes `true`, `false` or a number
  (not 0 is true), a number or integer takes a finite number, a string takes the text as given.
  A value of another kind makes the bindings that read it undecided and lists them as `Binding`
  with the name, the value and the kind; the widget keeps its own value, as for any undecided
  binding. Nothing is evaluated beyond this reading.
- **Bounds.** At most 64 variables per composition (`ScreenError::TooManyVariables`), a name of at
  most 128 bytes and a value of at most 256, neither holding a control character, and a name that
  is more than `/` (`VariableError`). The command line refuses a malformed setting and a 65th
  setting with exit code 2, the count before it opens the Archive. The GUI reads at most 65 typed
  settings and the renderer refuses the 65th.
- **Unknown names are reported.** Before the first canvas is laid out, the survey of ADR-0110
  reads every canvas that can be reached, and notes the paths of their variable operations. A
  variable that no path matches is listed with `used: false` in `Report::variables`, in JSON as
  `variables`, on stderr as `unknown game variable: <name>`, and in the UI tab as a line of the
  report. It changes nothing and does not change the exit code: a name that one screen does not
  read is no failure of the render.
- **The report lists the variables in force**, the last setting of each name with its value as
  given. `variables` is absent from the JSON when nothing was set, so existing consumers and
  snapshots keep their shape.
- **A root widget a binding takes out of the screen** is listed as before; with variables set it
  reads `not drawn: <field> is bound to game variables, whose values leave it off`, since the
  variables are no longer all zero.
- **The UI tab** offers the state "At rest" and "Powered on" (the preset `powerstate=1`) beside
  size, background and format, and a field for more settings, `NAME=VALUE` separated by blanks or
  commas, which follow the preset and so win over it. Both apply to screens only and are chosen
  again in each session; the persisted render settings do not hold them. Settings that are no
  `NAME=VALUE` are shown below the field and ignored.

## What this makes of the Avenger Titan on LIVE

`--var powerstate=1` instantiates the content of the MFD but the picture stays empty, since what
an MFD shows is chosen at run time by `CanvasReferenceRecord` (ADR-0110, out of scope); the radar
canvas switches from its map to its radar branch with `isradar=1`, whose view is the star map
that the game draws into a render target.

## Considered Options

- Naming the whole path only: exact, but a user has to find `/seatdashboard/powerstate` in a
  Record first, and the relative paths hold a namespace that is not known.
- Evaluating the value as an expression: no need, and an attack surface on a command line that
  other programs may fill.
- A fixed list of presets in the CLI: `--var` covers them, and the GUI keeps the one preset the
  issue names.
