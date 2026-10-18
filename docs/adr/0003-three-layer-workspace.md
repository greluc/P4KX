# Three-layer crate workspace with enforced dependency direction

The tool covers many game formats and three front-ends (CLI, GUI, MCP server), and the closest prior
art grew single files past 300 KB. The workspace is split into `p4kx-*` crates in three layers:
format crates (foundation, archive, cryxml, dataforge, localization, texture, audio, chunks, model,
gfx) depend only on `p4kx-foundation`; service crates (assembly, gltf, blender, comparison, export,
extraction) depend on format crates; front-ends (`p4kx`, `p4kx-gui`, `p4kx-mcp`) depend only on
services. CI rejects a dependency against the direction and source files above about 400 lines.
Libraries are synchronous and parallelise with rayon; tokio appears only in the MCP front-end.

## Considered Options

- A few large crates with modules: fewer manifests, but the boundaries would not be checked.
- Async libraries throughout: parsing is CPU-bound, so async there would only wrap blocking work.
