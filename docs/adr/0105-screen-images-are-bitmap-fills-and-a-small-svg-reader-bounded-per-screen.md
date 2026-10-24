# Screen images are bitmap fills and a small SVG reader, bounded per screen

Decided for issue #537, on top of ADR-0089 (screens), ADR-0101 (texts) and ADR-0020 (the `Scene`).

## What the Records hold, on LIVE with `p4kx dcb fields` and `dcb export`

- `WidgetImage` (7472 widgets in the `BuildingBlocks` Records) names `imagePath`, a `.tif` that the
  Archive holds as `.dds`, with `imageScalingMethod` `Fill` (7439), `Contain` (121) or `Cover` (19),
  `flipH`, `flipV`, `containPositionX/Y` and a `color`. 322 of them name no path: they are boxes
  that only a background colour fills. 44 name an `svgFill.svgPath` instead.
- `WidgetCustomShape` (3923) names `svgPath` and the vector fields in the widget itself;
  `WidgetImage`, `WidgetIcon` and the others keep the same fields in `svgFill`:
  `scalingBehavior` (`Contain`, `StretchXY`, `StretchX`, `StretchY`, `Center`), `flipHorizontal`,
  `flipVertical`, `enableColorOverlay`, `containPositionX/Y`, `renderShape`, `enableNineSliceRect`
  with `nineSliceRect` (the cut lines `left`, `right`, `top`, `bottom` as shares of the graphic) and
  `nineSliceScale`. `WidgetIcon` names its graphic as `iconProperties.customIcon` (177), or a preset
  of an enumeration that has no graphic of its own in the Records (112); the custom shapes that
  show the same icons name files such as `.../Widgets/IconWidget/arrow_carat_left.svg` for the
  preset `ArrowCaratLeft`.
- `WidgetForegroundOverlay` and `WidgetBodyBackground` have `urlOptional` and
  `textureProperties`, but the Records leave the URL empty everywhere: the texture is chosen by
  the game at run time.
- The cascade sets the same fields as modifiers: `ImagePath`, `SvgPath` (`FieldModifierString`),
  `EnableColorOverlay`, `SvgFlipHorizontal`, `ImageFlipHorizontal`, `EnableNineSliceRect`, and the
  colour of an image or a shape as `FillColor` (16675 uses).
- The SVG files are Adobe Illustrator exports (`<g>`, `<path>`, `<rect>`, `<circle>`, `<use>`,
  `transform`, presentation attributes, `display="none"`, sometimes a `DOCTYPE` and a `<style>`).
  An element can name a palette slot in its `id`, as `colorstyle:Bright_opacity:40`.

## Decisions

- Textures are decoded by the front-end and handed to the composer as `p4kx_texture::RgbaImage`
  through `ImageSources { texture, vector }`, as the fonts of ADR-0101 are, so the service crate
  never opens the Archive (ADR-0003). `p4kx-extraction` gains `ArchiveImages`, which finds an
  Entry by the path of a Record (with and without `Data/`, any case), decodes the smallest mip
  whose longer side still covers the screen (and stays below 2^25 pixels) through
  `p4kx-texture`, merging the alpha tail, and reads a vector graphic up to the limit it is given.
  The CLI and the real-Archive tests use it.
- A texture is drawn as a bitmap fill of a rectangle of the `Scene`: `Fill` stretches the image
  over the widget, `Contain` keeps its aspect ratio inside, `Cover` keeps it and crops it to the
  widget; `containPositionX/Y` place the free space; flips mirror the fill; the colour of the
  widget (cascade `FillColor`, else `color`) multiplies the texture through the colour transform
  of the placed item, with the alpha of the widget. Each texture is decoded once per screen and
  kept by its path; the scene's bitmaps travel with the `Composed`, so the PNG rasteriser and the
  SVG writer both draw them (SVG output embeds them as PNG).
- An SVG file is read with `roxmltree` into `Scene` layers (fills and strokes, paths with
  quadratic pieces), once per path and screen, and placed as one shape with one item per
  widget: the transform fits the `viewBox` to the widget (`Contain`; `StretchXY`; `StretchX` and
  `StretchY` fill that axis and fit the other at most to the widget; `Center` at natural size),
  `containPosition` places it, flips mirror it, and `enableColorOverlay` replaces every colour
  with the colour of the widget through the item's colour transform. Without an overlay colour an
  element whose `id` names a palette slot takes that colour from the palette of the canvas, with
  its opacity. A nine-slice applies only to the stretching behaviours (the Records enable it, with
  its default cut lines, on icons that are fitted with `Contain`, where it would only distort
  them). It builds a shape of its own, cutting each axis whose cut lines lie in order inside the
  graphic (caps at `nineSliceScale`, the middle stretched) and fitting the other axis as the
  scaling behaviour says; control points move with their points, which is exact inside a slice
  and an approximation across a cut line.
- The SVG reader is bounded because the files are untrusted: at most 2 MiB, 2^16 XML nodes,
  nesting of 32 groups (deeper elements are left out and listed), 2^18 path commands per file
  and 2^22 per screen, 1024 style rules, 4096 ids, 1024 graphics and 1024 textures per screen;
  every number must be finite and within 10^7; the elements visited, `use` expansions included,
  are capped at 2^20, and the copies a nine-slice or a palette slot makes count against the path
  budget of the screen; an entity declaration in a `DOCTYPE` refuses the
  file (a `DOCTYPE` without an internal subset is skipped), `use` follows only `#id` links inside
  the file and counts the commands it expands to, and nothing is fetched. Textures are bounded
  by `Limits::texture_pixels` (2^26 pixels per screen, 2^25 for one) and their decoder limits;
  the pixel and path budgets of a screen are in `Limits`.
- A texture or graphic that is missing, cannot be read or is beyond a limit lists the widget
  (`texture missing or unreadable`, `vector graphic missing or unreadable`, with the path and the
  cause) and the widget is skipped; an icon preset is the file `<snake_case>.svg` in the folder
  `UI/Textures/Vector/General/ModularKit/Widgets/IconWidget` (a convention seen in the paths of
  the custom shapes, listed as missing when the Archive lacks the file); a graphic a widget names
  only at run time (empty `svgPath`), and the overlays and backgrounds without a URL, are listed
  with their own reasons. An image widget without a path is a plain box and says nothing. Placements whose
  alpha is 0 at rest are not drawn. The report counts `images_drawn` and `images_left_out`.
  Without `ImageSources`, image, custom shape and icon widgets are listed as without a renderer,
  as texts are without fonts.
- Listed and not drawn: gradients (drawn flat in their first colour, listed), clip paths, masks,
  filters, `<image>` and `<text>` inside an SVG, the nonzero fill rule (the `Scene` fills
  even-odd), rotation, pivot and anchor (#538), `segmentedFill`, `strokeExtent`,
  `playhead` and the animation of overlays.

## Considered Options

- Decoding textures inside `p4kx-screens` with a path-reading trait of its own: it would repeat
  the sibling-part logic of `p4kx-extraction`; a decoded image is all the composer needs.
- Using `usvg` or `resvg`: a large dependency tree for an SVG subset of a few elements, and the
  `Scene` needs paths, not pixels, to keep the SVG output.
- Stroking and filling an SVG per widget instead of sharing its shape: every placement would
  hold its own copy of the paths; one shape with a transform and a colour transform is what the
  `Scene` already offers.
