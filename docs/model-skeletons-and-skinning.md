# Skeletons and bone weights

What the reader of `p4kx-model` assumes about the bones of a model and the weights of its
vertices, and what the real Archives say about it. The tests are
`crates/p4kx-extraction/tests/real_archive/skeletons/` (`#[ignore]`d, run with `P4KX_DATA_P4K`;
no entry involved is encrypted, so they need no Archive key; they pass on LIVE). Surveyed on
2026-10-20.

## Where the bones are

| Chunk | ID | Version | Files of LIVE |
|---|---|---|---|
| `CompiledBones2` | `0xC2011111` | `0x900` | every `.skin` (20,131) and every `.chr` (566) |
| `CompiledBones` | `0xC201973C` | `0x901` | 9,237 `.cga` and 212 `.cgf` |

A `.chr` holds a skeleton and no mesh. A `.skin` holds its skeleton and a `MeshInfo` chunk, and its
`.skinm` geometry companion the mesh. A skin has no `NodeMeshCombo` chunk, so the model reader gives
it one root node named after the file. The `.cga` and `.cgf` files with the second form are static
models with animated parts; they have no bone weights.

### `CompiledBones2` (`0x900`)

A `u32` bone count, then one entry of 68 bytes per bone, then the bone names as NUL-terminated
strings to the end of the chunk. An entry:

| Offset | Size | Field |
|---|---|---|
| 0 | 4 | controller ID |
| 4 | 4 | limb ID (unused) |
| 8 | 4 | parent (`i32`, −1 for a root) |
| 12 | 28 | pose relative to the parent: quaternion `x, y, z, w`, translation `x, y, z` (`f32`) |
| 40 | 28 | pose in the model's space, the same layout |

In the 611 skins of a first sample (29,989 bones) the world pose is the composition of the relative
ones along the parents in every bone: the world position equals the parent's world position plus
the parent's world rotation applied to the relative translation (largest difference 2.3e-5 m), and
the world rotations are of unit length. The test checks the rotation too (`parent ⊗ relative`, up to
sign). A parent comes before its child in every bone of the sample, but the reader does not rely on
it: it checks that each parent exists and that no bone is its own ancestor.

The names are split at the NUL bytes and kept in step with the bones, empty names included; a
name table with fewer names than bones is corrupt. (StarBreaker drops empty names, which shifts
every name after one.)

### `CompiledBones` (`0x901`)

A header of 48 bytes (`u32` bone count, `u32` size of the name table, then 40 bytes of which the
reader uses none), one record of 16 bytes per bone, the name table, then the relative poses of all
bones and then the world poses of all bones, 28 bytes each as above. A record:

| Offset | Size | Field |
|---|---|---|
| 0 | 4 | controller ID |
| 4 | 2 | limb ID |
| 6 | 2 | number of children |
| 8 | 2 | parent (`i16`, −1 for a root) |
| 10 | 4 | unknown (−1, −1) |
| 14 | 2 | index of the node of the `NodeMeshCombo` chunk the bone drives (`Bone::node`) |

`Bone::node` is for the animations of #107. These bones are listed by `model info` but not written to
the GLB of a static model (see ADR-0039).

## Bone weights

The mesh chunk of a skin's `.skinm` has one more data stream than a static model's, a bone map:

| Tag | Element size | Per vertex | `.skinm` of LIVE |
|---|---|---|---|
| `BONEMAP` `0x677C7B23` | 12 | four `u16` bone indices, four `u8` weights | 9,565 |
| `BONEMAP32` `0x6ECA3708` | 24 | eight `u16` bone indices, eight `u8` weights | 10,566 |

That is every one of the 20,131 `.skinm`, one stream each. The eight-bone form is the heads and the
meshes that follow them. In the sample of the test (520 skins and skeleton files with all their
levels, 5,395,751 vertices), 640,854 vertices have more than four bones.

In all 3,998,013 vertices of the first sample:

- the weights of a vertex add up to 255;
- every bone with a weight is a bone of the skeleton, so the index is the position in the bone
  list of `CompiledBones2`, not a node index (the remap through `ObjectNodeIndex` that
  Cryengine-Converter describes concerns the `0x901` form, which has no bone map);
- the weights are in descending order in 99.7 %, not in the rest, and the reader sorts them;
- the strongest bone of 96 % of the vertices is nearer to the vertex than the median bone is, which
  shows that the vertices and the bind poses are in the same space and that the index is the right
  one. The test asserts a share of 90 % over its sample (94.4 % on LIVE); the vertices
  that are not near are hair, teeth and piercings that hang on one bone.

`Influences` holds eight pairs per vertex whatever the stream, strongest first; a bone listed twice
is added up (no file does it as far as the survey goes, it is a guard), and the unused pairs are bone
0 with weight 0.

## The skeleton of a level of detail

The `X_lod<n>.skin` file of a skin has its own `CompiledBones2` chunk, and the bone indices of
`X_lod<n>.skinm` refer to that one. In most files it equals the one of `X.skin`; in 354 levels of
LIVE it differs: the level `_lod1` of the boots `f_nvy_bridgeofficer_uniform_01_boots`
has a skeleton of 13,736 bytes (about 200 bones) where the `.skin` has 960 bytes (14), and its
weights go up to bone 51. Reading such a level against the model's bones fails (some skins did,
before the reader looked at the level's file). The model reader therefore reads the
`X_lod<n>.skin` file of a skinned level and keeps its skeleton in `Lod::bones` when it differs.
`model info` marks such a level with its bone count (`Skeleton` column, `skeleton_bones` in JSON).

## Mesh boxes of skins

#340 made the reader dequantise with the second bounding box and left the first for skins and
characters, which could not be tested on real data. They do not need it: all 20,131 `.skinm` of
LIVE store 20-byte vertices with float positions, none is quantised, and no `.chr` has a mesh.
`the_vertices_of_every_skin_are_floats_so_the_quantisation_box_does_not_matter` asserts it.

## Texture coordinates that are not numbers

14 `.skinm` of LIVE (helmets of the RSI pilot flightsuit and the explorer, the front landing gear
of the 400i, a medical strap; 3,565 vertices) hold a half float NaN or infinity as a texture
coordinate. The reader used to refuse the whole model for it, so these skins could not be opened at
all. The coordinate reads as 0 now and the vertex stays. `Mesh::non_finite_uvs` counts the vertices
that had such a coordinate in either set, and `model info` shows the count per mesh (`Bad UVs`,
`non_finite_uvs` in JSON).

## glTF

See ADR-0039. A skin is written with the bones as nodes, `JOINTS_n` as `u16` and `WEIGHTS_n` as
normalised `u8`, and the inverse bind matrices as `MAT4`, all in glTF's Y up. The checker of the
testkit tests the bind pose through the whole chain: the global matrix of each joint node times
its inverse bind matrix is the identity.
