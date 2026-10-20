# The SVG export of a UI frame charges the budgets of the raster

`p4kx ui render --svg` writes the first frame of a UI movie as an SVG document so that it scales
without loss. It is a second backend of the backend-free `Scene` of ADR-0020 (`Scene::write_svg`
next to `Scene::rasterize`), so shapes, sprites, colour transforms, masks and what is skipped come
from the same composition and an SVG and a PNG of one movie list the same elements.

## Budgets

A movie is untrusted, and an SVG that a viewer rasterises later is no safer than a PNG drawn here.
Writing therefore runs a planning pass over the items first that charges what `rasterize` charges:
the covered area of every fill, stroke and mask build to the budget of `MAX_COVERAGE_FACTOR` times
the canvas, with the same bounds, and the pixels of the bitmap copies (plain, or transformed by a
colour transform that is not only an opacity) to the bitmap limit. The frame ends at the same item
and the report lists the same `Skipped` entries; the shape conversions are charged by composing the
scene. Only the memory limit of masks does not apply, since no mask is built here. Nothing is
written that the plan left out: unused shapes and bitmaps are not emitted. The bitmaps arrive
decoded under the limits of the raster (`MAX_UI_RENDER_BITMAP_PIXELS`).

## Document

- Output is streamed through the writer given, a `BufWriter` of a temporary file next to the target
  that replaces it whole or not at all; bitmaps are PNG-encoded straight into a base64 stream, one
  at a time, so a bitmap costs no buffer of its encoded size.
- Every shape is a `<g id="sN">` in `<defs>` for each colour transform it is placed with, and every
  placement a `<use>` with its transform, inside one group that maps twips to the canvas. Masks are
  `<mask>` elements of white shapes, nested through the `mask` attribute of their parent, and the
  items they clip sit in a `<g mask>`.
- Gradients are `linearGradient` and `radialGradient` in the gradient square with the transform of
  the fill and the spread method; bitmaps are one `<image>` each with a `data:image/png;base64,`
  URI, never a path or a URL, so no reference can leave the document. Repeating bitmap fills are
  patterns.
- Numbers are written with fixed precision, trailing zeros removed, so the same scene gives the
  same bytes; no hash order or time enters.
- The only text from the movie or the Archive is the movie name in `<title>`; it is escaped, and
  characters XML 1.0 cannot hold become U+FFFD. Every other attribute is a number, a colour or an
  index the writer made.

## Where it differs from the raster

- A bitmap fill that does not repeat is drawn once inside a clip of its path; the raster continues
  the edge pixels of the bitmap beyond it, which an SVG cannot say. A stroke painted with such a
  bitmap tiles.
- A hairline is a stroke of one pixel with `vector-effect="non-scaling-stroke"`.
- Gradient interpolation, the single cap of a stroke and the unsupported filters and blend modes
  are as ADR-0020 describes for the raster.

## Considered Options

- Writing every placement inline without `<use>`: no definitions to plan, but every shape is
  repeated per placement and the file grows with the placements instead of the shapes.
- Rasterising through `resvg` to compare pixels in tests: a large dependency for a check the
  structure tests and a real-Archive well-formedness run already cover.
- An `svg` crate or `quick-xml` for writing: the document is a few fixed element kinds, numbers
  and one escaped string; the escaping and number code is small and tested here.
