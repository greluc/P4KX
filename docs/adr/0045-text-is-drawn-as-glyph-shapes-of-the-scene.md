# Text of UI movies is composed into glyph shapes of the scene

Decided for issue #169. Static texts (`DefineText`, `DefineText2`) and text fields
(`DefineEditText`) are laid out while the frame is composed (ADR-0020), and every glyph becomes an
ordinary shape of the `Scene`: the outline of the glyph, filled with the colour of the text, placed
with a transform that scales the em square of the font to the text height. The raster (ADR-0020)
and the SVG writer (ADR-0030) therefore draw the same outlines, charge the same budgets and skip the
same elements, and SVG text is written as paths, never as `<text>` elements with a font name. The
only text written into an SVG stays the escaped title.

## Fonts

- Glyph outlines come from the fonts of the movie (`DefineFont`, `DefineFont2`, `DefineFont3`; the
  em square is 1024 units, 20480 for `DefineFont3`). Compacted GFx fonts, `DefineFont4` and fonts
  whose glyph shapes the exporter stripped have no outlines. A font of the movie is found by ID, and a
  text field by the class name or the `face` of its markup, ignoring ASCII case, in the fonts of
  the movie and then in `TextOptions::fonts`, the fonts a caller reads from other movies. Those are
  searched by name only: an ID means something only inside its own movie.
- No system font is ever loaded. A character whose font or glyph is missing is drawn as an empty
  box of the text colour, a fixed shape, and listed: `missing_font`, `missing_glyph` (the code, at
  most eight per text) and `missing_glyph_index` (static text). These reasons count as drawn
  without the effect, like filters, so the exit code of `ui render` is 1 and the box is visible.
- Fonts that a movie imports from another movie (`ImportAssets`) are resolved by the caller that
  has the Archive and passed in `TextOptions::fonts` (ADR-0048).

## Layout

- A text field follows the Flash text model: a gutter of two pixels, margins, indent and leading
  of `DefineEditText`, the ascent and descent of the font (80 % and 20 % of the height without
  layout data), the horizontal advance of the glyph plus the letter spacing, kerning only when the
  markup asks for it, left, right and centre alignment (justified is left), and wrapping after the
  last space that fits when the field wraps. The field is not clipped to its bounds, and fields that
  resize to their text keep their bounds.
- HTML text is the Flash subset: `p` (align), `font` (face, size in pixels, `#RRGGBB` colour,
  letterSpacing, kerning), `br`; other tags only group, entities are decoded, and a nesting deeper
  than 32 is ignored. A password field draws `*`.
- A text that is `@key` or `@key,P=a,P=b` is a localisation key, resolved by `TextOptions::localise`
  (`p4kx ui render --lang`, with the `global.ini` of that language of the Archive and its line
  breaks decoded); the parameters fill the printf-style placeholders `%ls`, `%s`, `%S`, `%d` and
  `%i` in order. The placeholder syntax is a best effort from the strings of the game, not a
  specification. A key the texts do not hold is drawn as it is and listed as `unlocalised`.

## Limits

Movie data is untrusted, so text is bounded like the rest of the scene.

- Reading: the glyph entries of all static texts of a movie are at most `MAX_TEXT_GLYPHS` (2^20),
  charged before they are allocated; a tag beyond it is a failed tag.
- Composing: a text field shows at most `MAX_TEXT_CHARS` (4096) characters, the rest is listed as
  `text_truncated`; every character parsed and every character placed is charged to
  `MAX_SCENE_GLYPHS` (2^17), and the frame ends at the first text beyond it (`too_many_glyphs`).
  A text field is parsed once per ID, so a text placed many times costs its parse once. Glyph
  conversions are charged to `MAX_SCENE_RECORDS` once per glyph and colour, glyph layers and path
  commands to `MAX_SCENE_PATHS` and `MAX_SCENE_SEGMENTS` per placed glyph, and the covered area
  to the fill budget of the raster and the SVG, all as shapes are.
- Markup: tags are looked for within 512 bytes and entities within 12, at most 256 distinct styles
  and 16 attributes a tag are kept, so no input makes the parser quadratic.

## Considered Options

- Writing SVG `<text>` elements with the font name: scales and selects, but depends on fonts the
  viewer has, which the game's own do not, and puts text of the movie into the document.
- Embedding the fonts as SVG fonts or WOFF: large, and the outlines are the same paths.
- Loading a system font as the fallback: differs from machine to machine and would make renders not
  reproducible; an empty box is visible, listed and stable.
- Rasterising text in the rasteriser only: SVG and PNG would no longer list the same elements.
