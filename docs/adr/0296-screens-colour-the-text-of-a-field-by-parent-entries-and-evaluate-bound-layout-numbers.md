# Screens colour the text of a field by parent entries and evaluate bound layout numbers

Decided for issue #952. Amends ADR-0287 (the text a text field holds) and ADR-0110 (bound fields
at rest).

## Context

With ADR-0287 the powered MFD of the Avenger Titan (`m_mfd_screen`, `--var powerstate=1`) drew 52
shapes and 4 texts on LIVE, 16 texts without a colour. The style entries that colour them, such as
`Blue Objects` (`FillColor` for a widget whose parent has a tag and lacks another), select by
`Parent` and name no `Type`, so the rule of ADR-0287 (only entries that require the type `Text`)
left them out. A text field has no child widget in the Records; the only widget below it is the
text it draws, so an entry that selects "a child of a widget with tag T" can only reach the text
where the field carries T.

The `SizeX`, `SizeY`, anchor, pivot and scale fields that canvases bind to variables were listed as
`bound layout fields ... are not evaluated at rest`. In the 191 canvases below `mfdscreens` 302
`SizeX`, 288 `SizeY`, 247 `AnchorX`, 275 `AnchorY`, 213 `PivotX` and 204 `ScaleY` fields are
bound (counted over the exported records).

## Decision

- **A parent entry reaches the text.** An entry is tried against the text of a text field, with
  the field as its `Parent`, when some selector list of it requires the type `Text` (ADR-0287) or
  requires a `Parent`, directly or inside an all-of, and names no type of the widget itself.
  Entries that name another type, select by tags or counts alone, or are inline styles of the
  widget are still left to the field. Of a selector list that does not select the text, the
  entry's other lists are not tried either. Only the modifiers a text draws with apply (fill,
  stroke, background, text settings), never geometry, activity, alpha or a canvas. They apply to
  the field after those that select the field itself, the text winning where both set one.
- **Bound layout numbers are applied.** A `NumberField` or `IntegerField` that binds a number field
  of the layout (`Size`, `MinSize`, `MaxSize`, `Anchor`, `Pivot`, `Scale`, `Orientation`,
  `Padding`, `Margin` with their axes or sides, `PosX`, `PosY`, `PosXOffset`, `PosYOffset`, the
  border radii, the flex spacing and proportions) takes the value its graph gives at rest, as for
  `Alpha`. The value sits between the field of the widget and the style entries: a style entry or
  an instance override beats it, it beats the field. A size keeps how the widget reads it (fixed,
  percent, of the other axis); an automatic size becomes fixed, as for a style entry.
- **Still not evaluated.** Fields that set a behaviour, a depth (`PosZ`, `SizeZ`) or a flex setting
  stay listed as `bound layout fields (behaviours, depth, flex) are not evaluated at rest`. A bound
  number whose graph cannot be decided is listed as `the input of the bound field <field> is not
  evaluated: <why>` and the widget keeps its authored value. So does a value that is not a
  number, not finite, beyond 1e6 in size, or negative for a size or a bound of one.

## What this makes of the screens on LIVE (`--brand aegs`)

Counts over the same build before and after, textures, fonts and localisation on. The 66 screens
of six ships (Avenger Titan, Aurora, Hornet Mk II, Cutlass Black, 100i, Freelancer) were compared
as well: on 22 of them a count improved (more shapes or texts, fewer things left out), on 43 none
changed, and on none did a count of what is left out rise. One screen draws fewer shapes:
`hc_hud_ship_velocity_ball_master` for Drake, 5 before and 1 after, while it leaves out 8 fewer
things (16 before, 8 after). Its bars are bound to `linearvelocity` and `input_rawlinear`, which
are zero at rest, so they have no length at rest; this is kept as the faithful state.

| Screen | Mode | shapes before / after | texts before / after | left out before / after |
| --- | --- | --- | --- | --- |
| `m_mfd_screen` | rest | 0 / 0 | 0 / 0 | 0 / 0 |
| `m_mfd_screen` | `powerstate=1` | 52 / 85 | 4 / 13 | 140 / 94 |
| `m_eng_mfdcontent` | rest | 0 / 0 | 0 / 0 | 179 / 134 |
| `gen_mc_c_misl` | rest | 19 / 19 | 3 / 3 | 23 / 21 |
| `gen_mc_s_commslistitem` | rest | 5 / 9 | 0 / 1 | 9 / 8 |
| `gen_mc_s_configheader` | rest | 0 / 0 | 0 / 0 | 0 / 0 |
| `mapdisplaymaster` | rest | 33 / 33 | 0 / 0 | 75 / 59 |
| `h_hud_v_ptargeting_aft_text` | rest | 1 / 1 | 0 / 0 | 10 / 10 |
| `vehiclenameserial_black_civ` | rest | 34 / 34 | 2 / 2 | 0 / 0 |

The powered MFD draws the header `ORDNANCE > ORDNANCE` and the percentage counters. Its root
content is laid out under the bound size of `m_eng_mfdcontent` (1 or 0.75 of the parent). The two
large light-blue segments keep their authored size; none of the bound numbers of the canvases they
sit in sets it.

## Comparison with the game (owner screenshots of the Avenger Titan and Gladius MFDs, LIVE)

The screenshots (looked at for the Gladius power and scanning pages; the other pages from the
owner's written description) show cyan lines and text on a near-black screen, no large solid light-blue block
anywhere, and on every page a bottom tab bar `< PAGE NAME >`. Against that, the powered
`m_mfd_screen` render:

- Matches: the palette of the text and lines (cyan on a transparent background that reads as dark),
  the header text row, the percentage counters, thin frames and bars.
- Does not match: two large solid light-blue blocks at the bottom corners, the cast backdrops of
  the self-status page (`MainCastBackgroundGreeble` shapes and the `Left/RightSegmentColor`
  entries, which fill a custom shape with the base colour). In the game they are not solid, so
  either the fill is not meant for them, or the page the game shows hides them (the entry
  `Background isCast` takes a widget out by tags). The data read in this change does not say which.
- Does not match: no bottom tab bar `< PAGE NAME >` can be made out in the render, and a page is
  not chosen:
  the render shows the union of what the default state instantiates, not one page such as POWER
  MANAGEMENT or WEAPON CONFIG, so the page contents (readouts, bars, rows, pips) cannot be
  compared page by page yet.

The RSI Constellation MFDs use another brand style: pale lavender text and frames, bold wide
capitals, orange accents for selected buttons, warnings and toggles, angled frames, slightly
darker grey panels and a bottom bar `« PAGE NAME »`. The tables here use the Aegis brand
(`--brand aegs`). The same powered `m_mfd_screen` with `--brand rsi` draws 113 shapes and 14
texts (8 left out); its palette and layout were not compared with the screenshots.

These gaps are larger than this issue and are follow-ups: the cast blocks, the tab bar and the page
choice by the variables of the MFD.

## Guesses, unverified against the game

The order of precedence for the layout of a widget is: the field of the widget, then the bound
number, then the style entries, then the overrides of its instance (a clone's modifiers). That
the text of a field is the only widget a parent entry can reach below the field; that a
bound number at rest takes the value of the graph with every game variable at zero or at the
`--var` setting, so a bar bound to a velocity has no length; that an automatic size bound to a
number becomes fixed.

## Considered Options

- Applying every entry to the text (the first version of #930): drew 85 shapes and 13 texts on
  the MFD but also entries meant for other widgets, which a text has no business with.
- Leaving the turn (`Orientation…`) out: the one screen that draws fewer shapes loses them
  through bound sizes alone, and the turn resolves further bindings on the radar and
  map screens (`bb_radar_c_app_starmap` leaves out 61 instead of 49 things without it).
