# P4K-Extractor

Opens, browses, compares and extracts the `Data.p4k` Archives of Star Citizen. See
[docs/ROADMAP.md](docs/ROADMAP.md) for the plan and [CONTEXT.md](CONTEXT.md) for the terms.

This is an unofficial fan tool, not affiliated with Cloud Imperium Games.

## Status

Early development (milestone M1). Available today:

- `p4kx list <archive>` lists every Entry of an Archive in the v1 layout: compression method,
  encryption flag, compressed size, size and path.

## Build

Requires the stable Rust toolchain (1.98 or newer).

```powershell
cargo build --release
.\target\release\p4kx.exe list "C:\Program Files\Roberts Space Industries\StarCitizen\LIVE\Data.p4k"
```

## Development

```powershell
cargo fmt --all
cargo clippy --workspace --all-targets
cargo test --workspace
cargo xtask
```

`cargo xtask` checks that crate dependencies follow the layer direction of
[ADR-0003](docs/adr/0003-three-layer-workspace.md) and that no source file exceeds 400 lines.
Every crate declares its layer in `[package.metadata.p4kx]`.

Tests use synthetic Archives from `p4kx-testkit`; no game files are needed or committed.

## Licence

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT licence](LICENSE-MIT) at your option.
