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
- `p4kx extract <archive> <entry paths…> [--out <folder>] [--conflict <policy>] [--convert [--textures <outputs>] [--mip <level>]]` writes the
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
  `--convert` writes CryXml Entries (`CryXmlB`, `CryXml`, `CRY3SDK`, either byte order) as
  UTF-8 XML under their own path; text XML and every other Entry stay byte-identical. A CryXml
  Entry that cannot be converted fails alone with the reason, and `if-changed` compares an
  existing file with the converted XML. The summary then also counts the converted Entries.
  `--convert` also writes the DataForge Entry (`Data\Game2.dcb`, falling back to
  `Data\Game.dcb`) as one Record XML file per Record below its folder instead of the `.dcb`,
  e.g. `Data/libs/foundry/records/…`, each file under `--conflict`; Records with error values are
  still written, and the summary counts the Records with their error values.
  `--convert` also converts each texture (`<name>.dds`, its parts `<name>.dds.1` … `.N` and any alpha
  tail `<name>.dds.a` with `.1a` … `.Na`) once; the PNG carries the tail in its alpha channel, a
  high-range texture (BC6H) is exported as OpenEXR `<name>.exr` instead, keeping its range, and a
  cubemap as six faces `<name>_posx` … `<name>_negz`:
  `--textures` picks the outputs, by default `merge,png`: `merge` writes one standard DDS under
  the header's path, `png` the mip `--mip <level>` (0, the largest, by default; a texture with
  fewer levels gives its smallest) as `<name>.png`, and `raw` the Entries unchanged as well, with
  `merge` only the parts. With `merge` and no `png`, an alpha tail is merged like any split
  texture under its own path `<name>.dds.a`, so it is not lost. A selected header takes its
  parts from the Archive even when they are not selected. A texture that cannot be converted,
  e.g. a missing part or a format without a decoder, is reported as not converted with the
  reason, its raw Entries are written (the parts only when the merged DDS was written), and the
  Extraction continues with exit code 1. A part selected without its header is written
  unchanged.
  `p4kx extract <archive> --filter <filter>` extracts every Entry the Filter matches instead; a Filter that matches nothing is invalid input.
- `p4kx info <archive>` prints the Channel, the Build (branch, version, change number, BuildId)
  from the `build_manifest.id` next to the Archive, and the number, encryption and sizes of its
  Entries. An Archive without a readable manifest has an unknown Build and still opens.
- `p4kx dcb info <archive>` opens the DataForge of the Archive (`Data\Game2.dcb`, falling back to
  `Data\Game.dcb`) and prints the Entry it was read from, its version and its number of Records,
  structs and enums. DataForge versions 6 and 8 are read; any other version is reported as not
  supported, and a DataForge Entry claiming more than 2 GiB is rejected before it is read.
- `p4kx dcb export <archive> [--format xml] [--combined] [--filter <filter>] [--out <folder>] [--conflict …]`
  writes Records as Record XML ([docs/record-xml.md](docs/record-xml.md)), one file per Record:
  `<record path>.xml`, or `<record path>/<Record name>.xml` for Records that share a record path.
  `--filter` selects Records by record path with the rules of `search`; `--out` and `--conflict`
  work as for `extract`. A value that cannot be read is written as an error at its field and the
  Record is still written; the summary counts these error values. The same Record always gives
  the same bytes. `--combined` writes every selected Record into one file in record-path order,
  named after the DataForge Entry (`Game2.xml`), under one `p4kx:records` root element.
- `p4kx dcb query <archive> "<query>"` runs a Query ([docs/query.md](docs/query.md)) over the
  Records and prints a table: the record path first, then one column per path of `select`, one
  row per Record in record-path order, e.g.
  `p4kx dcb query Data.p4k 'from EntityClassDefinition at "*/spaceships/*"'`. `from <type>`
  selects a type and every type deriving from it, `at "<glob>"` selects by record path with the
  rules of `search`. An invalid Query exits with code 2 and marks its position. `--format csv`
  writes RFC 4180 CSV with a header row and `--format json` (or `--json`) one line of JSON, an
  array of objects keyed by column with typed values; `--out <file>` writes the result to a file
  instead of stdout and prints a summary.
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
- `p4kx texture info <archive> <entry>` prints the format, size, depth, mip count, array size,
  cubemap faces and number of mip parts of a DDS texture; `p4kx texture merge <archive> <entry>
  --out <file>` merges the header Entry and its mip parts `<entry>.1` … `<entry>.N` into one
  standard DDS file that common tools open. Both also take a header file on disk instead of an
  Archive and an Entry, with its parts next to it. Legacy and DX10 headers are read; a missing,
  truncated or oversized part is named in the error and nothing is written. A split cubemap or
  texture array, whose parts hold one mip of every face, merges face by face into a valid
  cubemap or array. `info` also names the alpha or smoothness tail `<entry>.a` with its parts
  `<entry>.1a` … `<entry>.Na`; `merge <archive> <entry>.a` merges the tail on its own.
- `p4kx texture png <archive> <entry> --out <file>` decodes the largest mip of a texture and
  writes it as an 8-bit RGBA PNG; `--mip <level>` picks a smaller one, 0 being the largest. It
  takes a header file on disk like `info` and `merge`. BC1 to BC7 and the uncompressed formats in
  use decode with pure-Rust decoders; any other format fails with a message naming it and exit
  code 1. An alpha or smoothness tail is decoded and merged into the alpha channel. A cubemap is
  written as six face files `<stem>_posx.png` … `<stem>_negz.png` next to `<file>`, or with
  `--cross` as one horizontal cross in `<file>`. A high-range BC6H texture is written as OpenEXR
  with 32-bit float RGB instead, with the extension `.exr`, so its range is not clipped;
  `--tone-map` writes an 8-bit PNG instead, Reinhard tone-mapped and sRGB-encoded (ADR-0010). The
  message names both the requested and the written files, and `--tone-map` on any other texture
  only prints a note. Texture arrays and volumes are not decoded yet.
- `p4kx compare <old> <new> [--entries] [--records] [--filter <filter>] [--report <file>]`
  compares two Archives, e.g. LIVE against PTU. The Entry level joins both central directories
  by path (ignoring case and separator kind) and lists Entries as added, removed, or changed when
  their size or CRC-32C differs; nothing is decompressed, so it finishes in seconds. The Record
  level joins the Records of both DataForges by GUID and lists them as added, removed, moved (same
  GUID at another record path) or changed, with every changed scalar field and its old and new
  value; enum fields show the option names, and a value that cannot be read is a change with the
  reason. A Record that cannot be read at all is listed as unreadable with the reason, and the
  other Records are still compared. It prints both sides (Channel and Build) and the counts per
  change kind, for Entries also per top-level directory. `--filter` selects Entries by path and Records by record path
  with the rules of `search`; `--report` writes the full Comparison as JSON (see
  [docs/cli-json.md](docs/cli-json.md#p4kx-compare-old-new---json)). Every level is compared when
  none is given; the Record level needs a DataForge in both Archives (exit code 5 otherwise). The
  exit code is 0 without and 1 with differences.
- `p4kx compare <old> <new> --diff <entry>` prints the line diff of one Entry between both Archives
  as a unified diff: CryXml is converted to XML first, UTF-16 with a byte order mark is decoded,
  other text and text XML are compared byte for byte, and an Entry on one side only is diffed
  against an empty side. A binary Entry, or one larger than 128 MiB or 4 194 304 lines, gets a note
  saying whether it changed. `--json` prints the hunks as JSON (see
  [docs/cli-json.md](docs/cli-json.md#p4kx-compare-old-new---diff-entry---json)). The exit code is
  0 when the Entry is unchanged and 1 when it was added, removed or changed, also when its lines are
  equal after conversion.
- `p4kx audio info <archive> <entry>` prints the codec, channels, sample rate and duration of a
  Wwise WEM sound; `p4kx audio wav <archive> <entry> --out <file>` writes a PCM, IMA ADPCM or
  PTADPCM WEM as a 16-bit WAV file. Both also take a WEM file on disk instead of an Archive and an
  Entry. PCM, IMA ADPCM, Wwise Vorbis, Opus, PTADPCM and the console codecs are detected;
  converting any other codec fails with an error naming it, and a corrupt or truncated WEM fails
  with the reason.
- `p4kx bnk list <archive> <entry>` prints the ID, codec and duration of every WEM a Wwise BNK
  soundbank embeds; `p4kx bnk extract <archive> <entry> --out <folder>` writes them as
  `<id>.wem`, or with `--format wav` converted to WAV. Both also take a soundbank file on disk. A
  corrupt soundbank fails with an error naming the section.
- `--json` on any command prints its result as one line of JSON instead of text; the shapes are
  documented in [docs/cli-json.md](docs/cli-json.md).

### Exit codes

| Code | Meaning |
|------|---------|
| 0 | Success: every requested Entry was extracted or already existed; `compare`: no differences |
| 1 | Partial failure: some Entries or Records failed or were cancelled, the target or the settings cannot be written, or a texture cannot be read, merged or written; `compare`: the Archives or the `--diff` Entry differ |
| 2 | Invalid input: bad arguments, an Entry path the Archive does not contain (for `compare --diff`: neither Archive contains), a Filter that matches no Entry or Record, a Channel path that does not exist or was never added, a texture file that does not exist, a merge output that is one of its inputs, or `--json` without `--out` for `cryxml` |
| 3 | The Archive or the input file cannot be read: missing, not an Archive, not CryXml or corrupt, also a `compare --diff` Entry that cannot be read, does not hold its stated size or holds corrupt CryXml; a missing texture file is 2 |
| 4 | The Archive changed on disk while it was open, e.g. updated by the RSI Launcher; the Extraction stopped, and Entries written before may be stale |
| 5 | The DataForge cannot be read: the Archive holds none, its version is not supported or it is corrupt |
| 6 | `compare`: the report or the output cannot be written |
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
frame, and comparing the Entries of two Archives within 5 seconds. Its tests are ignored by
default, so `cargo test` and CI report them as `ignored`. Point `P4KX_DATA_P4K` to an Archive,
and `P4KX_DATA_P4K_B` to a second one for the Comparison, and pass `--ignored`; without the
variables the suite fails and says so.

```powershell
$env:P4KX_DATA_P4K = 'C:\Program Files\Roberts Space Industries\StarCitizen\LIVE\Data.p4k'
$env:P4KX_DATA_P4K_B = 'C:\Program Files\Roberts Space Industries\StarCitizen\PTU\Data.p4k'
cargo test --release -p p4kx-archive -p p4kx-extraction -p p4kx-comparison -p p4kx-gui --test real_archive -- --ignored
```

The Archive is only read. The extracted subset goes to a temporary folder below `TMP`, which is
removed afterwards.

## Licence

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT licence](LICENSE-MIT) at your option.
