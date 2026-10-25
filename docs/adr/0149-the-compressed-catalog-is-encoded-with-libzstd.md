# The compressed catalog is encoded with libzstd, the one C dependency of the encoders

The Basetool catalog is written as `.json.zst` at zstd level 19 with long-distance matching and a
window of at most 2^27 (ADR-0054). That is the one place P4KX links the C library libzstd, through
`zstd` 0.14.3, `zstd-safe` 8.2.0 and `zstd-sys` 2.1.2+zstd.1.5.7 (libzstd 1.5.7), the newest
versions on crates.io on 2026-10-25. ADR-0013, ADR-0014 and ADR-0016 weighed C against pure Rust
for reading and writing game data and chose pure Rust; ADR-0014 lists linking libzstd as a
drawback. This ADR keeps that choice for everything that reads the Archive and states why the
catalog encoder is the exception. Decided for issue #443, the follow-up of #407.

- **Only the encoder, only trusted data.** The `zstd` crate is a dependency of `p4kx-export`
  alone, which uses its streaming `Encoder` on the JSON the catalog serialiser produced itself.
  The two other crates that name it, `p4kx` and `p4kx-gui`, use it as a dev-dependency to decode
  what the tests wrote. Every byte read from the Archive, including every zstd frame in it, is
  still decoded by the pure-Rust `ruzstd` (`p4kx-archive`, `p4kx-character`), so no C code ever
  sees untrusted input.
- **Legacy decoders off.** The workspace declares `zstd` with `default-features = false`, which
  turns off `legacy` (the decoders for the zstd formats before v0.8), `zdict_builder` and
  `arrays`. No Cargo feature that links more of libzstd than the encoder and the test decoder
  needs is enabled.
- **ruzstd cannot write what the catalog needs.** In `ruzstd` 0.9.1 only the levels
  `Uncompressed` and `Fastest` (roughly zstd level 1) are implemented; `Default`, `Better` and
  `Best` are marked unimplemented. The catalog has to fit the 64 MB upload limit of the Basetool with room to
  grow, and level 19 with long-distance matching is what ADR-0054 measured (19.3 MB of JSON
  become 1.54 MB at `--long=27`). `ruzstd` keeps writing
  the CHF frame (ADR-0014), where no level matters.
- **The C toolchain is already needed.** `ring`, through `rustls` and `ureq` in `p4kx-update`,
  already compiles C for `p4kx-gui`, so building P4KX needs a C compiler on every platform
  whether or not `zstd-sys` is built.
- **Dev builds stay fast.** `[profile.dev.package.zstd-sys]` sets `opt-level = 3`, because an
  unoptimised libzstd would turn the level 19 encoder of every test and local run into minutes.
- **Licence.** libzstd is used under its BSD-3-Clause licence; the notice is part of
  `THIRD-PARTY-NOTICES`, checked by `cargo xtask notices`.

## Considered Options

- `ruzstd` for the encoder too: no C, but fast levels only, so a larger file than the measured level 19;
  rejected until it implements the higher levels.
- `structured-zstd` 0.2.5 and `zstd-pure-rs` 0.2.0, pure-Rust encoders with all levels: both are
  young 0.x crates with a handful of releases; their frames were not checked against the Basetool
  importer and a long-distance matching window of 2^27 was not confirmed. Worth re-evaluating
  when one of them is established, since it would remove the exception.
- gzip with `flate2`: the Basetool accepts `.json.gz`, but 4.35 MB against 1.54 MB at the same
  catalog (ADR-0054) leaves less room under the upload limit.
- Plain JSON only: over the upload limit as soon as the catalog grows; the windows and the CLI
  warn about a plain catalog close to the limit for exactly this reason.
