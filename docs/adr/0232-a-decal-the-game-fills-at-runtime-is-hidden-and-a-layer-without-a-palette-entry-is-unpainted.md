# A decal the game fills at runtime is hidden, and a layer without a palette entry is unpainted

Decided for issue #777. It refines ADR-0100 (the add-on's surface mapping) and follows ADR-0222 and
ADR-0225 (hidden materials are decided in Rust and the package never contains them). Evidence: the
LIVE Archive, every `.mtl` file below `Objects/Spaceships`, and the Blender packages of the
Hammerhead, Gladius and Cutlass Black.

## What the data says

- Every `MeshDecal` whose `TexSlot1` is the placeholder `$RenderToTexture` (also spelled
  `$RendertoTexture`) draws registration marks, name plates and serial numbers that the game fills at
  runtime: `RTT_Text_To_Decal`, `RTT_Render_To_Texture`, `Decal_Text`, `Decal_Serial_Number`,
  `Nameplate`. Their generation mask carries `USE_RTT_UV_FOR_DIFFUSE`. The Archive holds nothing to
  draw, so the export showed a white strip.
- Screens, HUD planes and cockpit glass (`DisplayScreen`, `UIPlane`, `GlassPBR`) name the same
  placeholder in `TexSlot9`, never in `TexSlot1`. They are the surface of a display and stay drawn.
- A layer of a `HardSurface` material that has a tint but no palette entry, such as the wear layer
  of `metal_steel` on the Gladius, `Paint_Secondary` (tint 0.013) of the Cutlass Black, `tech_greeble`
  and `panels_metal`, counted as painted. The add-on then skipped the layer's specular map, took the
  default specular colour (white) as the colour and left the diffuse mix unconnected.
- `engine_a` and `engine_b` of the Gladius are `Illum` materials with a specular map and no layers.
  They already took the specular-map path; their look is the studio light, not this cause.
- `Glass_NoRefract` of the Cutlass Black is `GlassPBR` with `PER_VERTEX_TINT`, a white diffuse
  colour and no palette. Its cause differs and is not part of this decision.

## Decision

- **Hidden by the model.** `Material::is_render_to_texture_decal` is a `MeshDecal` whose `TexSlot1`
  is `$RenderToTexture`, ignoring case. `Material::is_colourless_decal` includes it, so `hidden()`,
  the GLB writer, the Blender package, the viewer and the armour path all see the same rule. The
  package leaves the material out; the add-on has no copy of the rule.
- **Painted means a palette entry.** In the add-on a layer is painted only when a palette entry
  colours it. A tint alone multiplies the diffuse colour and the layer takes the specular-map path:
  metallic from the map in the 0.15 to 0.5 range, colour mixed from diffuse times tint and the map.

## Consequences

- Layers with a tint and no palette entry draw as grey metal with detail instead of white.
- A palette that names no entry for a layer leaves the layer unpainted, as before for untinted ones.
- Packages written before this change still contain the decals; writing them anew removes them.
