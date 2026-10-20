# Material libraries and plain-colour materials

What the reader of `p4kx-model` assumes about `.mtl` files, and how `p4kx-gltf` turns them into
plain colours. Surveyed on 2026-10-20 over all 26,083 `.mtl` Entries of LIVE. The tests are
`crates/p4kx-extraction/tests/real_archive_materials.rs` (`#[ignore]`d, run with `P4KX_DATA_P4K`;
both pass on LIVE and PTU) and the API tests of `p4kx-model` and `p4kx-gltf`.

## The file

A `.mtl` Entry is CryXml (the converted XML is 2 KB to 133 KB; the reader caps a file at
16 MiB). The root is `<Material>`. 19,126 files have a `<SubMaterials>` list (up to 116
sub-materials; the engine's limit is 128, the reader's 1,024), and 6,957 are one material that
serves every material ID. The material ID of a submesh is the position in the list. No file nests
a `SubMaterials` list in a sub-material or refers to another `.mtl` file by `File`, so there is no
sub-material cycle to follow; nested lists are not read, which also bounds the recursion.

Attributes per material: `Name`, `MtlFlags`, `Shader`, `StringGenMask`, `SurfaceType`, `Diffuse`,
`Specular`, `Emissive` (three comma-separated floats), `Opacity`, `Shininess` (0 to 255), `Glow`,
`AlphaTest`, and in the root `DefaultPalette`. A material may also have `Textures/Texture`
(`Map="TexSlot<n>"`, `File`), `PublicParams` and `MatLayers`; the reader keeps the texture slots
(placeholders such as `$TintPaletteDecal` and `$RenderToTexture` included, empty files left out)
and leaves the rest for textured materials (#105) and tints (#127).

- `MtlFlags` is a signed 32-bit number in the file (negative values occur); the reader keeps its
  32 bits. Bit `0x400` is the engine's no-draw flag: it is set on 24,626 materials, exactly the
  24,626 whose shader is `NoDraw` (one material each way differs). Bit `0x2` (two-sided) is set
  on 18,854 materials, mostly `HardSurface` and `Illum`.
- The colours are linear: `0.0097212186` is sRGB 0.1 linearised. glTF's factors are linear too,
  so they are written as they are.
- `Emissive` of `1,1,1` without `Glow` is common on `Illum` (8,243 of 13,625 materials with a
  non-zero emissive colour have no `Glow`), so the emissive colour counts only times `Glow`.
- Missing attributes read as white diffuse, a dim dielectric specular (0.04), no emission,
  opacity 1 and shininess 10. Malformed or non-finite numbers read like missing ones.

## Finding the library

The `MtlName` chunk names the library. The name is

- a path below `Data` without extension (`Objects/Spaceships/Ships/AEGS/Gladius/AEGS_Gladius`),
  the common case, resolved ignoring case and the kind of separator;
- a bare name (`f_facial_hair_021`), which the engine takes relative to the model: it is looked
  for as `<name>.mtl` beside the model first, then below `Data`;
- rarely with a `Data/` prefix and `.mtl` extension (`Data/Objects/.../m_qrt_combat_..._01.mtl`),
  which are stripped.

Of 150 sampled `.skin` files per Archive, 104 find their library; character skins often name a
library that lives in another folder and is chosen by the item's Entity, not by the skin (for
example `m_cds_legacy_medium_helmet_01_01_hd_sec`). Of the static models with a library name,
66 of 69 (LIVE) and 65 of 67 (PTU) find it; the rest name libraries the Archive does not hold
(`Objects_default_grid_blue`, `kbar_s1-3_bas_1`). A library that is missing is a status of the
model, not an error: the model reads and exports with the default material.

In an Assembly the library comes from the Entity: the Material of the Record's
`SGeometryResourceParams` geometry replaces the library the model names, and the model's own
library applies only when the Record names none (ADR-0040).

A name is checked before any file is asked for: an empty, absolute or drive-qualified path, or one
with an empty, `.` or `..` component, is `Unreadable` and never reaches the resolver. The Archive
resolver only looks the path up in its index; the disk resolver reads `<last component>.mtl`
beside the model.

## Plain colours in glTF

| glTF | from |
| --- | --- |
| base colour | `Diffuse`; for metals `Specular` |
| alpha | `Opacity`; 1 with `AlphaTest` (masked); at most 0.3 for `GlassPBR` |
| alpha mode | `MASK` with cutoff `AlphaTest` when above 0; `BLEND` for `GlassPBR`, the decal shaders `MeshDecal` and `Decal`, and `Opacity` below 1; else `OPAQUE` |
| double sided | `MtlFlags` two-sided, and the decal shaders |
| metallic | 1 when `Specular` is bright (luminance at least 0.5) and the `SurfaceType` starts with `metal` or `Diffuse` is dark (luminance under 0.2); else 0 |
| roughness | 1 − `Shininess` / 255; 0.5 for `HardSurface`, `LayerBlend_V2` and `Layer`, whose gloss comes from textures and layers |
| emissive | `Emissive` times `Glow`, divided by its brightest channel when that exceeds 1 |

These are heuristics for a file without textures, after the findings of the StarBreaker review
(#104). A material that draws nothing (the no-draw flag, the `NoDraw` shader or opacity 0) gets no
glTF material and its submeshes are not written; the summary counts them as hidden. A material ID
beyond the sub-materials, or any ID when the library is missing, gets one shared default material
(light grey, matt); the engine draws such IDs with `material_default.mtl`, which the reader does
not load.

`HardSurface` and `LayerBlend_V2` (23,770 and 46,806 materials) take their colour from `MatLayers`
and the tint palette, so their plain colour is the white `Diffuse`; tints are #127.

## Not examined

The normal maps (`TexSlot2`, `_ddna`, with the gloss in alpha) are not read here, so whether the
engine's tangent frame needs a green-channel flip in glTF stays open for textured materials
(#105).
