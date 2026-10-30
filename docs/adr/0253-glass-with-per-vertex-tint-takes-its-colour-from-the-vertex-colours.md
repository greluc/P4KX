# Glass with PER_VERTEX_TINT takes its colour from the vertex colours

Decided for issue #797. Amends ADR-0100 (the glass row).

## Context

`Glass_NoRefract` of the Cutlass Black is `GlassPBR` with the generation mask `PER_VERTEX_TINT`,
a white diffuse colour and no palette entry. The add-on gave it a white Transmission colour, the
same as every other glass without a palette entry, so the lamp lenses it covers drew as bright,
untinted glass.

The vertex colours of its triangles are not masks. On the lamp lenses the RGB values are neutral
greys that fall from 0.5 at the lens to 0 at the rim, with alpha 1: a tint. The vertex colours of
`glass_ext` on the same mesh, which has no `PER_VERTEX_TINT`, are the blue and white mask data of
the hull and stay unused for glass.

## Decision

- A `GlassPBR` or `Glass` material whose generation mask names `PER_VERTEX_TINT` takes its
  Transmission colour (Base Color) from the vertex colours: the colour attribute, times the
  description's colour (the `glass` palette entry when the material names it, else the diffuse
  colour) unless that colour is white.
- The vertex alpha is not used; the alpha follows the glass rule of ADR-0100.
- The rule is structural: the shader family and the generation mask, never a material or ship name.
- The node reads the attribute `Color`, which Blender's glTF import writes for `COLOR_0`. Glass on
  a mesh without that attribute drops the tint and keeps its plain colour, because the node would
  read black.
- The GLB has no rule of its own. glTF multiplies `COLOR_0` into the base colour, so `PER_VERTEX_TINT`
  glass is tinted there. The limit: the writer writes `COLOR_0` for every primitive of a mesh that has
  vertex colours, so glass without the flag, such as `glass_ext` next to mask colours, is also tinted
  and faded by mask data in glTF viewers, while the add-on ignores it. The add-on needs `COLOR_0` for
  layered surfaces, so the fix is writing it per primitive (#877).

## Consequences

- Tinted glass is no longer white in Blender; it is as dark as its vertex colours say.
- Glass without `PER_VERTEX_TINT` is unchanged.
- The game's own shader is not available; the rule rests on the vertex data. A glass whose vertex
  colours are masks but which carries the flag would draw dark in the masked parts.
