# Changelog

Notable changes of P4K-Extractor, one section per Release. The section of a version becomes the
notes of its GitHub Release; the workspace version names the next Release, and `cargo xtask`
fails while its section is missing.

## [Unreleased]

### Added

- `p4kx-mcp` DataForge tools: `get_record` returns a Record by GUID, or every Record of a record
  path, as Record JSON; `search_records` finds Records by record path and type name with Filters,
  in Record order; `list_structs` and `list_enums` list the struct definitions with parent and
  fields (data type, array, target struct or enum) and the enums with their values; `run_query`
  runs a Query and returns its typed columns and one page of rows as `p4kx dcb query --format
  json` writes them, the failed Records and the unresolved References, and an invalid Query as an
  error with its byte span. Every result pages with a cursor and holds at most 256 KiB of items;
  a single Record or row larger than that is an error. The DataForge is read once per open
  Archive, kept open with its indexes and read again when the Archive file changes. A Query is
  limited to 64 columns and 128 comparisons, selecting its Records stops after 30 seconds or when
  the client cancels, and the selection is kept for the pages that follow.

### Changed

- Folders whose names differ only by ASCII case, or whose Entries use `/` and `\` alike, are one
  folder, in the window's tree as in `list_entries` of `p4kx-mcp`, named as its first Entry in
  path order spells it (ADR-0023). `list_entries` lists the Entries of a folder in path order and pages in time proportional to the page; the window and the server share one
  folder index.

## [0.2.0]

The second Release: readable game data. CryXml and DataForge Records as XML and JSON, the
Basetool catalog, the Comparison of two Archives, textures and localisation, and first commands
for Queries, audio, models, UI movies, socpak containers, CHF files and the MCP server.

### Added

- `p4kx wwise resolve <archive> <trigger> [--json]`: resolves an audio trigger to its sound
  variants. Each Wwise event is followed through its Play actions and random, sequence, switch and
  blend containers to the Sound objects; every variant is listed with its soundbank, WEM ID,
  source (embedded, prefetched, streamed or plug-in) and container path, and an embedded WEM is
  located in the soundbank that holds it, with its offset and length. Broken links (a missing,
  unreadable or unfollowed object, a cycle, or a limit of depth, fan-out or size) are listed with
  the object where the chain breaks. Soundbank containers of version 154 are now decoded to their
  children.
- Verification of textures and localisation against all installed Channels, recorded in
  `docs/m5-verification.md`. A texture header Entry that ends with the 8-byte CryEngine trailer
  `CExtCEnd` after its last mip (`cm_inventory_probe_cm.dds`, in every Channel) is no longer
  refused with "not a whole number of mips": the trailer is left out of the merged DDS and of
  the mips read. Opening a texture only checks the trailer's size; `merge` and `read_mip` of a
  mip from the header Entry's tail verify its bytes (`TextureError::UnknownTrailer`), `read_mip`
  of a mip from a part does not.
- GUI: the menu "Record texts" chooses the language of the texts the Record preview shows next
  to localisation keys, in the Record tab and in the Record XML. It lists the languages whose
  `global.ini` the open Archive holds plus "Keys only", follows the interface language until
  one is chosen, and stores the choice in the settings (`record_language`). A key without a text
  in the language is marked "Missing"; control, bidirectional and line feed characters of the
  texts are shown as escapes. A language is read, indexed and copied once per Archive; switching
  it re-renders the open Record from the DataForge already held, without reading it again.
  `LanguageTexts::owned` resolves with an `OwnedTextSet`.
- GUI: a Localisation tab next to Query. It reads the `global.ini` of every language of the open
  Archive off the UI thread behind a loading indicator and shows one row per localisation key
  and one column per language, with missing translations marked. The search matches keys and
  texts of all languages ignoring case, runs off the UI thread while typing and keeps the
  window responsive on a full table; control and bidirectional characters in texts are shown as
  escapes. The key or the text of the selected cell is copied with two actions or Ctrl+C, and
  the table scrolls with the keyboard. All texts are in the German and British English interface
  files.
- Verification of the Basetool catalog against all installed Channels: Records with the words
  `debug`, `deprecated` or `dev` in their path are left out as development leftovers, and the
  measured sizes (at most 21.1 MB against the 64 MB upload limit) are recorded in
  `docs/basetool-catalog.md`.
- An unreadable selected Record no longer aborts `p4kx catalog` and the desktop app's Basetool
  catalog: it is left out and counted as a failure, the catalog is still written, `p4kx catalog`
  lists the failed Records on stderr and under `failed` in `--json` and exits with code 1, and the
  summary window lists them.
- `p4kx dcb export --lang <language>`: Record XML, Record JSON and NDJSON with the text of every
  localisation key next to the key, in the `global.ini` of that language; a key without a text is
  marked missing, a placeholder key stays alone. Additive: without `--lang` the output is byte
  for byte the same as before, and the export report names the `language` only with it.
  `RecordXml::write_with`, `RecordJson::write_with` and `TextResolver` in `p4kx-dataforge`,
  `LanguageTexts` and `RecordExportOptions::texts` in `p4kx-extraction`; the Record JSON Schema
  allows the new `{"key", "text"|"missing"}` object. `RecordFileFormat::write_file` takes the
  texts as an extra parameter.
- `p4kx model info|glb <archive> <entry>`: static `#ivo` models (`.cgf` with their `.cgfm`
  geometry companion) shown with their nodes, LODs, meshes, materials, bones and chunks, and
  exported as one glTF 2.0 binary file (GLB) that Blender opens upright and in metres: one node
  per model node with its transform, one primitive per submesh with positions, normals and
  texture coordinates, and one material per material ID. Unknown chunks are skipped and listed.
  Both commands also take a model file on disk with its companion next to it.
- `p4kx entity audio <archive> <entity> [--json]`: the audio triggers an Entity names, each with
  the Wwise events its ATL definition posts and the soundbanks whose `HIRC` section holds them,
  matched by the events' Wwise short IDs (FNV-1). Triggers without definition, without event or
  in no soundbank are listed with the reason; ATL definitions and soundbanks that cannot be read
  are warned about. The audio index reads every ATL definition and soundbank of the Archive once,
  in parallel.
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
- The Audio tab of the preview: select a WEM to see its codec, channels, sample rate and
  duration, and play it without extracting, with play, pause, seek and volume. PCM, IMA ADPCM,
  PTADPCM and Wwise Vorbis sounds play; another codec, a missing audio device or a sound too long
  to hold is named instead.
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
- **Export › Basetool catalog…** in the window: the Basetool catalog of the open Archive into a
  file chosen in the save dialog, in the background with progress and cancel, writing no partial
  file, and a summary of the entities and skipped Records per kind, the resolved and missing texts
  per language and the size with a warning near or over the basetool upload limit. The Record
  export and the catalog share the DataForge read once per open Archive.
- `p4kx_export::build_catalog_from`: builds the Basetool catalog from a DataForge Entry already
  read.
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
  Channel, Build and completeness per kind, starting with the manufacturers; test, template,
  dummy, debug, deprecated and dev Records are left out and counted, `--filter` makes it partial,
  and a warning comes before the 64 MB upload limit. `p4kx catalog schema` prints its JSON Schema, checked in as
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
- `p4kx audio ogg` and `p4kx bnk extract --format ogg` repackage Opus WEMs (Wwise Opus, Ogg Opus
  in a WEM and Switch Opus) as standard Ogg Opus files without re-encoding, with the pre-skip,
  the channel mapping of the Wwise layout and the sound's exact duration; `p4kx audio wav` and
  `p4kx bnk extract --format wav` decode them at 48 kHz to 16-bit WAV in the speaker order of the
  WEM, with the pure-Rust decoder opus-pure under the BSD 3-Clause licence, whose notice ships in
  `THIRD-PARTY-NOTICES`. The `OggReport` of `p4kx-audio` gains a `codec` field and its
  `codebooks` becomes an `Option`, `None` for Opus; `p4kx audio ogg --json` reports `codec` and a
  `null` `codebooks` for Opus.
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
- Texture groups in the window: a split texture (`.dds` with `.dds.N` parts and an Alpha tail)
  is one row with its total size, its parts one level in below it. Checking a row checks all its
  Entries, a search that matches a part shows the whole texture, sorting by size uses the total
  size, and "Selected" and "Shown" extract every part. Parts without a header stay single rows.
- The long views of the preview and the Comparison scroll with the keyboard: Tab focuses the Text,
  Hex and Image previews, the line diff, the field list of a Record change and the field list of
  the Record preview, and the arrow keys, Page Up, Page Down, Home and End scroll them like the XML
  view.
- The Extraction summary lists the Records that were not written with their record path, GUID and
  reason, as it lists failed Entries.

### Changed

- `p4kx list` folds the parts of a split texture into the row of its header: sizes of the whole
  texture, a ` (+N parts)` suffix on the path and a `parts` field in `--json`. `--parts` restores
  one row per Entry.
- `p4kx extract --filter` and `p4kx socpak extract --filter` now also take every Entry of a
  texture whose header, part or Alpha tail matches.
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
