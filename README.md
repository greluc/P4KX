<p align="center">
  <img src="docs/design/p4kx-logo.svg" alt="P4KX logo: data brackets around an X" width="128" height="128">
</p>

# P4KX

Opens, browses, compares and extracts the `Data.p4k` Archives of Star Citizen. See
[docs/ROADMAP.md](docs/ROADMAP.md) for the plan, [CONTEXT.md](CONTEXT.md) for the terms and
[docs/user-help.md](docs/user-help.md) for using the window.

P4KX is an unofficial Star Citizen fan project, not affiliated with the Cloud Imperium
group of companies and not endorsed by it. It ships no game data and reads only your own Star
Citizen installation. You are responsible for complying with the Star Citizen End User Licence
Agreement (EULA) and Terms of Service when you use it.

All content not authored by the P4KX contributors or its users is the property of its
respective owners. Star Citizen®, Roberts Space Industries® and Cloud Imperium® are registered
trademarks of Cloud Imperium Rights LLC.

Copyright (c) 2026 the P4KX contributors

## Status

Early development (milestone M1). Available today:

- `p4kx list <archive>` lists every Entry of an Archive: compression method, encryption flag,
  compressed size, size and path. Archives in the v1 (ZIP64) and the v2 (`Jiji` trailer) layout
  are recognised automatically; any other layout version is rejected with its number. A split
  texture (`.dds` with its parts and alpha tail) is one row with the size of the whole texture and
  ` (+N parts)` behind its path (`parts` in `--json`); `--parts` lists every part as an Entry of
  its own.
- `p4kx search <archive> <filter>` lists the Entries whose path matches a Filter, in the same
  columns as `list` but unfolded: every matching part is a row of its own. A Filter with `*` or `?` is a glob over the full path: `*` matches any run of
  characters, separators included, `?` exactly one character, and the glob must match the whole
  path (`*.xml`, `Data\Libs\*`, `*\defaultProfile.???`). Any other Filter matches when it occurs
  anywhere in the path (`Libs\Config`). Both ignore ASCII case and treat `\` and `/` alike; every
  other character, spaces and brackets included, is literal, and an empty Filter matches every
  Entry. No match is not an error.
- `p4kx grep <archive> <text> [--in records,loc,entries] [--filter <filter>] [--lang <language>,…]
  [--limit <n>] [--per-item <n>] [--format table|json]` finds a text in the contents: in the string,
  localisation key and enum values of all Records (with the field path), in the texts of each
  language's `global.ini` and in the lines of text and CryXml Entries (CryXml converted to XML
  first). It matches part of a word and ignores case, so `Gladi` finds `AEGS_Gladius`. Nothing is
  indexed or stored: each search scans in parallel and streams, in a second or two on LIVE.
  Results are bounded (`--limit` per source, `--per-item` per Record or Entry) and say whether more
  exist; Entries with binary data, above `--max-entry-size` or encrypted without the Archive key
  are skipped and counted on stderr ([docs/cli-json.md](docs/cli-json.md)). The window searches
  the same way in its *Search* tab.
- `p4kx verify <archive> [--filter <filter>]` checks the data of every Entry (or those the Filter
  matches) against the CRC-32C the Archive directory states, in parallel and without writing
  anything, and prints every mismatch with the stored and the computed value; an Entry that
  cannot be read is reported as failed. The checksum is computed while the data streams past, so
  a run needs one read and decode of the Archive (timings in [docs/verify.md](docs/verify.md)).
  Exit code 1 when an Entry mismatches or fails. `p4kx extract --verify` does the same for each
  Entry while it is written: a mismatching Entry is not written, and the summary counts it
  (`mismatched` in `--json`). `p4kx socpak verify <archive> <container…>` and `socpak extract
  --verify` check a container with the plain CRC-32 of its directory. Checking is off unless
  asked for.
- `p4kx extract <archive> <entry paths…> [--out <folder>] [--conflict <policy>] [--convert [--textures <outputs>] [--mip <level>] [--audio <format>]]` writes the
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
  `--convert` also converts WEM sounds (`<name>.wem`) as `--audio` says, by default `ogg`: Wwise
  Vorbis is rebuilt as Ogg Vorbis and Opus repackaged as Ogg Opus, `<name>.ogg`, without
  re-encoding, and every other codec decoded to a 16-bit PCM WAV `<name>.wav`; `wav` decodes every
  sound to WAV, and `raw` keeps the WEMs. Each soundbank (`<name>.bnk`) is written unchanged and
  unpacked into the folder `<name>` next to it, each embedded WEM named by its ID and written in
  the same format. A sound whose codec cannot be decoded is reported as not converted, naming the
  codec, and written unchanged as `.wem`; plug-in media and the prefetched start of a streamed WEM
  are written unchanged without a report. Every file follows `--conflict`, `if-changed` comparing
  with the converted output. Sounds and soundbanks are converted on a few threads of their own,
  within a memory budget of 1 GiB.
  `p4kx extract <archive> --filter <filter>` extracts every Entry the Filter matches instead; a Filter that matches nothing is invalid input.
- `p4kx info <archive>` prints the Channel, the Build (branch, version, change number, BuildId)
  from the `build_manifest.id` next to the Archive, and the number, encryption and sizes of its
  Entries, and the number of Entry issues (ADR-0025). An Archive without a readable manifest has
  an unknown Build and still opens. A malformed field of one directory record, such as a path
  that is not UTF-8, an encryption value other than 0 or 1 or v2 data outside the file, does not
  stop an Archive from opening: `info`, `list` and `extract` print up to ten `warning: Entry …`
  lines to stderr for the Entries concerned.
- `p4kx dcb info <archive>` opens the DataForge of the Archive (`Data\Game2.dcb`, falling back to
  `Data\Game.dcb`) and prints the Entry it was read from, its version and its number of Records,
  structs and enums. DataForge versions 6 and 8 are read; any other version is reported as not
  supported, and a DataForge Entry claiming more than 2 GiB is rejected before it is read.
- `p4kx dcb export <archive> [--format xml|json|ndjson] [--combined] [--filter <filter>]`
  `[--lang <language>] [--tags] [--out <folder>] [--conflict …]` writes Records as Record XML
  ([docs/record-xml.md](docs/record-xml.md)) or Record JSON
  ([docs/record-json.md](docs/record-json.md)), one file per Record: `<record path>.xml`, or
  `<record path>/<Record name>.xml` for Records that share a record path (`.json` for JSON).
  `--format ndjson` writes one `records.ndjson` with one Record per line in record-path order.
  `--filter` selects Records by record path with the rules of `search`; `--out` and `--conflict`
  work as for `extract`. A value that cannot be read is written as an error at its field and the
  Record is still written; the summary counts these error values. The same Record always gives
  the same bytes. `--combined` writes every selected Record into one file in record-path order,
  named after the DataForge Entry (`Game2.xml`), under one `p4kx:records` root element.
  `--lang` writes the text of every localisation key next to the key, or marks the key missing,
  in the chosen language; without it the output carries no texts. `--tags` writes the tag path
  (`Ship.Series.Avenger`) of the Tag next to every Reference to a Tag; without it the output carries
  only GUIDs.
- `p4kx dcb tags <archive> [<tag>] [--tree] [--format table|json]` prints the tag tree of the
  `TagDatabase`: every Tag with its tag path, GUID and legacy ID in tree order, or with `--tree` the
  names indented below their parent. `<tag>` is a tag path or GUID and limits the listing to that Tag
  and everything below it; with `--json` the document has the shape described in
  [docs/cli-json.md](docs/cli-json.md). The Tags tab of the window shows the same tree.
- `p4kx dcb query <archive> "<query>"` runs a Query ([docs/query.md](docs/query.md)) over the
  Records and prints a table: the record path first, then one column per path of `select`, one
  row per Record in record-path order, e.g.
  `p4kx dcb query Data.p4k 'from EntityClassDefinition at "*/spaceships/*"'`. `from <type>`
  selects a type and every type deriving from it, `at "<glob>"` selects by record path with the
  rules of `search`. An invalid Query exits with code 2 and marks its position. `--format csv`
  writes RFC 4180 CSV with a header row and `--format json` (or `--json`) one line of JSON, an
  array of objects keyed by column with typed values; `--out <file>` writes the result to a file
  instead of stdout and prints a summary.
- `p4kx entity loadout <archive> <entity>` prints the default Loadout tree of an Entity named by
  class name (`AEGS_Avenger_Titan`), GUID or record path: every port with the Entity attached to
  it, its size and helper, and recursively the ports of each attached Entity. Ports that cannot be
  resolved, such as a dangling Reference, are listed with the reason. For a ship it also reads the
  vehicle implementation XML with the Modification its Record selects: ports the implementation
  hides are marked `(invisible)`, and the treads, wheels and landing gear are listed. With
  `--json` the tree has the shape described in [docs/cli-json.md](docs/cli-json.md). A docked
  vehicle, such as the Merlin in a Constellation's docking collar, is listed as
  `[docked vehicle]`; `--stop-at-docked-vehicles` leaves out its own ports. `--exclude <port>`
  drops a branch of the Loadout and `--replace <port>=<entity>` attaches another Entity, with its
  own Loadout, to a port; a port is its path of port names joined by `/`, and an Entity that does
  not fit the port's item types and sizes is rejected with the reason. Both can be given several
  times, here and for `entity glb`.
- `p4kx entity glb <archive> <entity> --out <file>` exports an Entity as one GLB: its geometry and
  that of every Entity of its default Loadout, each placed at its port as a node named
  `<port>/<entity>` below the node of the Entity it is attached to. `--lod <n>` applies to every
  model and `--materials colours` writes plain colours, with the material library an Entity's
  Record names replacing the one its model names; `--materials textures` also embeds the
  textures of every part as PNG, each image once, at the mip `--texture-mip <n>` picks for the
  whole Assembly. The tint palette of each part (its Entity's, else the default of its material
  library) gives the colours of the materials that take one, so liveries look right. Parts
  whose Entity or geometry cannot be resolved are skipped and listed, and so are palettes and
  textures that cannot be resolved; with `--json` the report has the shape described in
  [docs/cli-json.md](docs/cli-json.md). A docked vehicle such as a Merlin or an Archimedes is
  placed as one part, without the parts of its own Loadout. `--interior` adds the ship's rooms:
  the socpak containers its Record names, each at its bone of the hull, below an `Interior` node
  of the Entity; what cannot be resolved is listed, and without the flag only the hull is written.
- `p4kx entity package <archive> <entity> --out <folder>` writes an Entity as a Blender package,
  a folder that does not depend on Blender's file format: `scene.json` (format version, source
  Channel and Build, the node hierarchy with names and transforms), one GLB per distinct mesh
  and the textures of the materials as PNG, each written once. `--lod`, `--texture-mip`,
  `--exclude` and `--replace` work as for `entity glb`. The folder is written beside the target
  and moved into place when complete; it must not exist or must hold an earlier package. The
  format is described in [docs/blender-package.md](docs/blender-package.md), the JSON Schema of
  `scene.json` is [schemas/blender-package.schema.json](schemas/blender-package.schema.json).
- `p4kx character package <archive> --out <folder>` writes a Character as a Blender package: the
  head of a CHF file (`--chf`), armour and clothing (`--piece`, `--loadout`) on the default body
  (`--body`), as one skinned GLB with the skeleton, the head's expressions as morph targets and
  the animations (`--anim`, `--anim-from`), listed in `scene.json`. The Blender add-on imports it
  as an armature with shape keys and actions.
- `p4kx interior glb <archive> <container>... --out <file>` exports the Interior of a socpak
  container, a ship's rooms or a room on its own, as one GLB: a node per container, per streamed
  `.soc` file and per placed model, containers nested in it at their position and rotation.
  `--lod <n>`, `--materials colours` and `--no-vertex-colours` apply to every model. Containers nested in themselves,
  and containers, files and models that cannot be resolved, are listed; with `--json` the report
  has the shape described in [docs/cli-json.md](docs/cli-json.md).
- `p4kx armour glb <archive> --piece <entity> --body male|female --out <file>` exports pieces of
  armour and clothing worn on the default body of a variant as one skinned GLB: the body's
  skeleton once as bone nodes, one skin over them, the skin of the body and, for every `--piece`
  (repeatable; class name, GUID or record path), the geometry its Record names for the variant's
  tag, with its bones matched to the skeleton by name. `--loadout <loadout>` takes the pieces from
  a Loadout instead: a file on disk of that name is tried first and read as an XML Loadout,
  otherwise an Entity's default Loadout by class name, GUID or record path, or an XML Loadout
  (`Scripts/Loadouts/…xml`) by its path in the Archive; its armour and clothing (`Char_Armor_*`,
  `Char_Clothing_*`) are worn. A `--piece` then takes the place of the Loadout's pieces of the
  same item type, or is added. A piece with no geometry for the variant, one that cannot
  be resolved, or whose model is missing or unreadable, is skipped and listed; with `--json` the report has
  the shape described in [docs/cli-json.md](docs/cli-json.md). `p4kx armour bodies` lists the
  variants.
- `p4kx wwise resolve <archive> <trigger>` lists every sound variant of an audio trigger with
  its soundbank, WEM ID, source and container path, e.g. `Play_QDrive_Spool in SSQD_QDrive.bnk:
  WEM 98393519 embedded, not located; event 2178236557 > action 98720655 > blend 228967944 > …`;
  broken links are listed with the object where the chain breaks, and `--json` gives the shape in
  [docs/cli-json.md](docs/cli-json.md). A streamed WEM is located as its loose WEM Entry, and
  localised sounds are resolved for `--lang <language>` with `--fallback-lang` (`English(US)` by
  default), e.g. `WEM 8000 streamed localised German, at Data\Sounds\wwise\Media\German\8000.wem`.
- `p4kx wwise events <archive> [--search <text>]` searches the audio triggers and the Wwise events
  they post by name (a substring or a glob such as `Play_*_Spool`, ignoring case) and lists each
  trigger with its events, their soundbanks and the Entities that name it, in definition order;
  `--limit` and `--offset` page through large results, `--entities` bounds the Entities listed
  per trigger, `--no-entities` skips the DataForge, and `--json` gives the shape in
  [docs/cli-json.md](docs/cli-json.md).
- `p4kx entity audio <archive> <entity>` lists the audio triggers an Entity names, each with the
  Wwise events of its ATL definition and the soundbanks holding them, e.g.
  `Play_QDrive_Spool: Play_QDrive_Spool in SSQD_QDrive.bnk`; triggers that reach no soundbank are
  listed with the reason, and `--json` gives the shape in [docs/cli-json.md](docs/cli-json.md);
  `--sounds` lists the sounds of every trigger with their source and location, in `--lang`.
- `p4kx wwise extract <archive> <trigger> --out <folder>` writes the sounds of an audio trigger as
  `<trigger>[_<WEM ID>].ogg|wav` (`--format ogg|wav|raw`, `--conflict skip|overwrite|if-changed`,
  `--lang`); a sound that cannot be decoded is reported and written as `.wem`.
  `p4kx entity audio --extract <folder>` does the same for every trigger of an Entity.
- `p4kx loc languages <archive>` lists the languages the Archive holds a `global.ini` for, with
  their codes (`en`, `de`, `fr`, `it`, `es-ES`, `es-419`, `pt-BR`, `ja`, `ko`, `zh-Hans`,
  `zh-Hant`); `p4kx loc get <archive> <key> [--lang <language>] [--fallback <language>]` prints the
  text of a localisation key, with or without the leading `@`, ignoring case, in `--lang` (English
  by default) and otherwise in `--fallback`, with `\n` decoded; a key with no text in either is
  reported as missing and exits with code 2, a placeholder key such as `@LOC_EMPTY` prints
  nothing and succeeds. `p4kx loc export <archive> [--lang <language>,…] [--format json|csv]
  [--out <file>]` writes every key with its text per language, sorted by key. A `--lang` is a
  code, a folder name such as `german_(germany)` or a name such as `german` where one language has
  it. UTF-8 with or without a byte order mark and UTF-16 files are read; invalid bytes become
  U+FFFD and lines without `=` are skipped, and a `note:` line on stderr names each such Entry
  (also for `catalog`). `loc export --format csv` writes a UTF-8 byte order mark and CRLF line
  ends. `--fallback en` is the usual choice; there is no default. `--json` gives the shapes in [docs/cli-json.md](docs/cli-json.md).
- `p4kx catalog <archive> --out <file> [--filter <filter>] [--exported-at <seconds>]` writes the
  Basetool catalog for the basetool's P4K import ([docs/basetool-catalog.md](docs/basetool-catalog.md)):
  schema version, Channel and Build, completeness per kind and the manufacturers, items (with
  their item class) and ships with ground vehicles, ordered by GUID, with test, template, dummy,
  debug, deprecated and dev Records left out and counted as skipped, and names and descriptions
  resolved in English and German. `--filter` marks the catalog partial. It prints the counts per
  kind, the item classes read and the unresolved texts per language and warns when the catalog
  nears or exceeds the 64 MB upload limit. An `--out` name ending in `.zst` writes the catalog as
  one zstd frame (level 19, content size declared, plain `zstd -d` decodes it), a fraction of the size, which the Basetool accepts as
  it does plain JSON. A selected Record that cannot be read is left out,
  listed on stderr with its GUID and reason and counted as failed; the catalog is still written
  and the exit code is 1. `p4kx catalog schema` prints the catalog's JSON Schema.
- `p4kx ships <archive> --out <file> [--format json|ndjson] [--filter <filter>]
  [--exported-at <seconds>]` writes the base data of every ship
  ([docs/ship-base-data.md](docs/ship-base-data.md)): hull mass and hit points summed from the
  vehicle implementation XML with its Modification applied, the cargo capacity of the cargo grids
  of the default Loadout, `AttachDef.Size` and the Loadout as class names per port path, each
  value with the file and field it was read from. Docked vehicles are marked and not counted. A
  ship that cannot be read is listed on stderr and the exit code is 1.
- `p4kx channels` lists the installed Channels with Build version, source and Archive path. It
  looks below `C:\Program Files\Roberts Space Industries\StarCitizen`, below the library folders
  named in the RSI Launcher log (`%APPDATA%\rsilauncher\logs`, see
  [ADR-0005](docs/adr/0005-library-folders-from-the-launcher-log.md)) and in manually added
  folders and Archives. `p4kx channels add <path>` and `p4kx channels remove <path>` manage the
  manual entries in `settings.toml` in `%APPDATA%\P4KX` (the platform config directory
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
- `p4kx compare <old> <new> --schema` compares the schema of both DataForges instead of their
  contents: the structs, properties (data type, conversion type, target), enums and enum options
  added, removed or changed, each struct and enum with the number of Records it concerns on both
  sides. `--json` prints the report (see
  [docs/cli-json.md](docs/cli-json.md#p4kx-compare-old-new---schema---json)); the Comparison tab
  of the window shows it in the Schema list once the Record level was compared. The exit code is 0
  without and 1 with differences.
- `p4kx compare <old> <new> --diff <entry>` prints the line diff of one Entry between both Archives
  as a unified diff: CryXml is converted to XML first, UTF-16 with a byte order mark is decoded,
  other text and text XML are compared byte for byte, and an Entry on one side only is diffed
  against an empty side. A binary Entry, or one larger than 128 MiB or 4 194 304 lines, gets a note
  saying whether it changed. `--json` prints the hunks as JSON (see
  [docs/cli-json.md](docs/cli-json.md#p4kx-compare-old-new---diff-entry---json)). The exit code is
  0 when the Entry is unchanged and 1 when it was added, removed or changed, also when its lines are
  equal after conversion.
- `p4kx audio info <archive> <entry>` prints the codec, channels, sample rate and duration of a
  Wwise WEM sound; `p4kx audio wav <archive> <entry> --out <file>` writes a PCM, IMA ADPCM,
  PTADPCM, Wwise Vorbis or Opus WEM as a 16-bit WAV file, and
  `p4kx audio ogg <archive> <entry> --out <file>` rebuilds a Wwise Vorbis WEM as a standard Ogg
  Vorbis file and an Opus WEM as a standard Ogg Opus file, both without re-encoding. All three
  also take a WEM file on disk instead of an Archive
  and an Entry. PCM, IMA ADPCM, Wwise Vorbis, Opus, PTADPCM and the console codecs are
  detected; a conversion the codec does not support yet fails with an error naming it, and a
  corrupt or truncated WEM fails with the reason.
- `p4kx bnk list <archive> <entry>` prints the ID, codec and duration of every WEM a Wwise BNK
  soundbank embeds; `p4kx bnk extract <archive> <entry> --out <folder>` writes them as
  `<id>.wem`, with `--format wav` converted to WAV, or with `--format ogg` rebuilt as Ogg Vorbis or Ogg
  Opus.
  Both also take a soundbank file on disk. A corrupt soundbank fails with an error naming the
  section.
- `p4kx ui assets <archive> <entry>` lists what a GFx or SWF UI movie holds: its header, every
  tag code with its count (unknown tags kept and counted), and the shapes, morph shapes, bitmaps
  (lossless, JPEG, PNG, GFx external and sub-images), fonts and sprites with their IDs and export
  names; a tag that cannot be read is listed instead of failing the movie, with exit code 1.
  `p4kx ui extract <archive> <entry> --out <folder>` writes every bitmap as `bitmap_<id>.png`
  and every shape as `shape_<id>.json` (morph shapes as `morph_shape_<id>.json`) in twips as the
  movie stores them; an external image is decoded from the DDS texture it names next to the
  movie (on disk only inside the movie's `Data` folder), a sub-image is cut from its bitmap.
  Both also take a movie file on disk, compressed (`CWS`, `CFX`, `ZWS`) or not (`FWS`,
  `GFX`).
- `p4kx ui render <archive> <entry> --out <file.png> [--size <W>x<H>|<W>] [--background
  transparent|movie|#RRGGBB[AA]]` renders the first frame of a UI movie, with its sprites, as a
  PNG: shapes with solid, gradient and bitmap fills (external images included) and strokes, with
  the transforms, colour transforms and masks of their placements, and morph shapes at their
  ratio. `--size` with both sides fits the stage centred into them, with one side keeps its
  aspect ratio; the default is the stage size. Static texts and text fields are drawn from the glyph
  outlines of the fonts of the movie, with their size, colour and alignment; a character whose font
  or glyph is missing is drawn as an empty box and listed, since no system font is used. Buttons and
  videos are skipped, filters and blend modes are not applied, and every element left out is listed
  (exit code 1). A movie is
  untrusted: placements, paths, conversions, masks, the canvas and the area the fills cover are
  bounded, and what lies beyond a limit is listed instead of drawn.
- `p4kx ui render … --lang <language>` resolves the localisation keys (`@key`, with `,P=`
  parameters) of text fields in the `global.ini` of that language of the Archive; it needs an ENTRY.
  A key the language does not hold is drawn as it is and listed.
- `p4kx ui render … --svg` writes the frame as an SVG document instead, which scales without loss:
  shapes are written once and placed with their transforms, gradients, strokes, colour transforms
  and masks become SVG elements, and bitmaps are embedded as PNG data. The same input gives the
  same bytes, `--size` and `--background` set the size and the backdrop of the document, and the
  same limits end the frame at the same element as for the PNG. A bitmap fill that does not repeat
  is drawn once without continuing its edge pixels, and a stroke painted with a non-repeating
  bitmap tiles it. Text is written as paths, never as `<text>` elements. A document is at most 1 GiB;
  a larger one is an error and no file is left.
- `p4kx ui hologram <archive> <entity> --out <file.png> [--size <W>x<H>|<side>] [--background
  <colour>] [--view-angle <degrees>] [--elevation <degrees>] [--fov <degrees>] [--distance-scaler
  <factor>] [--tint <colour>] [--opacity <0..1>] [--glow <0..4>] [--lod <n>] [--software]
  [--exclude <port>] [--replace <port>=<entity>]` draws the hologram of a vehicle: the Assembly of
  the Entity with its Loadout, as a transparent tinted shell that glows along its edges, seen
  through the narrow lens of the game's `UIHoloVehicle_Config` Record (field of view, view angle
  and distance factor, 10, 60 and 1.5 in the current game, each overridable). The picture is
  drawn offscreen without a window, by the same wgpu renderer as the 3D viewer, on the graphics
  card or with `--software` on the software adapter (WARP, lavapipe), which gives a picture that
  does not depend on the graphics card. The background is `transparent` or `#RRGGBB[AA]`; the PNG
  has straight alpha. Textures and interiors are not drawn.
- `p4kx ui screens <archive> <entity>` lists the `BuildingBlocks` screens of an Entity and of
  everything in its default Loadout, e.g. the MFDs, the radar and the HUD of a ship: per screen the
  port, the Entity, the canvas Record and its size, the canvases, textures, vector graphics, movies
  and localisation keys it uses, and its widgets by element type with whether they are drawn; the
  UI movies the Entities name are listed after them. `--json` carries the same.
- `p4kx ui render <archive> --screen <canvas> --out <file.png> [--svg] [--size …] [--background …]
  [--brand <code>]` renders such a canvas, named by record path or GUID, as a still through the
  GFx renderer: the layout of its widgets (sizes, margins, padding, flex containers, raw units)
  is computed from the Records and the boxes, circles and polygons are drawn in the colours of
  the style cascade (palette slots of the default and manufacturer style, style entries with type,
  tag, parent and ancestor selectors at rest). Texts, textures, vector graphics, lists, clones and
  every other widget type without a renderer are left out and listed, with the features the layout
  approximates (exit code 1); no ActionScript is run.
- `p4kx chf to-json <file> [--out <file>]` converts a CHF character file, e.g. one saved by the
  character customiser, to indented JSON on stdout or in `--out`: its inner versions (7 and 8
  are read), body type, face DNA with every face part named, item ports and material choices.
  `p4kx chf from-json <file> --out <file>` writes the JSON back as a CHF file: unchanged, it
  gives the original file byte for byte; edited, a new file with a correct CRC-32C. The tool never
  writes into a Star Citizen installation, and that includes the `user` folder of a Channel, so
  `--out` cannot be `LIVE\user\client\0\CustomCharacters` (exit code 2): write the file elsewhere
  and copy it in by hand. A file with
  a wrong size, magic or CRC-32C, an unknown version or a corrupt body fails with the reason
  (exit code 3), as does invalid or unwritable JSON (exit code 2). The JSON is described in
  [docs/cli-json.md](docs/cli-json.md#the-chf-json).
- `p4kx chf glb <archive> <file> --out <file>` exports the head of a CHF character as one skinned
  GLB: the head skin of the character's head rig, its vertices moved by the character's face DNA
  into the character's neutral face (the neutral morphs of the DNA rig beside the skin, mixed by
  the DNA's blends: per face part up to the four strongest heads), worn on the default body of the
  character's body variant. The teeth and what the character wears on its head are included: the
  eyes, hair, brows, lashes and beard, with the materials the character chose (its head material,
  skin tone, iris colour, hair melanin and dye) and their textures (`--materials colours` leaves
  the textures out, `--texture-mip N` picks their size). The head rig's expressions (about 400 of
  its 495 blendshapes, e.g. `CTL_expressions blinkR`) are written as named morph targets of the
  head, mixed by the DNA so that they move the character's own face and stored sparse;
  `--expression NAME` (repeatable) limits them, `--no-expressions` leaves them out. Only LOD 0 has
  expressions, and hair, eyes and teeth follow only the neutral shape of the head. `--lod N` picks
  the level of detail of head and body, `--no-vertex-colours` leaves the vertex colours out. A head
  that cannot be shaped (its rig is missing, belongs to another rig or does not match the skin) is
  listed and left out of the GLB, which is still written, and the exit code is 1; an item or a
  material the Archive does not have is listed, and a character without face DNA exits with 2. The
  report is described in [docs/cli-json.md](docs/cli-json.md).
- `p4kx socpak list <archive> <container…>` lists the Entries of a socpak container in the
  columns of `list`, read from the Archive without extracting the container first; each further
  path names a container inside the one before. `p4kx socpak extract <archive> <container…>
  [--filter <filter>] [--out <folder>] [--conflict <policy>] [--convert]` writes its Entries, or
  those the Filter matches, with the rules of `extract`; `if-changed` compares with the CRC-32
  containers store. Containers nest at most 8 deep; a container whose Entries overlap, whose
  stored sizes disagree or which declares more than 2048 times its stored size for an Entry is
  refused, as are compressed containers that together decode to more than 512 MiB along one chain. A container that cannot be
  opened fails with exit code 3 and an error naming its Entry.
- `p4kx chunks list <archive> <entry>` prints the chunk table of a CryEngine chunk file (`.cgf`,
  `.cga`, `.skin`, `.chr`, their geometry companions, `.caf`, `.dba`) in the `#ivo` or `CrCh`
  container: index, type, type ID, version, chunk ID, byte order, offset and size, with unknown
  types marked; `p4kx chunks dump <archive> <entry> --chunk <index> --out <file>` writes one
  chunk's bytes unchanged. Both also take a chunk file on disk.
- `p4kx model info <archive> <entry>` shows an `#ivo` model (`.cgf` or `.skin` with its geometry
  companion `.cgfm` or `.skinm`, or a skeleton file `.chr`): its nodes with their hierarchy, its
  LODs and meshes with vertex and triangle counts, the material library and material IDs, its bones
  with parent and bind position, and every chunk with whether it was read, skipped or unknown, and
  the materials of its `.mtl` file;
  `p4kx model glb <archive> <entry> --out <file> [--lod <n>] [--materials colours|textures]
  [--texture-mip <n>]` writes it as one glTF 2.0 binary file (GLB), Y up and in metres, with the
  normals, tangents and vertex
  colours the meshes have (`--no-vertex-colours` leaves the colours out), and lists the unknown
  chunks it skipped. `--lod` picks the level of detail, 0 by default; the LODs come from the
  companions `<name>_lod<n>.<extension>m`, and a level the model lacks exits with code 2.
  `--materials colours` (the default) gives every sub-material of the model's `.mtl` file a glTF
  material with base colour, opacity, roughness, metalness and emissive colour and no textures;
  submeshes whose material draws nothing (collision and helper geometry) are left out, and a
  missing or unreadable `.mtl` file is reported while the model exports with a grey default
  material. `--materials textures` also embeds the textures of the materials as PNG images: the
  diffuse map as base colour, the normal map (green flipped to glTF's up, the blue channel
  reconstructed), a metallic-roughness image from the normal map's smoothness and the specular
  map, shared images once; `--texture-mip` picks the mip level, 0 (the largest) by default. A
  texture that is missing or cannot be decoded leaves the plain colour and is listed. A skin
  (`.skin`) is written with its skeleton and bone weights as a glTF skin (up to eight bones per
  vertex, `JOINTS_0`/`WEIGHTS_0` and `_1`), a `.chr` skeleton file as bone nodes. Both also take a
  model file on disk with its companions next to it, which has no textures to read.
- Encrypted Entries need the Archive key, which you enter yourself; the tool does not contain it
  ([ADR-0027](docs/adr/0027-archive-key-comes-from-the-user.md)). Without it an Archive still
  opens, lists, searches and compares, and reads every unencrypted Entry; an encrypted Entry fails
  with a message that says the key is missing, or that it does not fit. `--key <hex>` (32
  hexadecimal digits) or the environment variable `P4KX_ARCHIVE_KEY` gives it for one run, with
  the option before the variable before the stored key. `p4kx key set [<hex>] [--archive <archive>]`
  stores it in your profile (`%APPDATA%\P4KX\archive.key` on Windows, the platform
  config directory elsewhere, readable by your user alone where the platform allows it), after
  checking it against an encrypted Entry of the Archive, an installed Channel when none is named;
  without `<hex>` it reads the first line of stdin, which keeps the key out of the process list
  and the shell history. `p4kx key show` shows the stored key and the one in use with all but
  the last four digits hidden, `p4kx key remove` deletes the stored key. The key never appears in
  logs, error messages or JSON. `--key-file` (or `P4KX_KEY_FILE`) names another file for the stored
  key, for the command line, the window and the MCP server alike.
- `--json` on any command prints its result as one line of JSON instead of text; the shapes are
  documented in [docs/cli-json.md](docs/cli-json.md).

### Exit codes

| Code | Meaning |
|------|---------|
| 0 | Success: every requested Entry was extracted or already existed; `compare`: no differences |
| 1 | Partial failure: some Entries or Records failed or were cancelled, the data of an Entry does not match its checksum (`verify`, `extract --verify`), the target or the settings cannot be written, a texture cannot be read, merged or written, a tag or definition of a UI movie cannot be read or written, or `ui render` leaves an element out (skipped, drawn without an effect, or beyond a limit; for `--screen` also a widget type without a renderer or an approximated layout), or `chf glb` or `character package` could not shape the head by its DNA (the output is written without the head); `compare`: the Archives or the `--diff` Entry differ |
| 2 | Invalid input: bad arguments, an Entry path the Archive does not contain (for `compare --diff`: neither Archive contains), a Filter that matches no Entry or Record, a Channel path that does not exist or was never added, a texture or UI movie file that does not exist, a merge output that is one of its inputs, a target or output inside a Star Citizen installation (the tool never writes there), a `ui render` canvas that is empty or larger than 16384 pixels a side or 2^25 pixels, `--json` without `--out` for `cryxml` or `chf to-json`, or CHF JSON that is invalid, has an unknown version or cannot be stored as CHF, a CHF file for `chf glb` or `character package` that holds no face DNA or whose DNA belongs to an unknown head rig, `loc` for a language the Archive holds no `global.ini` for or a key with no text in the language or its fallback, and `loc export` of an Archive without any `global.ini`, a canvas for `ui render --screen` that no Record, several Records or a non-canvas Record names, or whose side is below 1 or above 16384 pixels |
| 3 | The Archive, a socpak container in it or the input file cannot be read: missing, not an Archive, not CryXml, CHF or a UI movie, corrupt or with a CHF CRC-32C mismatch or unknown CHF version, also a `compare --diff` Entry that cannot be read, does not hold its stated size or holds corrupt CryXml, and a `global.ini` of `catalog` or `loc` that is over 64 MiB, holds more than 1,048,576 keys or cannot be read (invalid bytes and lines without `=` are no error); a missing texture or UI movie file is 2 |
| 4 | The Archive changed on disk while it was open, e.g. updated by the RSI Launcher; the Extraction stopped, and Entries written before may be stale |
| 5 | The DataForge cannot be read: the Archive holds none, its version is not supported or it is corrupt |
| 6 | `compare`: the report or the output cannot be written |
| 7 | The Archive key is missing or wrong: an Entry is encrypted and no key was given with `--key`, `P4KX_ARCHIVE_KEY` or `p4kx key set`, or the key does not decrypt the Archive |
| 130 | Interrupted: a second Ctrl+C ended an Extraction at once; temporary `.p4kx-*.part` files may remain |

## MCP server

`p4kx-mcp` is a read-only [Model Context Protocol](https://modelcontextprotocol.io) server over
stdio, so an AI client can look into the Archives. It is built with the official Rust MCP SDK
(`rmcp`) and speaks every protocol version the SDK negotiates, up to `2026-07-28`. Its tools never
write a file.

What the tools return is the content of the Star Citizen Archive. The AI client may forward it to
third parties, for example the cloud model provider behind the client. Game texts and media must
not be redistributed. You remain responsible for complying with the Star Citizen EULA and Terms of
Service.

| Tool | Result |
|------|--------|
| `list_channels` | The detected Channels with Archive path, source and Build, found like `p4kx channels` |
| `archive_info` | Channel, Build, number of Entries and encrypted Entries, sizes, `issue_count` and the first 50 `issues` (Entry path and defect) of the directory |
| `list_entries` | One folder of an Archive (`folder`, e.g. `Data/Libs`): its subfolders with their Entry count, then its Entries |
| `search_entries` | The Entries matching a Filter (`filter`), with the rules of `p4kx search` |
| `read_entry_text` | A range of an Entry as text (`path`, `offset`, `max_bytes`): CryXml as XML, UTF-16 by its byte order mark, otherwise UTF-8 |
| `read_entry_bytes` | A range of an Entry's data as base64 (`path`, `offset`, `max_bytes`) |
| `get_record` | A Record by `guid`, or every Record of a record `path`, as Record JSON |
| `search_records` | The Records whose record `path` and `type` name match Filters, in Record order |
| `list_structs` | The struct definitions matching a `name` Filter, with parent and declared fields (data type, array, target struct or enum) |
| `list_enums` | The enum definitions matching a `name` Filter, with their values |
| `run_query` | One page of a Query result (`query`): typed columns, rows as `p4kx dcb query --format json` writes them, failed Records, unresolved References |
| `list_chunks` | The chunks of a chunk file Entry (`path`) as `p4kx chunks list` shows them: index, type, type ID, known, version, chunk ID, byte order, offset and size; `type` (a name or `0x` ID) narrows it. No chunk data is returned |
| `get_model_info` | A model Entry (`path`) summarised as `p4kx model info --json`: geometry, material library and IDs, levels of detail, and the number of materials, nodes, meshes, bones, animations and chunks |
| `list_model_items` | One `section` of a model Entry, paged: `materials`, `nodes`, `meshes`, `bones`, `animations`, `unread_animations` or `chunks`, each item shaped as in `p4kx model info --json` |

Every tool except `list_channels` takes `archive`: a Channel name such as `LIVE` (ignoring case)
or the absolute path of a `.p4k` file on a local drive. UNC, device, relative and `..` paths, a
`:` after the drive (an alternate data stream) and a path that resolves to a network drive or
device are refused. Without it the tools use the Archive given with `--archive` when the server
was started. Opened Archives stay open between calls and are opened again when their file
changes. Listings page: `limit` sets the page size (100 by default, at most 1000), and a result
with `next_cursor` has a next page that `cursor` asks for; a cursor only continues the listing
that returned it, on the same version of the Archive. The DataForge tools read the Archive's DataForge once, keep it
open with its indexes while the Archive stays open, and hold at most 256 KiB of items in one result:
a larger page ends early with `next_cursor`, and a single Record or row larger than that is an
error. `list_entries` lists folders as
[ADR-0023](docs/adr/0023-entry-folders-fold-case-and-separators.md) defines them. `--config-dir`, `--launcher-dir` and
`--default-library` (or `P4KX_CONFIG_DIR`, `P4KX_LAUNCHER_DIR`, `P4KX_DEFAULT_LIBRARY`) work as
for `p4kx channels`.

The server reads encrypted Entries with the Archive key from its `--key <hex>` option, the
`P4KX_ARCHIVE_KEY` variable of its configuration or the key stored with `p4kx key set` or in the
window, in that order; the stored key is read on each call, so one stored while the server runs
applies to the next call. A tool that reads an encrypted Entry without a key fails with an error
that says how to give it. `--key-file` (or `P4KX_KEY_FILE`) names another file for the stored key.

The MSI installs `p4kx-mcp.exe` next to `p4kx.exe` in
`%LOCALAPPDATA%\Programs\P4KX`, and the portable ZIP holds it too. Add it to the AI
client with its full path. Claude Code:

```powershell
claude mcp add p4kx -- "$env:LOCALAPPDATA\Programs\P4KX\p4kx-mcp.exe" --archive LIVE
```

Claude Desktop and other clients that read an `mcpServers` file, with `<user>` your Windows user
name:

```json
{
  "mcpServers": {
    "p4kx": {
      "command": "C:\\Users\\<user>\\AppData\\Local\\Programs\\P4KX\\p4kx-mcp.exe",
      "args": ["--archive", "LIVE"]
    }
  }
}
```

## Build

Requires the stable Rust toolchain (1.99 or newer).

```powershell
cargo build --release
.\target\release\p4kx.exe list "C:\Program Files\Roberts Space Industries\StarCitizen\LIVE\Data.p4k"
```

## Blender add-on

Every GitHub Release attaches `p4kx_importer-<version>.zip` next to the MSI and the portable ZIP.
In Blender 5.2 LTS: Edit > Preferences > Get Extensions > Install from Disk, pick the ZIP, then
import a [Blender package](docs/blender-package.md) with File > Import > P4KX Package
(scene.json). The add-on version always equals the Release version: `cargo xtask` (part of CI)
fails when `blender_addon/p4kx_importer/blender_manifest.toml` differs from the workspace
version, so bump both together when preparing a Release. The ZIP is reproducible; see
[blender_addon/README.md](blender_addon/README.md) to build it yourself.

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

The licence texts of the Rust crates in `THIRD-PARTY-NOTICES` are generated by
[cargo-about](https://github.com/EmbarkStudios/cargo-about) from `about.toml` and `about.hbs`.
After a dependency change run `cargo xtask notices --write`; `cargo xtask notices` checks the file
and runs in CI. Both need `cargo install cargo-about --version 0.9.2 --locked` and fail on any other version.

Tests use synthetic Archives from `p4kx-testkit`; no game files are needed or committed.

### Real-Archive suite

A second suite checks a real `Data.p4k`: opening and listing it within 10 seconds, its Build,
reading Entries of every compression method and encryption state present (size and CRC-32C),
extracting a small subset, and sorting and scrolling every Entry in the window within one 60 Hz
frame. It compares two Archives on both levels with plausible counts and a report that is valid
against `schemas/comparison-report.schema.json`, the Entries in less time than reading both
Archives at 8 GB/s would take (about 1.5 seconds against 37 for LIVE and PTU), produces the line
diffs of changed text Entries, and compares one Archive with itself without a difference on
either level. Its tests are ignored by default, so `cargo test` and CI report them as `ignored`.
Point `P4KX_DATA_P4K` to an Archive, and `P4KX_DATA_P4K_B` to a second one for the Comparison,
and pass `--ignored`; without `P4KX_DATA_P4K` the suite fails and says so, without
`P4KX_DATA_P4K_B` the tests over two Archives pass without checking anything. The suite also
needs the Archive key in `P4KX_ARCHIVE_KEY` and fails with a message when it is missing.

```powershell
$env:P4KX_DATA_P4K = 'C:\Program Files\Roberts Space Industries\StarCitizen\LIVE\Data.p4k'
$env:P4KX_DATA_P4K_B = 'C:\Program Files\Roberts Space Industries\StarCitizen\PTU\Data.p4k'
$env:P4KX_ARCHIVE_KEY = '<the Archive key as 32 hexadecimal digits>'
cargo test --release --workspace --locked -- --ignored
```

The Archive is only read. The extracted subset goes to a temporary folder below `TMP`, which is
removed afterwards.

## Licence

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT licence](LICENSE-MIT) at your option.

The Vorbis codebook libraries embedded in `p4kx-audio` come from
[ww2ogg](https://github.com/hcs64/ww2ogg), and the Opus decoder
[opus-pure](https://github.com/stephenberry/opus-pure) is a port of libopus; both are licensed
under the BSD 3-Clause licence; see [THIRD-PARTY-NOTICES](THIRD-PARTY-NOTICES), which ships with
every Release.
