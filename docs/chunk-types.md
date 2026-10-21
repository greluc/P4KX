# Chunk types of the `#ivo` container

The table in `crates/p4kx-chunks/src/types.rs` is checked against the real Archives by the
`#[ignore]`d test `every_chunk_type_of_the_archive_is_counted_and_the_unknown_ones_are_listed`
(`crates/p4kx-extraction/tests/real_archive/chunk_types.rs`). It reads every chunk file of the
Archive (`.cgf`, `.cga`, `.skin`, `.chr`, their geometry companions, `.caf`, `.dba`, `.aim`,
`.img`, `.soc`, and the other extensions it finds by signature) and of every socpak Container, and
counts the chunk types with version, chunk and file counts, extensions and example paths.

Surveyed on 2026-10-20: LIVE (955,057 chunk files, 9,668 socpaks), PTU (950,473; 9,635) and
TECH-PREVIEW (990,631; 10,646). No file failed to parse. The three Archives hold the same set of
`#ivo` IDs, so the table below is valid for all of them; the counts are those of LIVE.

## The four disputed IDs

| ID | Name | Evidence |
|---|---|---|
| `0x92914444` | `MeshInfo` | 435,929 chunks, 96 to 648 bytes, in `.cgf`, `.cga` and `.skin`, never in a geometry companion. Words 1 to 3 are vertex count, index count and subset count (a head: 5,584, 32,178, 1), words 6 to 11 an axis-aligned box whose minimum is not above its maximum in every chunk. Counts and a box: kept. |
| `0x2B7ECF9F` | `PositionBonemap` | 1,016 chunks in `.cga` (1,003) and `.cgf` (13); the examples read also hold a `CompiledBones` chunk. Header of 40 bytes with a dimension `D` at byte 16 and the table size at byte 20, then a table of `D * D * 8` bytes: in all 1,016 chunks `D` is 64, 128, 256 or 512 (827, 83, 66, 40), the table size is `D * D * 8` and the chunk size is 40 plus the table. In the two examples read, the table holds byte values up to 255 and is two thirds zero. The measured fact is a square table of 8-byte cells; reading it as a map of bone indices and weights looked up by position is an interpretation (StarBreaker's and scdatatools' name), but the chunk holds no bounding data. Renamed from `BoundingData`. |
| `0x58DE1772` | `StatObjPhysics` | One chunk in each of 134,967 files: every `.cgf` (125,401) and `.cga` (9,566), none in `.skin`, `.chr` or a geometry companion; 80 bytes to 195 MB. The render geometry is in `Skin2` of the companion, so this is not LOD data. Layout in all 134,967 chunks: counts at bytes 0, 16 and 40, a table of 8-byte entries (size, index) from byte 80, and the blobs it sizes (header, table and blobs add up to the chunk size within 16 bytes). In the example read, a model with 33 meshes has 33 large blobs, indexed per mesh, next to 25 blobs of 140 bytes of float data (rotation-matrix like), the form of per-mesh physics proxies. Renamed from `LodMeshData`, agreeing with StarBreaker and with scdatatools (`U6_StatObj_Physics`). |
| `0xBE5E493E` | `ExportFlags` | One chunk in each of 155,664 files: every `.cgf`, `.cga`, `.skin` and `.chr`, none in companions. 16 bytes (65,476 files) or 128 bytes (90,188 files, with 32 bytes after the head that look like a hash). Head words are `0, 1, 3, N` (566 files: `0, 1, 1, 0`) with `N` zero or a number below 2^24, the same in many files, like a tool build. The same place and role as the `CrCh` `ExportFlags` chunk: one per model file, written by the exporter. Renamed from `AssetMetadata`. |

The earlier names came from the comments of Cryengine-Converter, which parses none of these chunks.

## Other IDs added

Named after StarBreaker (MIT) and scdatatools (MIT), which read them from the game binary; the
sizes show each is a real, regular chunk:

| ID | Name | Files | Extension |
|---|---|---|---|
| `0x0A2485B6` | `RigLogic` | 130 | `.skin` (heads) |
| `0x7E035555` | `RigInfo` | 66 | `.skin` (heads) |
| `0xF7086666` | `ProtosInfo` | 30 | `.skin` |
| `0x2B8A2C29` | `SimTopology` | 3 | `.skin` |
| `0xFF277A9A` | `SimDeformation` | 24 | `.skinm` |
| `0x57A38888` | `BShapesGpu` | 132 | `.skinm` (heads) |
| `0x90C62222` | `PhysicalHierarchy` | 826 | `.chr`, `.skin` |
| `0xB32459D2` | `VisAreas` | 171 | `.cgfm`, `.cgam` |
| `0x1E14A062` | `VfxHelpers` | 4 | `.cga` |
| `0xC1C36AFE` | `VisibilityGroups` | 1 | `.cga` |
| `0x1BBC4103` | `Skeleton` | 551 | `.aim` |
| `0xF5C6EB5B` | `BShapes` | 551 | `.aim` |

`BShapes` is the `.aim` form; the ID `p4kx` had for it (`0x875CCB28`) and the old `BShapesGpu`
(`0x57A3BEFD`) occur in no Archive.

## Table entries no measured Archive holds

`0x57A3BEFD` (`BShapesGpu`), `0x875CCB28` (`BShapes`), `0x8335674E` (`MtlName`), `0x83353533`
(`MtlNameVariant`), `0x90C66666` (`CompiledPhysicalBones320`), `0x90C687DC`
(`CompiledPhysicalBones`), `0x9293B9D8` (`Mesh`) and `0xB875B2D9` (`Skin`) occur in none of
LIVE, PTU and TECH-PREVIEW. All eight are unverified and are kept for files of older builds, with
the names of Cryengine-Converter and scdatatools. Three have a later form that the data holds:
`0x8335674E` became `0x83353333`, `0xB875B2D9` became `0xB8757777`, and `0x57A3BEFD` became
`0x57A38888`. `0x875CCB28` has the `.aim` form `0xF5C6EB5B` instead. For `0x83353533`,
`0x90C66666`, `0x90C687DC` and `0x9293B9D8` no counterpart is known: of the physical bones only
`PhysicalHierarchy` `0x90C62222` occurs, and `Mesh` is carried by `Skin2`.

## Still unknown

Present in all three Archives, no name evidence: `0x322BA3C7` (`.dba`, 8 files), `0x570FCCBA`
(`.dba`, 15), `0x822D7322` (`.caf`, 168), `0xC2387AD1` (`.caf`, 427), `0x3C2CC7AB` (`.aim`, 5),
`0x370275B7` and `0x6B8DF259` (`Animations.img`, `DirectionalBlends.img`, one each). They are
listed as unknown by `p4kx chunks list`.

## `CrCh`

The Star Citizen types of `.soc` files and the types `0x0000`, `0x300A`, `0x4000`, `0x4001`,
`0x4003` and `0x4005` of the survey are left to #309. The `CrCh` model chunks (`Node`, `Mesh`,
`MeshPhysicsData`, `MtlName`) of the one model file are in docs/crch-models.md.
