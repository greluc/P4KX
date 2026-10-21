# Older `CrCh` models

Issue #108. The survey is the `#[ignore]`d test `the_crch_files_of_the_archive_are_counted_by_container_version_and_chunk`
(`crates/p4kx-extraction/tests/real_archive/crch_survey.rs`). It reads every Entry of an Archive
that is not encrypted, so it needs no Archive key, and counts the files by signature and
container version and the `CrCh` chunks by type, chunk version and byte order. The Entries it
leaves out are 4,157 `.xml`, 118 `.json` and one `.chf` in LIVE: none of them is a chunk file.

## What the Archives hold

Surveyed on 2026-10-20 in LIVE, PTU and TECH-PREVIEW. Every `CrCh` file has the container version
`0x746`; no file has the signature `CryTek` or the container versions `0x744` and `0x745`, and
no chunk has a big-endian flag.

| Extension | `CrCh` files in LIVE | PTU | TECH-PREVIEW | Content |
|---|---|---|---|---|
| `.vvg` | 1 | 1 | 1 | the model below |
| `.dst` | 141 | 141 | 297 | terrain textures, chunk type `0x0000` v`0x8` |
| `.cigvoxel` | 27 | 27 | 27 | chunk type `0x4000` v`0x6` |
| `.cigvoxelheader` | 27 | 27 | 27 | chunk type `0x4001` v`0x3` |

Every `.cgf`, `.cga`, `.skin`, `.chr`, geometry companion, `.caf` and `.dba` is `#ivo`. The
`CrCh` chunk types of the Star Citizen `.soc` files are left to #309 (docs/chunk-types.md).

**The one `CrCh` model of the Archives** is
`Data\Objects\Spaceships\Ships\ANVL\Carrack\Exterior\ANVL_Carrack_no_colide.vvg`, the same in all
three Archives:

| Index | Chunk | Version | Size | Read as |
|---|---|---|---|---|
| 0 | `SourceInfo` `0x1013` | `0x0` | 148 | skipped |
| 1 | `Timing` `0x100E` | `0x918` | 52 | skipped |
| 2 | `ExportFlags` `0x1015` | `0x1` | 164 | skipped |
| 3 | `MtlName` `0x1014` | `0x802` | 196 | the material library |
| 4 | `Helper` `0x1001` | `0x744` | 16 | skipped |
| 5 | `Node` `0x100B` | `0x824` | 204 | the root node |
| 6 | `MeshPhysicsData` `0x1018` | `0x800` | 18,705 | the triangles |
| 7 | `Mesh` `0x1000` | `0x801` | 260 | the header naming chunk 6 |
| 8 | `Node` `0x100B` | `0x824` | 204 | the node `$physics_proxy_..._mesh` |
| 9 | unknown `0x300A` | `0x1` | 24 | skipped, reported as unknown |

It is a collision proxy of the Carrack: 913 vertices and 474 triangles inside a box of
60.2 × 122.2 × 23.3 m. Its geometry is not in `DataStream` chunks but in the physics data.

## The layouts that are read

Only layouts that real files use are read. All values are little-endian.

**`Node` `0x824`**, 204 bytes (`CryHeaders.h` `NODE_CHUNK_DESC_0824`): name in 64 bytes, NUL
terminated; at `0x40` the chunk ID of the object (`Mesh` or `Helper`), at `0x44` the chunk ID of
the parent or `0xFFFFFFFF`, at `0x48` the number of children, at `0x4C` the chunk ID of the
`MtlName`, at `0x50` four bytes of flags; at `0x54` a 4×4 matrix of `f32` with the translation in
the last row (the row-vector convention); then position, rotation and scale, three controller IDs
and the length of the property string, which are not read. The matrices of the two nodes are the
identity (to 2 × 10⁻⁷), so the files do not show whether the matrix is a world or a local
transform. The reader takes it for the local transform, relative to the parent, as
Cryengine-Converter (`ChunkNode.LocalTransform`) and the engine's CGF loader (`localTM`) do: this
is a decision (ADR-0041), not a measurement.

**`Mesh` `0x801`**, 260 bytes: flags, flags 2, then at `0x08` the numbers of vertices and indices
and of subsets, at `0x14` the chunk ID of the subsets, at `0x1C` sixteen chunk IDs of data
streams, at `0x5C` four chunk IDs of physics data, at `0x6C` the bounding box (minimum, maximum),
and 30 reserved words. The `Mesh` of the Archives declares no vertices, indices, subsets or
streams and names the physics data `29` in the first slot. The reader returns the first
physics data ID; a header that declares streamed data is refused with `StreamedMesh`, because no
file uses that layout and a reader built only from the specification would be untested.

**`MeshPhysicsData` `0x800`**: a header of 24 bytes whose first word is the length of the physics
data, then the data. In the data of the Carrack proxy (offsets from the start of the data):

| Offset | Content | Evidence |
|---|---|---|
| `0x00` | the first header field, not a version | not read |
| `0x44` | primitive type `1` (Cryengine-Converter `PhysicsData.cs`: 0 cube, 1 polyhedron, 5 cylinder, 6 other) | read; any other value is refused with `PhysicsPrimitive` |
| `0x48` | number of vertices `913`, number of triangles `474` | read; the vertex table below fills exactly the declared count and its box equals the box in the `Mesh` header (−30.118…30.118, −64.801…57.380, −14.882…8.388) |
| `0x50` | `8` and `64`: probably the least and most triangles per node of the bounding-volume tree | not read |
| `0x5A` | 913 vertices of three `f32` | read |
| after them | 474 triangles of three `u16` vertex indices; the largest index is 835 | read, every index is checked against the vertex count |
| after them | about 500 bytes that are zero but for one word, then the bounding-volume tree | not read; the zeros are probably the material IDs of the triangles, so every triangle has the material ID 0 here |

The data before `0x44`, from `0x4C` to `0x5A` and the tree are not decoded; the triangles do not need them. The offsets `0x48` and
`0x5A` are those of this one file. A file with another layout is refused by the primitive type or
by the bounds checks, or its vertices and
triangles are misread; that cannot be told from one sample.

**`MtlName` `0x802`**: the first 128 bytes are the material library, the same field as in `#ivo`.
The remaining 68 bytes (a word `0x10`, a word `0x1100` and `0xFF` bytes) are not read.

## What is not read

Layouts no file of the three Archives uses are not implemented: `Mesh` `0x800` and `0x802` and
`DataStream` and `MeshSubsets` chunks (the streamed meshes of Cryengine-Converter, whose
`MtlName`, `Node` `0x823` and compiled-bones layouts are likewise unused), the `CryTek`
container, big-endian chunks, skeletons and morph targets in `CrCh` (`CompiledBones` `0x2000`
and others), and `CrCh` animations. Their chunks are listed as skipped (known type) or unknown
by `p4kx model info`. Big-endian chunks of the chunks the model reads are refused with
`BigEndian`.

## The model the reader builds

- One node per `Node` `0x824` chunk, in table order; the parent by chunk ID. Duplicate chunk IDs,
  an unknown parent, a cycle and a singular parent matrix are corrupt chunks.
- One level of detail, LOD0, without a geometry companion: one mesh for each node whose object is a
  `Mesh` chunk naming physics data. The mesh has positions and a triangle list, no normals,
  tangents, colours or texture coordinates, and one submesh of material ID 0 over all indices.
- `material_library` from the first `MtlName` `0x802` chunk; the library is resolved as for `#ivo`.
- `Model::chunks` lists every chunk with `read`, `skipped` or `unknown`.

The export is a node tree and a mesh like the `#ivo` export. It differs in what the files hold:
no normals (viewers compute flat ones), no texture coordinates, one material.
