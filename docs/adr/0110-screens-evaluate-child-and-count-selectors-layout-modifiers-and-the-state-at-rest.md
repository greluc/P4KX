# Screens evaluate Child and Count selectors, layout modifiers and the state at rest

Decided for issue #539, on top of ADR-0089 (the cascade), ADR-0101 (texts), ADR-0105 (images) and
ADR-0106 (layouts, clones and the geometry modifiers). Nothing here was observed in the game;
every rule is read from the Records and from what the names of the fields say, and the guesses are
listed at the end.

## What was found, on LIVE with `p4kx dcb export` over the 4995 Records below `ui/buildingblocks`

- **Selectors.** `Child` (140 uses) holds `conditions` that one child of the widget meets; they
  are tags (100), types (39), any-of tags (19), not-tags (6) and, five times, a `Parent`.
  `Count` (63 uses) has `type`, `count` and `offset`: `First` (49) with a count of 1 or 2 and an
  offset from 0 to 3, and `Every` (14) with a count of 2, 4 or 8. `Ancestor` (12969 uses) has
  `breakConditions`, which 8719 of them fill; the `Count` that the first Records show sits inside
  an `Ancestor` with a break, and some `Ancestor` has a break but no conditions.
- **Modifiers that change the layout** and were not read yet: `MarginTop` to `MarginLeft`,
  `FlexGrowProportion`, `FlexShrinkProportion` and the offsets of the orientation (674 uses
  together), and the enumerated `WidthBehavior` and `HeightBehavior` (2836 uses: `Percent`,
  `Fixed`, `Auto`, `PercentOfX`, `PercentOfY`), which say how the amount of the `SizeX` and
  `SizeY` beside them is read, so that `SizeX 1` with `Percent` is a whole parent. 370 more set
  `FlexDirection`, `FlexWrap` and the justification and alignment of a flex container.
- **Bindings.** 2584 Records have `operations`, a graph whose nodes read game variables
  (`BindingsBooleanVariable`, `…NumberVariable`, `…IntegerVariable`, `…StringVariable`, which carry
  no value), component parameters (`…ComponentParameter` with a `defaultValue`) and calculate
  with them (`BooleanInvert`, `…EvaluateAnd`/`Or`, `…FromInteger`, `…FromNumber`,
  `…FromIntegerSwitch`, `NumberFromBoolean`, `…Arithmatic`, `NumberClamp`, ...), and field
  bindings (`…BooleanField`, `…NumberField`, `…IntegerField`, `…StringField`) that set a field of a
  widget from a node. The fields that are set are `IsActive` (5605), `PrimaryStateTag` (2489),
  `Alpha` (742), `Instantiated` (698), `SizeX`/`SizeY` (674, 672), the anchor, colours, `Disabled`
  and `ParamInput0` to `ParamInput4` of canvas widgets (1605), which are the values the nested
  canvas reads through its component parameters.
- **The Avenger Titan.** The MFD `m_mfd_screen` holds one canvas widget whose `Instantiated` is
  `powerstate != 0`. The annunciator binds the severity of each issue to `ParamInput0..4` of its
  nested canvas, `EnableBackground` to `IsActive` of the grey background, and `powerstate == 1` to
  `Instantiated` of its boot canvas. The radar binds hundreds of `IsActive` and `Instantiated`
  fields to variables of the star map.

## Decisions

- **`Child`** holds when some child of the widget meets all its conditions. The children are those
  the layout draws under it: its own children, and for a clone or list the instances of its
  template. A `Child` that holds a `Parent` or `Ancestor` is not evaluated (five uses), since the
  child would need the widget above it; it does not apply and is listed.
- **`Count`** selects by the place of the widget among its siblings, counted from 0 over all of
  them, hidden ones included. `First` selects the `count` siblings from `offset` on, `Every` each
  `count`-th sibling from `offset` on. A count of 0 or below selects nothing and values are held
  to 2^20. Rows of a list are siblings, so a `Count` colours rows. A `Count` of another type is not
  evaluated and is listed.
- **`Ancestor` with `breakConditions`** looks at the ancestors from the nearest outwards and stops
  at the first that meets the break conditions, without looking at that one.
- **Shape of the tree for the selectors.** Only canvases whose styles hold such a selector pay for
  it: the place of a widget and the subjects of its children are made when a style sheet in force
  uses `Count` or `Child`, and the cache of resolved styles and measured sizes tells widgets apart
  by place only then. The cascade budget of ADR-0089 counts every child a `Child` looks at.
- **Layout modifiers.** The geometry modifiers of ADR-0106 gain `Margin*`, `FlexGrowProportion`,
  `FlexShrinkProportion`, `OrientationXOffset`, `OrientationYOffset`, `OrientationZOffset`,
  `WidthBehavior` and `HeightBehavior`, and the enumerated `FlexDirection`, `FlexWrap`,
  `FlexAxisJustification`, `FlexCrossAxisJustification` and `FlexItemAlignment` for widgets that
  have a flex container. A behaviour keeps the amount of the size it replaces unless the entry
  sets one; `Auto` takes the size from the content. Numbers are finite and held to ±10^6.
- **The state at rest** is what the `operations` of a canvas give its widgets when every game
  variable has its zero value: false, 0, an empty text. Component parameters take the value the
  holding canvas widget names (`paramInputValues`) or binds to a `ParamInput` field, else their
  `defaultValue`. The graph is evaluated per canvas and per set of parameters, once, with the
  nodes listed above; a node that is not evaluated, an input that is not connected, a cycle, a
  chain deeper than 128 and a number that is not finite make the value unknown, and the widget
  keeps its own. `and` with a false input and `or` with a true input are decided without the
  unknown ones. It sets:
  - `Instantiated` false: the widget is not laid out and not drawn, nor are its children, its
    clone or list instances and its nested canvas;
  - `IsActive`: below a style entry that sets the field and above the field of the widget;
  - `Alpha`: held between 0 and 1, in the same order;
  - the `ParamInput` fields of canvas widgets, which the nested canvas reads.
  Other bound fields (state tags, sizes, anchors, colours, `Disabled`, paths, ...) are not
  evaluated and are listed per canvas as `Binding`: state tags and layout fields in one line each,
  any other field by its name.
- **The shape of the tree is a property of the screen.** Before the first canvas is laid out, every canvas that can be reached from it (and every `stylesheetOverride`) is read, at most 4096 of them, and the screen takes the place of the widgets and the children for `Child` when any of their sheets uses `Count` or `Child`. Resolved styles are told apart by the place and by the children of the widgets above, so a nested canvas whose sheet uses `Ancestor` with `Count`, or `Parent` with `Child`, sees the same tree wherever it is entered from.
- **Numbers that are not finite** never leave the graph: a node that makes one is unknown, so they reach no `Alpha` and no parameter.
- **Root widgets that a binding removes** are listed as `Binding` (`not drawn at rest: Instantiated is bound to game variables, which are zero at rest`, likewise `IsActive`), so that an empty picture says why.
- **Hidden widgets leave the layout.** A widget whose `IsActive` is false is not laid out, whether
  the field, a style entry or a binding says so. Until now a style entry hid a widget but left its
  room; the field always removed it.
- **`stylesheetOverride`.** A canvas widget may name a `BuildingBlocks_Style`. Its entries are
  laid over the sheets of the canvas the widget holds, after them so that they win, and stay in
  force for the canvases nested below. The resolved styles are told apart by the override, so the
  same canvas under two widgets takes the look of each. Its palette is not read, and a reference
  that no Record answers adds nothing.
- **Bounds.** At most 32 clones with modifiers are in force at once and one clone applies at most
  256 modifiers (`Limits::clones_in_force` and `Limits::modifiers_per_clone`); what is beyond is dropped and `truncated` says so. `Count`
  values, style numbers and the evaluation of the graph are bounded as above, and the evaluation
  of one state costs one visit of each node.

## What this makes of the Avenger Titan, `--brand aegs`

| Screen | Before | After |
| --- | --- | --- |
| `m_mfd_screen` | 100 widgets in 11 canvases, nothing drawn | 1 widget: `powerstate` is 0 and the content is not instantiated |
| `mapdisplaymaster` | 532 widgets in 58 canvases, 149 shapes; `Child` not evaluated once | 127 widgets in 10 canvases, 28 shapes |
| `h_eng_annunciator_master_left` | 32 widgets, 24 shapes | 30 widgets, 22 shapes: the grey background, `EnableBackground` being false |

## Still out of scope

The report lists the bound fields that are not evaluated (as `Binding`), the selectors that are not evaluated (as `StyleEntry`) and the root widgets that a binding takes out of the screen. It does not list the rest of this section: modifiers that no reader knows are dropped without a note, and so are the entries of a `stylesheetOverride` palette, animations, `CanvasReferenceRecord` modifiers, `LayoutPolicy` and the `Min`/`Max` behaviours.

State tags and the entries they select (`PrimaryStateTag` and the others, 3300 bound fields),
animations and transitions, the bound number fields of layout and colour (`SizeX`, `AnchorY`,
`FillColor`, ...), `CanvasReferenceRecord` (1022 modifiers and 12 bindings that swap a nested
canvas), the colour palette of a `stylesheetOverride`, the modular kit as components of their own
(its widgets are drawn as the Records say, with the styles above), `LayoutPolicy`
and the `Min`/`Max` behaviours as modifiers, game variables that have a value at rest other than
zero (the power state of a ship is 0 here, so its MFD is off), the nodes of the graph that are not
listed (localisation, timestamps, waveforms, colour and tag switches) and `IntegerFromNumber`.

## Guesses, unverified against the game

The reading of `Count` (`First`, `Every`, offset and count), the exclusive stop of
`breakConditions`, the order of a binding between a style entry and the field of the widget, the
zero value of the variables as the state at rest, and the removal of a hidden widget from the
layout by a style entry, that `ParamInput0` to `ParamInput4` of a binding name the parameters of the nested canvas by the same names, and the precedence of a `stylesheetOverride` over the styles of the
canvas.

## Considered Options

- Leaving variables unknown and evaluating three-valued: almost every graph reads a variable, so
  the screen would keep every state at once, which is what the issue wants to end.
- Letting a caller set the variables of a screen: the right way to show an MFD that is on, and a
  follow-up; the zero state is the one that needs no data.
- Running the graph at the cascade, per widget: the same graph would be read for every row and
  every nested instance; one evaluation per canvas and set of parameters is enough, since the
  results only depend on those.
