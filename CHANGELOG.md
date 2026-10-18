# Changelog

Notable changes of P4K-Extractor, one section per Release. The section of a version becomes the
notes of its GitHub Release; the workspace version names the next Release, and `cargo xtask`
fails while its section is missing.

## [Unreleased]

### Added

- `p4kx dcb query <archive> "<query>"`: Queries over the Records of the DataForge, defined in
  `docs/query.md`: select Records by type (subtypes included) and record-path glob, and list
  scalar fields by dot path as columns of a table, the record path first. Errors name the problem
  and mark its position in the Query, with a suggestion for a mistyped type or field name.
- `p4kx texture info` and `p4kx texture merge`: read a DDS texture split into a header and mip
  parts, from an Archive or from files on disk, and merge it into one standard DDS file.
- `p4kx texture png`: decode a mip of a BC1, BC2, BC3 or uncompressed texture with pure-Rust
  decoders and write it as an 8-bit PNG; the largest mip by default, another with `--mip`.
- Cubemaps and alpha tails: `p4kx texture merge` merges split cubemaps and texture arrays face by
  face, `p4kx texture png` writes a cubemap as six face files or with `--cross` as one cross, an
  alpha or smoothness tail `.dds.a` is merged into the alpha channel of the PNG, and
  `p4kx texture info` reports it.
- High-range BC6H textures, signed and unsigned: `p4kx texture png` decodes them with a pure-Rust
  decoder to floating-point RGB and writes OpenEXR (`.exr`, 32-bit float RGB) instead of a PNG, so
  their range is not clipped, cubemaps as six faces or a cross; `--tone-map` writes a Reinhard
  tone-mapped 8-bit PNG instead.
- `p4kx compare <old> <new> [--entries] [--filter] [--report <file>]`: the Entries added, removed
  and changed between two Archives, from their central directories without decompressing, with
  counts per top-level directory, a JSON report with a checked-in schema, and exit code 1 when the
  Archives differ.
- `p4kx compare --records`: the Records of both DataForges joined by GUID and listed as added,
  removed, moved or changed, with each changed scalar field and its old and new value, in the
  summary counts and the JSON report; a Record that cannot be read is listed as unreadable with
  the reason. Without `--entries` or `--records` both levels run.
- `p4kx compare --records` compares the whole Record tree: changes inside nested structs and
  arrays carry their location, e.g. `hull.plates[1].armor`, array elements added or removed at the
  end are listed as such, and changed References show the old and new target's GUID and record
  path; weak pointers compare by their location inside the Record as well.
- `p4kx compare <old> <new> --diff <entry>`: the line diff of one text, XML or CryXml Entry as a
  unified diff, or as JSON hunks with `--json`; CryXml is converted to XML first, an added or
  removed Entry is diffed against an empty side, and a binary Entry is reported as changed or not.
- `p4kx audio info` and `p4kx audio wav`: describe a Wwise WEM sound from an Archive or from disk
  and write PCM WEMs as WAV files; every Wwise codec is detected.
- `p4kx bnk list` and `p4kx bnk extract`: list the WEMs a Wwise BNK soundbank from an Archive or
  from disk embeds, with ID, codec and duration, and write them as `<id>.wem` or converted to WAV;
  the `BKHD`, `DIDX`, `DATA`, `HIRC` and `STID` sections are parsed, and a corrupt soundbank fails
  with an error naming the section.
- `p4kx cryxml`: convert a CryXml file on disk (`CryXmlB`, `CryXml`, `CRY3SDK`, either byte
  order) to well-formed UTF-8 XML; text XML is written unchanged.
- Conversion during Extraction: `p4kx extract --convert` and the Conversion switch of the
  Extraction window write CryXml Entries as UTF-8 XML; text XML and every other Entry stay
  unchanged. The switch is remembered with the other Extraction settings, and the summaries and
  `--json` count the converted Entries.
- `p4kx dcb info`: the version and the number of Records, structs and enums of the DataForge
  (`Data\Game2.dcb`, falling back to `Data\Game.dcb`); versions 6 and 8 are read, so it works on
  every current Channel, and any other version is reported as not supported.
- `p4kx dcb export`: write Records as Record XML, one file per Record under its record path, with
  owned structs nested, arrays in stored order, References and weak pointers as links (GUID,
  record path, type and, inside the Record, the location) and explicit nulls. Unreadable values
  such as dangling References are reported per value while the rest of the Record is written.
- DataForge in the window: a DataForge node below the folders of the path tree shows the Records
  as folders by record path, with record folders where several Records share one. It loads in the
  background the first time it is opened, with a loading indicator, and is kept until the Archive
  is reloaded or changes on disk; an unsupported version or a load error is named on the node. The
  new DataForge tab of the preview shows the version, the numbers of Records, structs and enums and
  the first Records.

- Archive menu entries "Reload" (F5) and "Extract selected…" (Ctrl+E), with the same keyboard
  shortcuts working anywhere in the window.

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
