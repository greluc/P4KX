# P4K-Extractor

Opens, browses, compares and extracts the `Data.p4k` Archives of Star Citizen. See
[docs/ROADMAP.md](docs/ROADMAP.md) for the plan, [CONTEXT.md](CONTEXT.md) for the terms and
[docs/user-help.md](docs/user-help.md) for using the window.

This is an unofficial fan tool, not affiliated with Cloud Imperium Games.

## Status

Early development (milestone M1). Available today:

- `p4kx list <archive>` lists every Entry of an Archive: compression method, encryption flag,
  compressed size, size and path. Archives in the v1 (ZIP64) and the v2 (`Jiji` trailer) layout
  are recognised automatically; any other layout version is rejected with its number.
- `p4kx search <archive> <filter>` lists the Entries whose path matches a Filter, in the same
  columns as `list`. A Filter with `*` or `?` is a glob over the full path: `*` matches any run of
  characters, separators included, `?` exactly one character, and the glob must match the whole
  path (`*.xml`, `Data\Libs\*`, `*\defaultProfile.???`). Any other Filter matches when it occurs
  anywhere in the path (`Libs\Config`). Both ignore ASCII case and treat `\` and `/` alike; every
  other character, spaces and brackets included, is literal, and an empty Filter matches every
  Entry. No match is not an error.
- `p4kx extract <archive> <entry paths…> [--out <folder>] [--conflict <policy>]` writes the
  given Entries (stored, deflate or zstd, encrypted or not) into a new folder
  `SC-EXPORT-<Channel>-<Build version>-<timestamp>` in the current directory (the Channel is the
  name of the folder holding the Archive, an unknown Build is `UnknownBuild`, the timestamp is
  UTC), or into `--out`. Entry paths match with `\` or `/` and ignore case.
  `--conflict` decides what happens to a file that already exists in the target: `skip` (the
  default) keeps it, `overwrite` replaces it, and `if-changed` replaces it only when its size or
  CRC-32C differs from the Entry; kept files are reported as skipped. Entries are written in
  parallel, each to a temporary file that replaces the destination once complete. Entry paths
  that are absolute, carry a drive prefix, contain `..` or name a Windows device are rejected,
  and nothing is written outside the target. A corrupt Entry fails alone and is named with the
  reason. While stderr is a terminal a progress line shows the Entries done; Ctrl+C cancels the
  run promptly, leaves no partial file and reports the Entries not written as cancelled; a second
  Ctrl+C quits at once. The
  run ends with a summary of extracted, skipped, failed and cancelled Entries.
  `p4kx extract <archive> --filter <filter>` extracts every Entry the Filter matches instead; a Filter that matches nothing is invalid input.
- `p4kx info <archive>` prints the Channel, the Build (branch, version, change number, BuildId)
  from the `build_manifest.id` next to the Archive, and the number, encryption and sizes of its
  Entries. An Archive without a readable manifest has an unknown Build and still opens.
- `p4kx channels` lists the installed Channels with Build version, source and Archive path. It
  looks below `C:\Program Files\Roberts Space Industries\StarCitizen`, below the library folders
  named in the RSI Launcher log (`%APPDATA%\rsilauncher\logs`, see
  [ADR-0005](docs/adr/0005-library-folders-from-the-launcher-log.md)) and in manually added
  folders and Archives. `p4kx channels add <path>` and `p4kx channels remove <path>` manage the
  manual entries in `settings.toml` in `%APPDATA%\P4K-Extractor` (the platform config directory
  elsewhere). `--config-dir`, `--launcher-dir` and `--default-library` (or `P4KX_CONFIG_DIR`,
  `P4KX_LAUNCHER_DIR`, `P4KX_DEFAULT_LIBRARY`) override these locations. Launcher credential
  files and the launcher's settings store are never opened.
- `p4kx cryxml <file> [--out <file>]` converts a CryXml file on disk, e.g. an Entry extracted
  earlier, to UTF-8 XML on stdout or in `--out`. All three signatures (`CryXmlB`, `CryXml`,
  `CRY3SDK`) and both byte orders are read; the XML is indented by two spaces with LF line
  endings, and the same input always gives the same bytes. The output is always well-formed:
  characters a name cannot hold, `:` included, become `_`, a repeated attribute name gets the
  first free suffix `_2`, `_3`, …, and bytes that are not UTF-8 read as `?`. A file that already
  is text XML is written unchanged. A truncated or corrupt file is an error and writes nothing.
- `--json` on any command prints its result as one line of JSON instead of text; the shapes are
  documented in [docs/cli-json.md](docs/cli-json.md).

### Exit codes

| Code | Meaning |
|------|---------|
| 0 | Success: every requested Entry was extracted or already existed |
| 1 | Partial failure: some Entries failed or were cancelled, or the target or the settings cannot be written |
| 2 | Invalid input: bad arguments, an Entry path the Archive does not contain, a Filter that matches no Entry, a Channel path that does not exist or was never added, or `--json` without `--out` for `cryxml` |
| 3 | The Archive or the input file cannot be read: missing, not an Archive, not CryXml or corrupt |
| 4 | The Archive changed on disk while it was open, e.g. updated by the RSI Launcher; the Extraction stopped, and Entries written before may be stale |
| 130 | Interrupted: a second Ctrl+C ended an Extraction at once; temporary `.p4kx-*.part` files may remain |

## Build

Requires the stable Rust toolchain (1.99 or newer).

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

### Real-Archive suite

A second suite checks a real `Data.p4k`: opening and listing it within 10 seconds, its Build,
reading Entries of every compression method and encryption state present (size and CRC-32C),
extracting a small subset, and sorting and scrolling every Entry in the window within one 60 Hz
frame. Its tests are ignored by default, so `cargo test` and CI report them as
`ignored`. Point `P4KX_DATA_P4K` to an Archive and pass `--ignored`; without the variable the
suite fails and says so.

```powershell
$env:P4KX_DATA_P4K = 'C:\Program Files\Roberts Space Industries\StarCitizen\LIVE\Data.p4k'
cargo test --release -p p4kx-archive -p p4kx-extraction -p p4kx-gui --test real_archive -- --ignored
```

The Archive is only read. The extracted subset goes to a temporary folder below `TMP`, which is
removed afterwards.

## Licence

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT licence](LICENSE-MIT) at your option.
