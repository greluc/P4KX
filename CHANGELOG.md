# Changelog

Notable changes of P4K-Extractor, one section per Release. The section of a version becomes the
notes of its GitHub Release; the workspace version names the next Release, and `cargo xtask`
fails while its section is missing.

## [Unreleased]

### Added

- socpak containers in the path tree of the window: a container expands like a folder, opened in
  the background with a spinner and cancelled by collapsing it, and nested containers expand too.
  Selecting a container lists its Entries for search, preview and the extraction of the selected
  or shown Entries. Opened containers are kept per Archive within 1 GiB of memory, closing the
  least recently used collapsed ones first; a corrupt container says so on its row, and a change
  of the Archive on disk closes them.
- `p4kx ui assets <archive> <entry> [--json]` and `p4kx ui extract <archive> <entry> --out
  <folder>`: list the tags, shapes, morph shapes, bitmaps, fonts and sprites of a GFx or SWF UI
  movie with their IDs and export names, and write its bitmaps as PNG and its shapes as JSON in
  twips. Compressed (zlib, LZMA) and uncompressed movies are read, unknown tags are kept and
  counted, a broken tag is reported without failing the movie (exit code 1), and GFx external
  images are decoded from the DDS textures next to the movie, on disk only inside the movie's
  `Data` folder. Also for movie files on disk.
- `p4kx socpak list <archive> <container…>` and `p4kx socpak extract <archive> <container…>`:
  socpak containers, also nested ones, are read from the Archive without extracting them first,
  listed like Archives and extracted with `--filter`, `--out`, `--conflict` and `--convert`.
  Nesting depth, Entry count, decoded size, compression ratio and overlapping Entry data are
  bounded; a corrupt container fails with exit code 3 and an error naming its Entry.
- `p4kx chf to-json <file> [--out <file>]` and `p4kx chf from-json <file> --out <file>`: CHF
  character files as JSON and back. The new `p4kx-character` crate checks magic, size and
  CRC-32C, decompresses the zstd body and reads its inner versions (7 and 8), body type, face DNA,
  item ports and material choices; an unchanged JSON gives back the original file byte for byte,
  an edited one a new file with a correct CRC-32C. Unknown versions and CRC-32C mismatches are
  errors that name them.
- `p4kx-mcp`: a read-only MCP server over stdio for AI clients, with the tools `list_channels`,
  `archive_info`, `list_entries` and `search_entries`. An Archive is named by Channel or by
  absolute path, per call or with `--archive`; listings page with a cursor, at most 1000 items a
  page. `p4kx-mcp.exe` ships in the portable ZIP and the MSI next to `p4kx.exe`.
- `p4kx-mcp` tools `read_entry_text` and `read_entry_bytes`: read an Entry in ranges from a byte
  offset, as text (CryXml converted to XML, UTF-16 by its byte order mark, otherwise UTF-8;
  binary Entries are an error) or as base64. A range is at most 256 KiB of text or 192 KiB of
  bytes; a larger `max_bytes` is clamped and the result says so, and `next_offset` continues.
  Stored Entries, encrypted or not, are read from the offset directly; compressed ones decode at
  most 1 GiB to reach it. At most four Entries are decoded whole for text at a time.
- The Query tab of the window: run Queries over the DataForge of the open Archive in the
  background with progress and cancel, see an invalid Query explained with its line and column
  and underlined in the editor, sort the result table by any column, page through large results,
  and export the result as CSV or JSON exactly as `p4kx dcb query --out` writes it.
- The Record changes of a Comparison in the window: a Records tab lists every added, removed,
  moved, changed and unreadable Record with its record path and type, sorted and filtered like the
  Entries, and a selected Record shows its field changes with old and new value, links by the
  record path of their target and unreadable values with their reason.
- The Export menu of the window with **Record export…**: the Records selected in the Record tree
  or matching a record-path filter as Record JSON, one file per Record or one NDJSON file, into a
  new or an existing folder with a Conflict policy, in the background with progress, cancel and a
  summary of the Records written and those with error values. DataForge and the planned files are
  read once per open Archive and reused by every later export.
- `p4kx_export::write_planned_record_export` with `RecordFilePlans`, and
  `p4kx_extraction::export_planned_record_files` with `RecordFilePlan`: Record exports that plan
  the files of a DataForge's Records once; `RecordExportOptions::progress` reports every Record
  as it ends.
- `p4kx entity loadout <archive> <entity> [--json]`: the default Loadout tree of an Entity found
  by class name, GUID or record path, resolved recursively through nested and overriding
  Loadouts, XML Loadouts and port default items, with each port's sizes, types and helper; a port
  without Entity or with a dangling Reference, an unknown class name or a cycle is listed with the
  reason instead of failing the tree.
- `p4kx dcb query <archive> "<query>"`: Queries over the Records of the DataForge, defined in
  `docs/query.md`: select Records by type (subtypes included) and record-path glob, and list
  scalar fields by dot path as columns of a table, the record path first. Errors name the problem
  and mark its position in the Query, with a suggestion for a mistyped type or field name.
- `p4kx dcb query --format table|csv|json [--out <file>]`: Query results as RFC 4180 CSV with a
  header row or as JSON, an array of objects keyed by column with values typed as booleans,
  numbers and strings, written page by page to stdout or to a file; the columns of the table
  fit every row.
- `where` conditions in Queries: comparisons with `=`, `!=`, `<`, `<=`, `>`, `>=` on numbers,
  strings, GUIDs, enums by name and booleans, string matching with `contains`, `starts with` and
  `like` ignoring case, joined by `and`, `or`, `not` and parentheses. A comparison of
  incompatible types is marked as an error, and a missing value never matches.
- Paths into nested data in Queries: dot paths through inline structs and strong pointers, `[n]`
  for one array element (null when out of range), `[*]` for every element and `[TypeName]` for
  the elements of a polymorphic array of that type, e.g.
  `Components[VehicleComponentParams].crewSize`. A path through `[*]` or `[TypeName]` gives a
  list per cell, a JSON array in CSV and JSON, and a comparison over it holds when one of its
  values holds.
- `->` in Query paths follows a Reference to the Record it links to, or a weak pointer to its
  instance, in `select` and `where`, chained and behind array elements, e.g.
  `AttachDef.Manufacturer->Code` or `escorts[*]->name`. A null or dangling Reference gives null
  and is counted as unresolved in the result summary; `->` where no Reference stands is marked
  as an error.
- `p4kx catalog <archive> --out <file>`: the Basetool catalog in its new, versioned format with
  Channel, Build and completeness per kind, starting with the manufacturers; test, template and
  dummy Records are left out and counted, `--filter` makes it partial, and a warning comes before
  the 64 MB upload limit. `p4kx catalog schema` prints its JSON Schema, checked in as
  `schemas/basetool-catalog.schema.json`.
- English and German names and descriptions in the Basetool catalog, resolved from the Archive's
  `global.ini` files next to their localisation keys, with `\n` as a line feed; a key or
  language the Archive lacks or an empty text leaves that text `null`, the placeholder keys
  `@LOC_EMPTY`, `@LOC_PLACEHOLDER` and `@LOC_UNINITIALIZED` give no text, and `p4kx catalog`
  counts resolved, unresolved and placeholder texts per language. A corrupt `global.ini` ends
  `p4kx catalog` with exit code 3.
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
- The Comparison tab of the window: pick an old and a new Archive from the detected Channels, the
  Recent Archives or any file, compare their Entries, their Records or both off the UI thread with
  progress and cancel, and see both Builds, the counts per change kind and per top-level directory,
  and the sortable list of added, removed and changed Entries, filtered with the rules of the
  search. **Save report…** writes the same JSON as `p4kx compare --report` with the current
  levels and Filter, the Record level included.
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
- `p4kx audio ogg`: rebuild a Wwise Vorbis WEM as a standard Ogg Vorbis file without re-encoding,
  from an Archive or from disk. The headers are rebuilt with the default codebooks and checked
  with a Vorbis decoder, falling back to the aoTuV codebooks; the file plays for the sound's exact
  duration. The codebook libraries come from ww2ogg under the BSD 3-Clause licence, whose notice
  ships as `THIRD-PARTY-NOTICES` in the ZIP and the MSI.
- `p4kx audio wav` and `p4kx bnk extract --format wav` decode Wwise IMA ADPCM and PTADPCM WEMs,
  mono and multi-channel, to 16-bit WAV. IMA ADPCM durations count 64 frames per 36-byte block
  per channel, as Wwise writes them, instead of 65.
- `p4kx audio wav` and `p4kx bnk extract --format wav` decode Wwise Vorbis WEMs to 16-bit WAV for
  the sound's exact duration, with the codebooks the Ogg export picks; the channels keep the
  order and speaker mask of the WEM. A sound that cannot be decoded fails with an error naming
  the WEM and the reason.
- `p4kx bnk list` and `p4kx bnk extract`: list the WEMs a Wwise BNK soundbank from an Archive or
  from disk embeds, with ID, codec and duration, and write them as `<id>.wem`, converted to WAV or
  rebuilt as Ogg Vorbis; the `BKHD`, `DIDX`, `DATA`, `HIRC` and `STID` sections are parsed, and a
  corrupt soundbank fails with an error naming the section.
- `p4kx chunks list` and `p4kx chunks dump`: list the chunks of a CryEngine chunk file (`.cgf`,
  `.cga`, `.skin`, `.chr`, their geometry companions, `.caf`, `.dba`) from an Archive or from disk,
  in the `#ivo` (`0x900`) or the `CrCh` (`0x746`) container, with type, type ID, version, chunk
  ID, byte order, offset and size, unknown types marked; and write one chunk's bytes to a file.
- `p4kx cryxml`: convert a CryXml file on disk (`CryXmlB`, `CryXml`, `CRY3SDK`, either byte
  order) to well-formed UTF-8 XML; text XML is written unchanged.
- Conversion during Extraction: `p4kx extract --convert` and the Conversion switch of the
  Extraction window write CryXml Entries as UTF-8 XML; text XML and every other Entry stay
  unchanged. The switch is remembered with the other Extraction settings, and the summaries and
  `--json` count the converted Entries.
- Texture Conversion during Extraction: with `p4kx extract --convert` and the Conversion switch,
  each texture is merged into one DDS file and its largest mip exported as PNG once, not once per
  part, with its alpha tail in the PNG's alpha channel, a high-range texture as OpenEXR and a
  cubemap as six face files; `--textures merge,png,raw` and `--mip <level>`, and the same choices in the Extraction
  window, pick the outputs and keep the raw parts. A texture that cannot be converted is listed
  with its reason, its raw Entries are written and the Extraction continues. The choices are
  remembered with the other Extraction settings.
- `p4kx dcb info`: the version and the number of Records, structs and enums of the DataForge
  (`Data\Game2.dcb`, falling back to `Data\Game.dcb`); versions 6 and 8 are read, so it works on
  every current Channel, and any other version is reported as not supported.
- `p4kx dcb export`: write Records as Record XML, one file per Record under its record path, with
  owned structs nested, arrays in stored order, References and weak pointers as links (GUID,
  record path, type and, inside the Record, the location) and explicit nulls. Unreadable values
  such as dangling References are reported per value while the rest of the Record is written.
- `p4kx dcb export --format json|ndjson`: write Records as Record JSON (`docs/record-json.md`,
  with a checked-in JSON Schema), one file per Record under its record path or one
  `records.ndjson` with one Record per line in record-path order, with the structure and field
  order of the Record XML, References as objects with `guid`, `path` and `type`, and errors with
  their reason. The new service crate `p4kx-export` writes the Record export in every form.
- `p4kx dcb export --combined`: every selected Record in one Record XML file in record-path
  order, named after the DataForge Entry, built in parallel and streamed to disk, byte-identical
  across runs.
- Conversion of DataForge during Extraction: with `p4kx extract --convert` or the Conversion
  switch of the Extraction window, the DataForge Entry is written as one Record XML file per
  Record below its folder instead of the `.dcb`, with progress and cancellation per Record; the
  summaries and `--json` count the Records and their error values.
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
