# P4K-Extractor

Opens, browses, compares and extracts the `Data.p4k` Archives of Star Citizen. See
[docs/ROADMAP.md](docs/ROADMAP.md) for the plan and [CONTEXT.md](CONTEXT.md) for the terms.

This is an unofficial fan tool, not affiliated with Cloud Imperium Games.

## Status

Early development (milestone M1). Available today:

- `p4kx list <archive>` lists every Entry of an Archive: compression method, encryption flag,
  compressed size, size and path. Archives in the v1 (ZIP64) and the v2 (`Jiji` trailer) layout
  are recognised automatically; any other layout version is rejected with its number.
- `p4kx extract <archive> <entry paths…> [--out <folder>]` writes the given Entries (stored,
  deflate or zstd, encrypted or not) into a new folder `SC-EXPORT-<Channel>-<Build>-<timestamp>`
  in the current directory (Channel and Build are `UnknownChannel` and `UnknownBuild` for now,
  the timestamp is UTC), or into `--out`. Entry paths match with `\` or `/` and ignore case.
  Existing files are kept and reported as skipped; Entry paths that are absolute, carry a drive
  prefix, contain `..` or name a Windows device are rejected, and nothing is written outside the
  target. A corrupt Entry fails alone and is named with the reason. The run ends with a summary
  of extracted, skipped, failed and cancelled Entries.

### Exit codes

| Code | Meaning |
|------|---------|
| 0 | Success: every requested Entry was extracted or already existed |
| 1 | Partial failure: some Entries failed or were cancelled, or the target cannot be written |
| 2 | Invalid input: bad arguments or an Entry path the Archive does not contain |
| 3 | The Archive cannot be read: missing, not an Archive or corrupt |

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
