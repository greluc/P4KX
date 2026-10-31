# Screens evaluate state tags, the text a text field holds and settled animations

Decided for issue #930. Amends ADR-0110 (state tags, animations) and ADR-0277 (the MFD stays
empty).

## Context

With the canvas swaps of ADR-0277 the MFD of the Avenger Titan (`m_mfd_screen`,
`--var powerstate=1`) entered 18 canvases and 206 widgets on LIVE and drew nothing: 0 shapes, 0
texts, 0 images.

Found on LIVE with `p4kx dcb export` over the Records below `ui/buildingblocks/ships/displays/
mfdscreens` (191 canvases):

- **State tag bindings.** 248 `BindingsStringField` operations set `PrimaryStateTag` (182),
  `SecondaryStateTag` (45), `TertiaryStateTag` (17), `QuarternaryStateTag` and `QuinaryStateTag`
  (2 each). Their inputs are `BindingsTagFromBoolean` (186; `isTrue` and `isFalse` name a tag, or
  `inputTrue` and `inputFalse` name a node that gives one), `BindingsTagFromIntegerSwitch` (38; a
  list of `IntegerTagPair` and a `defaultValue`, the integer pairs stored as 64-bit integers),
  `BindingsStringComponentParameter` (22; a nested canvas receives the tag through a
  `ParamInput`) and, twice, nothing.
- **Tags are the tags of the styles.** The tags are `UI.Style…` tags (`UI.Style.__Internal.selected`,
  `UI.Style.General.Secondary`, `UI.Style.ShipEvents.PowerState.Normal`, ...), the same that
  `styleTags` and the `Tag` selectors of style entries use. No widget field holds a state tag.
- **Text selectors.** 300 selectors have the type `Text`, none the type `TextField`, while the
  canvases hold 532 `WidgetTextField` and no `WidgetText`. The entries read `Type Text` with a
  `Parent` that holds a tag (`Body Text Colour`: `Type Text` below `Text_Body`), and the tag sits
  on the text field itself, whose own parent, a card, has none.
- **Animations.** An entry such as `PowerOnAnim` (`PowerState.Normal` or `.Warning`) holds no
  plain modifier, only an animation of `Alpha` from 0 to 1 with `fillMode` `Forward`, while the
  authored `Alpha` of the root of `m_eng_mfdcontent` is 0. `PowerOffAnim` (`PowerState.Off`)
  animates it the other way.

## Decision

- **A bound state tag is a tag of its widget.** The `…StateTag` fields of a canvas are evaluated
  with the graph of ADR-0110 (new nodes `TagFromBoolean` and `TagFromIntegerSwitch`; a tag node
  that gives no tag adds none) and the tags join the `styleTags` of the widget for every
  selector: `Tag`, `AnyOfTag`, `NotTag`, `Parent`, `Ancestor`, `Child` (the children carry theirs),
  below a tag the way a tag below its parent holds. The five slots are not told apart. A tag a
  nested canvas receives through a `ParamInput` is a string parameter that takes a tag; a
  string parameter that holds a non-empty text, a graph that is not connected and any node not
  evaluated leave the widget without the tag and are listed as `Binding`: `the input of the
  bound field PrimaryStateTag is not evaluated: …`. The game's events (`ShipEvents`) are not
  bound by any Record and add no tag.
- **A text field holds a text.** For the selectors, a `WidgetTextField` is the field and, below it,
  the text it draws, of the type `Text`. The entries that select that text, with the field as
  its `Parent`, are applied to the field after the entries that select the field itself; where
  both set a modifier, the text wins.
- **An animation settles.** The animation of a style entry (`FieldModifierAnimation`) puts the
  modifiers of its last keyframe in force when its `fillMode` is `Forward` or `Both` and its
  direction is `Forward`; with the direction `Reverse` the first keyframe. An animation that
  loops, alternates, does not hold its frame or has no embedded keyframes adds nothing.
  Transitions stay out of scope.
- **Missing textures are listed per screen.** `Report::missing_textures` names each Archive path
  of a texture that could not be read, once, at most 256; `p4kx ui render` prints each as
  `texture not in the Archive: <path>`, and the JSON report carries the array when it is not
  empty. The per-widget note stays.
- **The asset listing follows the swaps.** `ScreenAssets` lists the canvases that a style entry
  (default, embedded, every brand, shared and inline styles) or a boolean binding of a canvas may
  put in a canvas widget, whatever the brand, with their textures, vectors, movies and keys.
- **Integers of 64 bits are read.** The reader of whole numbers takes `Int64` and `UInt64` that fit
  an `i32`.

## What this makes of the Avenger Titan on LIVE (`--brand aegs`, `powerstate=1`)

| `m_mfd_screen` | at rest | `powerstate=1` |
| --- | --- | --- |
| widgets in canvases | 1 in 1 | 206 in 18 |
| shapes | 0 | 85 |
| texts drawn | 0 | 13 (7 left out) |
| images drawn | 0 | 15 (7 left out) |

The PNG shows the header (`ORDNANCE > ORDNANCE`), the bars and frames of the self status and the
missile list; at rest it is empty. Three textures of `S42/MFDs/AEGS` are not in LIVE and are
listed. Some texts overlap: the layout of bound sizes and anchors is not evaluated (ADR-0110).

## Guesses, unverified against the game

That a state tag acts as an extra style tag of its widget and not as a state the game keeps apart
(hover, selected); that the five slots do not differ; that the text of a text field is a child of
the field for `Parent`, the only reading that fits the 300 `Type Text` selectors of the data; that
an animation that holds its last frame is in that frame at rest; that a text that wins over the
field is what the game draws.

## Considered Options

- Leaving animations out and drawing the first frame: the root of the MFD has `Alpha` 0 as
  authored, so every text and shape under it stays invisible and the screen draws nothing.
- Mapping the type `Text` to every text widget (`Type` of a `WidgetTextField` is `Text`): the
  `Parent` of the selectors would then be the card above the field, which holds no tag in any
  canvas looked at, so the colours stay unset.
- Telling the five state slots apart: no selector or Record does; the tags share one namespace.
- Applying the text entries to a child widget of the field in the layout: the layout would grow
  widgets the Records do not hold; the cascade of the field is enough.
