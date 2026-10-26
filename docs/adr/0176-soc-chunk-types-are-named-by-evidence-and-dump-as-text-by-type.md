# SOC chunk types are named by evidence and dump as text by type

Decided for issue #309. Evidence: all 9860 `.soc` files of the LIVE socpaks, nested socpaks
included, read by the `soc_chunks` survey of `p4kx-extraction`.

## Decision

- **The `CrCh` table names the types of a `.soc` file** (`p4kx_chunks::crch`). A name is only a
  meaning where the data shows one: `CryXmlB` (`0x0004`, the entities), `CryXmlB2` (`0x0014`,
  `CryXmlB` data of version 3), `AreaShape` (`0x000E`), `IncludedObjects` (`0x0010`, ADR-0081),
  `ModelPaths` (`0x0013`, UTF-8 model paths, one per line) and `Json` (`0x0011`, unverified).
  The types whose layout is not read are named by their ID: `Soc0002`, `Soc0008`, `Soc0009` and
  `Soc000B` (unverified).
- **`0x0000`, `0x300A`, `0x4000`, `0x4001`, `0x4002`, `0x4003`, `0x4005`, `0x4007` and `0x5000`
  stay unknown.** No `.soc` file of LIVE holds them; the `0x4xxx` types and `0x300A` occur in
  voxel and model files, where a `Soc` name would be wrong.
- **`p4kx chunks dump --text`** writes a chunk as UTF-8 text by its type: `CryXmlB` and `CryXmlB2`
  as XML (a chunk that holds text XML is written as it is), `Json` and `ModelPaths` as stored.
  Text must be UTF-8. Any other type has no text form (exit code 2); data that is no valid
  `CryXml` or UTF-8, UTF-16 included, is exit code 3. Nothing is written on a failure.

## Survey

LIVE: `0x0002` in 9860 files, `0x0004` in 8570, `0x0010` in 7296, `0x0008` in 1582, `0x000E` in
2279, `0x0009` in 404, `0x0013` in 4, `0x0014` in 3. No file holds `0x000B` or `0x0011`. Every
`0x0004` and `0x0014` chunk reads as `CryXml`, every `0x0013` chunk is UTF-8.
