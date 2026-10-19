# Format crates may build on other format crates

ADR-0003 lets format crates depend only on `p4kx-foundation`. The M7 spec splits the chunk
container (`p4kx-chunks`) from its interpretation (`p4kx-model`), both format crates, so that new
chunk types need no change in the export code, and `p4kx-model` reads `.mtl` files that may be
CryXml. The model crate therefore depends on `p4kx-chunks` and later on `p4kx-cryxml`. A format
crate may now depend on `p4kx-foundation` and on other format crates; Cargo rejects cycles, and
format crates still never depend on services or front-ends. `cargo xtask layers` checks the new
rule. Decided for issue #102.

## Considered Options

- `p4kx-model` as a service crate: keeps ADR-0003 unchanged, but the spec names it a format
  crate, and every front-end-independent reader would drift into the service layer.
- `p4kx-model` reads chunks through its own trait that a service fills from `p4kx-chunks`: keeps
  the rule, but repeats the chunk table, the type IDs and their names in a second crate.
- One crate for container and interpretation: against the separation the spec asks for.
