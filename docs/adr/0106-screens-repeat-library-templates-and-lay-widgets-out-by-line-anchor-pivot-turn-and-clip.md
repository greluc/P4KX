# Screens repeat library templates and lay widgets out by line, anchor, pivot, turn and clip

Decided for issue #538, on top of ADR-0089 (screens) and ADR-0101 (texts). It closes the layout
gaps that `p4kx ui render --screen` listed: lists, clones and templates, stack, grid and wrapping
containers, pivot, anchor, rotation, overflow, bounds, sizes that come from the content and the
unit of raw canvases. Nothing here was observed in the game; every rule is read from the Records
and from what the names of the fields say, and the open ones are listed at the end.

## What was found, on LIVE with `p4kx dcb export` over the 4920 Records below `ui/buildingblocks`

- The `library` array of a canvas holds templates. 1336 widgets of the scenes have a parent
  `library[n]`: they are the children of the template. A `WidgetClone` (738) and a `WidgetList`
  (739) name a template or any widget of the canvas as `target`. A clone also has `modifiers`
  (262 uses of `FieldModifierPair`, all numbers such as `AnchorX`). A list has `arrayVariable`;
  80 lists name a `StaticVariableArray` of their canvas, whose columns list the values of the
  rows, and the others read an array that the game binds at run time.
- Layout policies: flex 4347, stack 313, grid pack 25, spacer 3. Flex containers wrap (`Wrap`
  1401, `WrapReverse` 10) or not (`NoWrap` 2589, `NoWrapInfinite` 347), justify along the axis
  (`Stretch` 64 beside the CSS values), justify their lines across (`crossAxisJustification`) and
  align their items (`itemAlignment`); items have `growProportion` and `shrinkProportion`. The
  cross justification and the item alignment are equal in almost every container (centre with
  centre, end with end, start with start), as a preset of the editor would set them.
- Widgets have `pivot` and `anchor` (35 % of the 44953 widgets of the scenes outside the HUDs set
  them apart, 8 % of those that fill their parent), `orientation` and `orientationOffset` (z turns
  of 90, 180, -90 and 45 degrees; x and y turns of 4, 90 and 180), `scale`, `overflow` (`Visible`
  51721, `Clip` 1731, `Fade` 85, `ClipFade` 68), minimum and maximum sizes and rounded and
  chamfered corners (8054 fixed radii in widgets, 14210 radii and 2682 chamfers in style
  entries). Style entries set the same fields at rest: `SizeX` and `SizeY` (4275, 3422; values
  such as 1, 0.7 and 320, so they replace the number and keep the behaviour of the size),
  `AnchorX`/`Y`, `PivotX`/`Y`, `ScaleX`/`Y`, `OrientationX`/`Y`/`Z`, `PosX`, `PosY`, the offsets,
  `Padding*`, `MinSize*`, `MaxSize*` and `FlexColumnSpacing` and `FlexRowSpacing`.
- Units. `coordinateMethod` is `useRaw` for 3034 canvases, `auto` for 381, `aspectOverridesWidth`
  for 364 and `aspectOverridesHeight` for 31. In raw canvases that are no HUD, 1079 of 1381 have a
  median fixed size of 20 or more and only 122 one of 3 or less; 240 root widgets are fixed at
  1920 x 1080, the size of their canvas; fonts have a median size of 23. The HUD canvases are the
  exception: 114 root widgets are 1.2 x 0.9 and 76 are 0.9 x 0.9 fixed units, while their fonts
  are still pixels (median 20). A canvas widget lays its canvas out in its box (`sizingMethod`
  `Size`, 3908) or scales it (`Scale`, 101).

## Decisions

- **Units.** A raw canvas counts fixed values in pixels of its own units, no more in canvas
  heights. A canvas whose largest root widget of fixed size, by area, is at most 3 on both axes
  (a world-space canvas, such as a HUD) counts in world units instead; roots of other sizes do
  not count, since some HUD canvases have twenty roots of which one is small. That root is scaled
  to fit the frame,
  `min(frame width / root width, frame height / root height)` pixels per unit, the origin of its
  root widgets is the centre of the frame, and font sizes stay pixels. The canvas is laid out at
  the size of the frame divided by that scale and drawn under the scale, so a nested canvas keeps
  the pixel of its host. A raw canvas nested in a widget of another size therefore keeps its
  pixels, as the issue #535 review asked to decide, unless the canvas is a world-space canvas,
  which fits its root into the box of the widget, or the widget scales it. `Scale` lays a canvas widget's canvas out at the canvas size and fits it
  into the box, centred; `Size` lays it out in the box. `aspectOverridesWidth` takes the width
  of the box from its height by the aspect of the canvas, `aspectOverridesHeight` the height from
  its width, at the start of the box; the names are the only evidence for them, and for a screen
  of its own size they change nothing.
- **Templates.** The widgets of the `library` follow the scene in the widget list of a canvas; a
  scene widget with the parent `library[n]` is a child of that template. A template is not drawn
  where it stands. A clone draws its target as its only child, laid out in the box of the clone
  by the sizing of the target, and the children of the target with it. Its modifiers apply as
  style modifiers to the target, or to the widget their own `target` names. A list does the same
  once per row of the static array its `arrayVariable` names, with at most 2^10 rows; the rows do
  not get their values, which a binding fills in at run time, and the list says so. A list whose
  array is a run-time variable has no rows at rest and says that too. Clones and lists
  are structural widgets: they have a renderer, so they are no longer listed as "no renderer".
- **Bounds.** At most 2^14 instances of templates are made per screen (`Limits::instances`), a
  template that contains a clone of itself, directly or through other templates, is repeated no
  deeper and listed, and the widget, depth and shape limits of ADR-0089 count every instance. The
  style of an instance, which depends on the type and tags of its ancestors only, is resolved once
  per ancestor chain, the modifiers of the clones in force and widget, so a list of a thousand
  rows costs one cascade. Widgets of the library whose parents form a cycle, like those of the
  scene, are listed and not drawn. Numbers a style entry sets for sizes, positions and padding
  are held to ±10^6 and pivot and anchor to ±10^4; non-finite ones are ignored.
- **Flex containers** place their children in lines, as the data suggests. The items of a line
  keep their sizes, grow by `growProportion` (an axis justification of `Stretch` shares what is
  free equally among items without growth), shrink by `shrinkProportion` times their size when
  the line is too long (an item without an item policy and `NoWrapInfinite` containers do not
  shrink), and are spread by the axis justification. A container that wraps starts a new line
  when the next item does not fit; `WrapReverse` puts the lines in the reverse order and starts
  them at the end of the cross axis, so a `Start` cross justification packs them at the end (a
  guess from the name). The lines
  are as high as their highest item, are placed across by the cross justification (and grow to fill
  the container for `Stretch`), and an item sits in its line by the item alignment (`Stretch`
  fills the line). A container that does not wrap has one line. Minimum and maximum sizes bound
  the items before and after the sharing; the sharing is not repeated after a bound held an item.
- **Stack and grid pack.** A stack is a flex container that does not wrap or shrink, along its
  `direction` with its `spacing` between children. A grid pack is a wrapping flex container along
  its `direction` (`Horizontal` fills rows, `Vertical` columns) with its `gutter` between the
  cells and the lines. A spacer container pushes its children apart by a physics step that is not
  run; its children are an overlay and the container is listed.
- **Anchor and pivot.** A child that is not placed by a container sits so that its pivot (a share
  of its own box) is on its anchor (a share of the content box of its parent), moved by its
  position and margin; the children of a flex, stack or grid container sit in the slot of the
  container, and an item out of the flow at its anchor. The pivot is also the centre of the turn
  and the scale of any widget.
- **Turn and scale.** `orientation` and `orientationOffset` add up; the turn about x, y and z is
  projected orthographically on the screen, z turning clockwise for positive angles (the sign is
  unverified for angles that are not multiples of 180), so a turn of 180 about x flips a widget
  over and one of 90 about x leaves nothing to see. `scale` is applied before the turn. The
  transform applies to the widget, its children and its nested canvases. A zero or non-finite
  transform draws nothing or is replaced by the default and listed; `inheritsRotation` and
  `inheritsScale` of `false` are not undone and are listed.
- **Overflow.** `Clip` and `ClipFade` clip the children to the box of the widget under its
  transform, with a scene mask; masks are made only for shapes that leave the box, and at most
  24 per screen, after which clipping stops and is listed. `Fade` and the fade of `ClipFade` are
  not drawn and are listed.
- **Sizes of automatic widgets** come from the content: a text as large as its glyphs, a
  container as large as its children (the sum along the axis of a flex, stack or grid container
  with the spacing, the largest child across and in an overlay), over the children whose size
  does not depend on the parent. A widget without such a content takes the size of its parent,
  and says so. Measuring is bounded by the widget limit and each result is kept per widget,
  ancestor chain and parent size.
- **Corners.** Rounded and chamfered corners of the box are drawn for the background, and a
  border of one width and colour follows the outline as a ring between two contours; borders that
  differ are drawn straight and listed. A radius is held at half the shorter side. Gradient
  backgrounds are drawn in their plain colour and listed.

## Still approximated, and listed in the report

Textures and vector graphics (#537), progress meters, windows, edit boxes, dust and light
widgets, the values of list rows and everything that a binding sets at run time, states and
animations, the perspective of turns about x and y, the fade of overflow, gradient backgrounds,
borders that differ on rounded boxes, spacer containers, `inheritsRotation` and `inheritsScale`,
the position of nested canvases in the aspect modes, and the measuring of automatic texts, which
is done twice (#564).

## Considered Options

- Keeping a raw unit of one canvas height: 78 % of the raw canvases outside the HUDs count in
  tens of units, and the root widgets that equal the canvas size would be 1080 times too large.
- A single line that fills the container across, as CSS does for a flex container that does not
  wrap: the data has a cross justification beside the item alignment, and sets both, so they are
  taken as two settings.
- Building the whole tree of widgets before laying it out: the resolved style is kept per
  ancestor chain instead, which gives the same sizes from the content without a second tree.
