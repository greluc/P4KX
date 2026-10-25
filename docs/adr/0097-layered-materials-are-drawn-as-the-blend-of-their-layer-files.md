# Layered materials are drawn as the blend of their layer files

Decided for issue #547. It builds on ADR-0080 (palettes; the layer textures of `MatLayers` are not
drawn), ADR-0085 (the viewer uses the GLB writer's colour function) and the `.mtl` reader of #158
(layers, `PublicParams`).

## What the data says

Measured on the live Archive (1,426 `.mtl` files of ships): 17,647 materials have `MatLayers`;
most have a `Primary` layer, 12,000 also a `Wear` layer; a few `LayerBlend_V2` materials have up
to six `BaseLayerN`. The material's own `Diffuse` and `Specular` are placeholders (0, or 1) and say
nothing about the colour. Each layer names a `.mtl` of its own (`Path`, and `Submtl` for a
sub-material by name) whose `Diffuse` and `Specular` are the layer's surface: `Diffuse` is often 1
because the colour lives in the layer's diffuse texture, metals have a dark `Diffuse` and a bright
`Specular`. The layer's `TintColor` (or the palette entry of its `PaletteTint`) tints it,
`GlossMult` scales its gloss, and `WearBlendBase` of the material's `PublicParams` says how much of
the surface the wear layers take. Blend masks are textures and vertex colours; they cannot be
read without textures.

## Decision

**Reading.** `MaterialStatus::load` reads the file of every layer through the companions and keeps
its `Diffuse` and `Specular` in `MaterialLayer::surface`. At most 256 distinct layer
files are fetched per library; a path that is no plain path below `Data`, a missing or unreadable
file and an absent `Submtl` leave the layer without a surface and are listed in
`MaterialLibrary::unresolved_layers` (reported as a `MaterialLibrary` issue of the Assembly and in
`model glb`). The #158 limits of the reader apply to every layer file. `TintPalette::apply` gives
each layer with a `PaletteTint` its palette entry (`MaterialLayer::palette_entry`).

**One function.** `p4kx_gltf::shading(material)` returns the base colour, whether the material is
a metal, the roughness and the blended diffuse and specular colour. The GLB writer (all models,
Assemblies and packages), the texture planning and the viewer, through `base_colour_factor`, use
it; a material without layers, or with none of its layer files read, is shaded exactly as before.

**Blend.** Each layer has the diffuse colour `layer Diffuse x tint` (the tint is the palette entry's
colour for a palette layer), the specular colour of its file (the palette entry's for a palette
layer) and the smoothness `0.5 x GlossMult` (the entry's glossiness times `GlossMult` for a palette
layer, as in ADR-0080; the glossiness of the layer's own file is not used). The tint, the diffuse
and the specular colour are clamped to 0 to 1 before they are multiplied, so hostile finite
values cannot overflow. The layer's base colour moves from its tinted diffuse towards its specular
colour by `metal_share(specular)`, the ramp of the texture baking (`metal_mix`), so colours and
textures modes agree: a dielectric keeps its diffuse colour, a metal takes its specular colour.
The layers are weighted by position and the wear share `w = WearBlendBase`: the first layer
`1 - w`, the others `w` between them, whatever their names; a layer without a surface has no
weight and the rest are renormalised. When the first layer is missing and the others have no
weight, the material keeps its own colours. The metalness is the weighted share of the layers'
`metal_share`, a metal from one half.

## Measured, and what the owner decided

Materials with a linear luminance under 0.05, live Archive, `real_layered_materials`:

| Ship | Materials | Before | Additive (diffuse + specular) | Lerp (decided) |
|---|---|---|---|---|
| Polaris | 219 | 39 | 10 | 49 |
| Gladius | 211 | 26 | 17 | 48 |
| Constellation | 293 | 22 | 20 | 69 |

Adding the specular colour to the diffuse colour brightens dielectrics and whitens metals and
disagrees with `metal_mix` of the textures mode, so the lerp blend is kept even though the share
of dark materials rises: before, many layered materials were white or black placeholders, and many
layers are dark by their own data (black paint, anodised metal, gunmetal, rubber, dark palette
entries). The criterion that the share drops clearly does not apply; brightness and detail come
with the layer textures (#551).

## Consequences

Layered hulls are flat colours of their layers; their detail (the layer diffuse and normal
textures, `UVTiling`, the blend masks) stays undrawn and is the follow-up of #547. Layers whose
diffuse colour is 1 because a texture carries it are drawn white or in their tint, not in the
texture's colour. Dark paints stay dark where the palette or tint says so.

## Amendment (issue #551)

The layers' diffuse and normal maps, their tiling and the blend map of the material are drawn in
the textures mode by ADR-0147; the flat colours of this record remain the colours mode and the
factors, and the shares by position remain the shares without a blend map.
