# The add-on builds material node trees from the package descriptions by a pure mapping

Decided for issue #160 (M13.4); it reads the descriptions of ADR-0092 in the add-on of ADR-0093.

## Decision

**Description in, graph description out, nodes last.** `material_graph.build_graph` maps a
material description, the descriptions of its layers, the mesh's palette and the alpha meaning of
the textures to a `GraphSpec`: plain data with node types, properties, input defaults and links.
`materials.py` is the only place that creates Blender nodes from it. The mapping needs no Blender,
so pytest covers it; the headless smoke test checks the nodes that come out. `package.py` reads
`materials`, `palettes`, `MeshMaterial.material` and `Mesh.palette_id` (`material_model.py`); a
malformed description or palette is dropped with one warning, never an error, and a 1.0 package
reads as having none. `SUPPORTED_MINOR` is 1.

**Pairing.** Blender's glTF importer turns the `extras` `p4kx_index` of a material into a custom
property of the same name. A created material is paired with the mesh's manifest entry of that
index; materials without the property (a 1.0 package) are paired by name, as before. A paired
entry with a description gets the node tree of the description; one without, the basic setup of
ADR-0093 (its Specular IOR Level placeholder is now a grey value of the specular map).

**Shader families.** The shader name, case folded, picks the family:

| Family | Shaders | Surface |
| --- | --- | --- |
| surface | `HardSurface`, `Illum`, `Layer`, `LayerBlend`, `LayerBlend_V2`, `Simple` | Principled BSDF; layers when the description has them |
| glass | `GlassPBR`, `Glass` | Principled with Transmission Weight 1, IOR 1.45, blended; the `glass` palette entry is the colour |
| decal | `MeshDecal` | Principled, blended; a `$TintPaletteDecal` slot takes the palette's primary tint; `DecalDiffuseOpacity` scales the opacity |
| hologram | `Hologram`, `HologramCIG` | Principled, the colour as emission (strength at least 1), blended |
| nodraw | `NoDraw`, `NoDraw_Proxy`, `Proxy`, or the `no_draw` flag | Transparent BSDF |
| unknown | anything else | the surface setup without layers, and a warning in the import report |

Every material also keeps `p4kx_material`, `p4kx_shader`, `p4kx_family` and its shader parameters
(`p4kx_param_<name>`, at most 64) as custom properties, so a value the mapping does not use is
not lost for a script or a later add-on version.

**Texture roles.** By the description's `usage`: `diffuse` is the base colour (sRGB), `normal`
a Normal Map node (tangent space; the package's normal maps are already OpenGL, so nothing is
inverted), `specular` a grey value into Specular IOR Level, `gloss` the smoothness, `occlusion`
multiplied into the colour, `detail` overlaid at 0.5, `mask` and the opacity alpha of the
diffuse map the alpha, `blend` and `wear` the layer and wear factors. `displacement` and `other`
are not used. All but the diffuse map are linear (the package's `colour_space`). A `TexMod` of a
slot becomes a Mapping node (tiling as scale, offset as location). Smoothness is the alpha of a
normal map whose texture `alpha` is `smoothness` (the image's alpha mode is then channel packed
so the colour is not premultiplied) or a `gloss` map; roughness is `1 - smoothness`. Without a
map it is `1 - shininess / 255`, or `1 - glossiness / 255` of the palette entry the material
takes. Metallic is 1 when the `surface_type` contains `metal`, else 0. `emissive` times `glow`
becomes Emission when `glow` is above 0.

**Palette tints.** The mesh's `palette_id` selects the palette; the material's `palette` the entry.
The entry's `tint` replaces the description's `diffuse` as the factor of the diffuse map (or is
the base colour without one), its `specular` is the Specular Tint and its `glossiness` the
smoothness. A mesh without a palette keeps the description's own colours.

**Layers.** A layer is drawn from the description its `material` names (its diffuse and normal
map, its own `diffuse` and shininess). Its colour factor is the entry named by its `palette`
times its `tint_colour`, else `tint_colour`, else its `diffuse`; `uv_tiling` scales its UVs and
`gloss_mult` its smoothness. If the base material has no diffuse map, the first layer is the
foundation of the surface; otherwise the base material is. Each further layer is blended over
the result by one channel of the base material's `blend` map (else `mask` map, else the mesh's
vertex colours): red for the first, green, blue and alpha. More than four blended layers are
cut off with a warning. The blend is the shared group **P4KX Layer Blend** (colour, roughness
and normal, the normal mixed and renormalised), so a normal is created only when a layer has
one. Nothing in the files fixes which channel belongs to which layer; this is the convention
used until a game-side reference says otherwise.

**Wear.** A layer with a `wear_tint` is multiplied by it where the wear factor is high, through
the shared group **P4KX Wear**. The factor is the base material's `wear` map, times the
`WearBlendBase` parameter when there is one, or the parameter alone.

**Alpha.** `alpha_mode` `opaque` links no alpha. `mask` links the mask map (else the diffuse
alpha) through a greater-than of `alpha_test`. `blend` links the same source times `opacity`, or
the `opacity` itself; glass, decals and holograms are always blended. The material's render method
is `BLENDED` for blend and `DITHERED` otherwise, and back-face culling follows `two_sided`.

**Shared groups.** The two groups are described as data (`graph_groups.py`) and created in the
file on first use by name; every material instances the same group, so a hull with a hundred
materials has one of each.

## Consequences

New shader parameters keep arriving as custom properties and need no change to the mapping until
one is worth using; a new family is a name in `material_graph.py`, its graph, and a row above.
The layer channel convention, the metallic rule and the roughness mapping are the first things to
revisit when a real ship is compared with the game. Decal stencils, parallax and damage are not
wired yet. The colour derivation of the viewer and the GLB (ADR-0097) may refine the layer
blend; the pure mapping is the place to follow it.
