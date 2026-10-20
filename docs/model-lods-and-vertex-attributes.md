# Levels of detail, tangents, vertex colours and the node chain

What the reader of `p4kx-model` assumes about these four, and what the real Archives say about it.
The tests are `crates/p4kx-extraction/tests/real_archive_model_lods.rs` and
`real_archive_vertex_attributes.rs` (`#[ignore]`d, run with `P4KX_DATA_P4K`, and with
`P4KX_DATA_P4K_B` for a second Archive; both pass on LIVE and PTU). Surveyed on 2026-10-20.

## Levels of detail

Level 0 is the geometry companion `<name>m` (`foyer.cgfm`). Level `n` from 1 on is
`<stem>_lod<n>.<extension>m` (`foyer_lod1.cgfm`); `foyer_lod1.cgf` beside it is a chunk file of
one `MeshInfo` chunk (216 bytes in the example) and carries no nodes, which is why the nodes of
every level come from the base `.cgf`. The `LodDistances` chunk of the base file, which `p4kx`
does not read, holds the distances the engine switches at.

LIVE holds 80,537 geometry companions of levels 1 to 5 (`.cgfm` 68,205 of LOD1, `.cgam` 8,669,
`.skinm` 3,663). Their levels are gapless but for `freelancer_pfx.cgfm`, which has only LOD4, so
the reader probes levels 1 to 15 and keeps the gaps. 28 skins also have a `_lod0` file; it is not
read (level 0 is `<name>m`).

The test reads 120 models with a `_lod1` file per extension (`.cgf`, `.cga`, `.skin`) and checks
that the levels ascend from 0 and include 1, that every mesh belongs to a node of the base file,
that 90 % of the level pairs have no more triangles than the one before, that every level
exports as a valid GLB, that a level read alone equals the same level read with the others, and
that a level the model lacks is `NoSuchLod`.

## Tangents

`QTANGENTS` (370,905 mesh chunks of LIVE) and the 8-byte `TANGENTS` stream (20,131, the skins)
store a tangent frame per vertex. The tangent is the packed tangent (`QTANGENTS`, the handedness
is bit 31) or the rotated X axis of the quaternion (`TANGENTS`, the handedness is the sign of W).
The 16-byte `TANGENTS` layout occurs in no file and gives no tangent.

The test derives a tangent from each triangle's positions and texture coordinates
(`dP/du`, and `dP/dv` for the bitangent) and compares: of the vertex corners with a clear
direction (|cosine| above 0.5) the decoded tangent points along `dP/du` in 98 % (`.cgf`), 99 %
(`.cga`) and 99.9 % (`.skin`), and `cross(normal, tangent) * w` points along `dP/dv` in 98 %,
99 % and 98.5 %. The remaining corners are triangles with mirrored or degenerate texture
coordinates. So the tangent runs along the U axis and the handedness is the one of glTF's
`TANGENT.w`, with the texture coordinates as stored (V down, as glTF).

This frame is the engine's own. Whether the engine's normal maps need a green-channel flip when
a viewer applies them with it is open; materials (#104) decide.

## Vertex colours

Each vertex of the 16-byte (quantised) and 20-byte (float) vertex stream carries four colour
bytes before the two half floats of the texture coordinates. Of the meshes of 400 files per
extension, 286 of 774 (`.cgf`), 787 of 944 (`.cga`) and 293 of 400 (`.skin`) hold a colour other
than opaque white. The most frequent are white with a varying alpha, magenta (`.cga`), red masks
(`.cgf`), a purple (`.skin`) and black.

The order of the bytes cannot be read from the data: the colours are masks and tints whose red
and blue channels have no check (as normals have the face normal). StarBreaker reads
blue, green, red, alpha; Cryengine-Converter reads red, green, blue, alpha. `p4kx` follows
StarBreaker (ADR-0028) and the `B8G8R8A8` format CryEngine uses for vertex colours. A reader that
finds the order wrong swaps one function.

The second colour stream (`COLORS2`, `0xD9EED421`) and the second UV set (`NORMALS2` of 4 bytes,
#343) are not read.

## The node chain

The nodes of a `.cga` or `.cgf` carry a transform to their parent, and the vertices of a mesh
are in its node's space. glTF has the same structure, so the writer gives every node its
transform and parent. To see whether that is right, the test places each mesh's vertices through
the whole chain of its node and compares the bounds with the box of the `MeshInfo` chunk, which
the model reader does not use. Of the models with several nodes, in a sample of 600 files per
extension of LIVE, the chain fits the box for 27 of 32 `.cgf` and 77 of 118 `.cga` models; the
node's own transform alone fits 22 and 41, and no transform 22 and 44. The remaining models have
a larger box than their geometry (hidden or removed parts), not a displaced one. The test asserts
that the whole chain fits better than either alternative and at least half of the models.
