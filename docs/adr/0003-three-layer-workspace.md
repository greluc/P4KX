# Three-layer crate workspace with enforced dependency direction

The tool covers many game formats and three front-ends (CLI, GUI, MCP server), and the closest prior
art grew single files past 300 KB. The workspace is split into `p4kx-*` crates in three layers:
format crates (foundation, archive, cryxml, dataforge, localization, texture, audio, chunks, model,
character, gfx) depend only on `p4kx-foundation`; service crates (assembly, gltf, blender, comparison, export,
extraction) depend on format crates and on other service crates, without cycles, as `cargo xtask layers` enforces; front-ends (`p4kx`, `p4kx-gui`, `p4kx-mcp`) depend only on
services. Outside the three layers, `p4kx-testkit` (synthetic test data) may only be a
dev-dependency, and `xtask` (repository checks) is tooling that no crate depends on. Every crate
declares its layer in `[package.metadata.p4kx]`; CI rejects a dependency against the direction and
source files above about 400 lines. ADR-0019 lets a format crate build on declared format crates.
Libraries are synchronous and parallelise with rayon; tokio appears only in the MCP front-end.

## Considered Options

- A few large crates with modules: fewer manifests, but the boundaries would not be checked.
- Async libraries throughout: parsing is CPU-bound, so async there would only wrap blocking work.
