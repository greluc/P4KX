# Changelog

Notable changes of P4K-Extractor, one section per Release. The section of a version becomes the
notes of its GitHub Release; the workspace version names the next Release, and `cargo xtask`
fails while its section is missing.

## [Unreleased]

### Added

- `p4kx texture info` and `p4kx texture merge`: read a DDS texture split into a header and mip
  parts, from an Archive or from files on disk, and merge it into one standard DDS file.
- `p4kx texture png`: decode a mip of a BC1, BC2, BC3 or uncompressed texture with pure-Rust
  decoders and write it as an 8-bit PNG; the largest mip by default, another with `--mip`.
- `p4kx compare <old> <new> [--entries] [--filter] [--report <file>]`: the Entries added, removed
  and changed between two Archives, from their central directories without decompressing, with
  counts per top-level directory, a JSON report with a checked-in schema, and exit code 1 when the
  Archives differ.
- `p4kx cryxml`: convert a CryXml file on disk (`CryXmlB`, `CryXml`, `CRY3SDK`, either byte
  order) to well-formed UTF-8 XML; text XML is written unchanged.
- Conversion during Extraction: `p4kx extract --convert` and the Conversion switch of the
  Extraction window write CryXml Entries as UTF-8 XML; text XML and every other Entry stay
  unchanged. The switch is remembered with the other Extraction settings, and the summaries and
  `--json` count the converted Entries.
- Texture Conversion during Extraction: with `p4kx extract --convert` and the Conversion switch,
  each texture is merged into one DDS file and its largest mip exported as PNG once, not once per
  part; `--textures merge,png,raw` and `--mip <level>`, and the same choices in the Extraction
  window, pick the outputs and keep the raw parts. A texture that cannot be converted is listed
  with its reason, its raw Entries are written and the Extraction continues. The choices are
  remembered with the other Extraction settings.
- `p4kx dcb info`: the version and the number of Records, structs and enums of the DataForge
  (`Data\Game2.dcb`, falling back to `Data\Game.dcb`); versions 6 and 8 are read, so it works on
  every current Channel, and any other version is reported as not supported.

### Changed

- The window follows the binding KRT design system and its desktop spec: KRT buttons, toggles,
  tabs, tables, tree, modals and alerts, Lato in three weights, the P4K-Extractor glyph as window
  icon and header mark, an empty state with an "Open Archive" action.

## [0.1.0]

The first Release: open, browse and extract Star Citizen Archives.

### Added

- Reading Archive layouts v1 and v2: ZIP64 central directory, Store, Deflate and Zstandard
  Entries, AES-128-CBC encrypted Entries.
- Detection of the installed Channels and the Build of each, plus manually added folders and
  Archives.
- The `p4kx` command line: `list`, `search` and `extract` with JSON output and documented exit
  codes.
- The P4K-Extractor window: Channel picker, drag and drop, recent Archives, path tree, sortable
  Entry table, search with quick-filter chips, text and hex preview, and Extraction of selected,
  shown or all Entries with progress, cancellation and conflict policies.
- A notice with a reload offer when the open Archive changes on disk.
- German and British English interface, KRT look.
- A portable ZIP and a per-user MSI installer.
- An optional check for a newer Release at start, off by default and switchable in the Help menu.
