# SOC chunk types are named by evidence and dump as text by type

Decided for issue #309. Evidence: all 9860 `.soc` files of the LIVE socpaks (9866 on PTU),
nested socpaks included, read by `real_soc_chunks`.

## Decision

- **The `CrCh` table names the types of a `.soc` file** (`p4kx_chunks::crch`). A name is only a
  meaning where the data shows one: `CryXmlB` (`0x0004`, the entities), `CryXmlB2` (`0x0014`,
  `CryXmlB` data of version 3), `AreaShape` (`0x000E`), `IncludedObjects` (`0x0010`, ADR-0081),
  `ModelPaths` (`0x0013`, UTF-8 model paths, one per line) and `Json` (`0x0011`, unverified).
  The types whose layout is not read are named by their ID: `Soc0002`, `Soc0008`, `Soc0009`,
  `Soc000B`, `Soc4002`, `Soc4007`, `Soc5000`.
- **`0x300A` stays unknown.** The issue lists it for `.soc` files, but no `.soc` file of LIVE or
  PTU holds it, while real model files do (the unknown chunk of the `CrCh` model tests); a name
  with the prefix `Soc` would be wrong there.
- **`p4kx chunks dump --text`** writes a chunk as UTF-8 text by its type: `CryXmlB` and `CryXmlB2`
  as XML, `Json` and `ModelPaths` as stored after a UTF-8 check. Any other type has no text form
  (exit code 2); data that is no valid `CryXml` or UTF-8 is exit code 3. Nothing is written on a
  failure.

## Survey

LIVE: `0x0002` 9860 files, `0x0004` 8570, `0x0010` 7296, `0x0008` 1582, `0x000E` 2279,
`0x0009` 404, `0x0013` 4, `0x0014` 3. No file holds `0x000B`, `0x0011`, `0x4002`, `0x4007` or
`0x5000`; PTU adds no type. Every `0x0004` and `0x0014` chunk reads as `CryXml`.
