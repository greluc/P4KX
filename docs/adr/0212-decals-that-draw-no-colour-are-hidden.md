# Decals that draw no colour are hidden

Decided for issue #734. It amends ADR-0100 (the add-on's surface mapping) and the hidden-material
rule of the GLB writer. Evidence: the LIVE Archive, imported into Blender 5.2 and compared against
in-game screenshots: the Cutlass Black and the ANVL Arrow.

## What the data says

- The engine pods of the Cutlass Black were covered by white panels. Ray casts in Blender hit
  `decal_placeholder`, a `MeshDecal` without any texture slot and with opacity 1. A livery fills such
  a placeholder in; without one the engine draws nothing.
- The Arrow showed white lines along every panel. Of 300 sampled bright pixels, 99 hit `poms`, an
  `Illum` material with `DECAL` in its generation mask and `DecalDiffuseOpacity` 0. In the game it
  only adds relief to the surface below; its light diffuse sheet was drawn opaquely, with an alpha
  mask of 0.43.

## Decision

- A material is a colourless decal when it is a `MeshDecal` without any texture slot, or when `DECAL`
  is one of the flags of its generation mask and `DecalDiffuseOpacity` is 0 or less.
- `Material::hidden` in `p4kx-model` includes colourless decals, so the GLB writer and the viewer skip
  them. The add-on builds the no-draw graph (a transparent shader) for them.
- A `DECAL` material with a positive or missing `DecalDiffuseOpacity`, and a `MeshDecal` with a
  texture slot, are drawn as before.

## Consequences

- The relief of a colourless decal (normal and parallax occlusion map) is not drawn either. A later
  change could lay its normal map over the surface below.
