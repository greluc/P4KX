# JSON output of `p4kx`

Every `p4kx` command takes `--json` (before or after the subcommand). With it, the command writes
exactly one line of compact JSON to stdout and nothing else; the exit codes stay the same. Errors
that end the command (an unreadable Archive, invalid input) are still printed to stderr as
`error: …` and produce no JSON. Warnings, such as an unreadable build manifest, go to stderr.

Paths of files on disk are strings as the operating system displays them. Entry paths are given
as stored in the Archive, with `\` as separator. Sizes are byte counts. Snapshots of every shape
live in [`crates/p4kx/tests/snapshots`](../crates/p4kx/tests/snapshots).

## Entry

The shape of one Entry in `list` and `search`:

| Field | Type | Meaning |
|-------|------|---------|
| `path` | string | Entry path in the Archive |
| `compression` | string | `store`, `deflate`, `zstd`, or `method-<n>` for an unsupported ZIP method id |
| `encrypted` | bool | Whether the data is encrypted |
| `compressed_size` | number | Size of the data in the Archive |
| `size` | number | Size after decompression |

## Build

The shape of a Build in `info` and `channels`; `null` where the Build is unknown.

| Field | Type | Meaning |
|-------|------|---------|
| `branch` | string | e.g. `sc-alpha-4.11.0` |
| `version` | string | e.g. `4.11.42.1234` |
| `change_number` | string | The requested Perforce change number |
| `build_id` | string | The BuildId |

## `p4kx list <archive> --json`

```json
{"archive": "C:\\…\\LIVE\\Data.p4k", "entries": [Entry, …]}
```

Entries are in Archive order.

## `p4kx search <archive> <filter> --json`

```json
{"archive": "C:\\…\\LIVE\\Data.p4k", "filter": "*.xml", "entries": [Entry, …]}
```

`filter` is the Filter as given; `entries` holds the matching Entries in Archive order and is empty
when nothing matches.

## `p4kx info <archive> --json`

```json
{"archive": "…", "channel": "LIVE", "build": Build, "entry_count": 4, "encrypted_count": 1,
 "compressed_size": 35, "size": 118}
```

`channel` is the name of the folder holding the Archive, or `null` when there is none; `build` is
`null` when the Build is unknown.

## `p4kx dcb info <archive> --json`

```json
{"archive": "…", "entry": "Data\\Game2.dcb", "version": 8, "record_count": 117709,
 "struct_count": 6696, "enum_count": 774}
```

`entry` is the path of the DataForge Entry as the Archive stores it.

## `p4kx dcb export <archive> --json`

```json
{"archive": "…", "entry": "Data\\Game2.dcb", "format": "xml", "target": "…", "exported": 3,
 "skipped": ["libs/foundry/records/ships/avenger.xml"],
 "failed": [{"guid": "…", "path": "libs/foundry/records/odd", "error": "…"}], "cancelled": 0,
 "error_values": 1, "records_with_errors": 1, "error": null}
```

`format` is `xml`, `json` or `ndjson`. `exported` counts the Record files written, or for
`ndjson` the lines of `records.ndjson`. `skipped` lists the files, relative to `target` with `/`
as separator, whose existing file was kept under `--conflict`; for `ndjson` it can only list
`records.ndjson`. `failed` lists the Records not written with GUID, record path and the reason
and its causes joined by `: `. Both are in definition order for `xml` and `json`; for `ndjson`,
`failed` lists the Records of another DataForge first, then the others in record-path order.
`error` is the reason and causes why `records.ndjson` could not be written, otherwise `null`.
`error_values` counts the values written as errors (see
[record-xml.md](record-xml.md#child-elements)) in the Records written or kept, and
`records_with_errors` the Records holding at least one. The exit code is 1 when `failed` is not
empty or `error` is set.

With `--combined` the report also holds `"file": "Game2.xml"`, the combined file relative to
`target`, and `exported` counts the Records written into it. When an existing file is kept under
`--conflict`, `skipped` holds its name and `exported` is 0; under `if-changed` the document was
still built, so `error_values` and `records_with_errors` still count its Records. A cancelled
run writes no file and counts every Record not failed in `cancelled`. Without `--combined` the
field is absent.

## `p4kx entity loadout <archive> <entity> --json`

```json
{"archive": "…", "entry": "Data\\Game2.dcb", "entity": Node, "ports": 100, "attached": 71,
 "empty": 29, "unresolved": 0}
```

`<entity>` is a class name such as `AEGS_Avenger_Titan` (also as the Record name
`EntityClassDefinition.AEGS_Avenger_Titan`), a GUID, or a record path, which is recognised by a
`/` or `\`; case does not matter. An Entity that nothing or several Records match exits with code
2, a root Record that cannot be read with code 5. `ports` counts every port in the tree, and
`attached`, `empty` and `unresolved` count them by what they hold. Ports that cannot be resolved
do not change the exit code.

A **Node** is one Entity:

| Field | Type | Meaning |
|-------|------|---------|
| `guid` | string | GUID of the Entity's Record |
| `record_path` | string | Record path without `.xml` |
| `name` | string | Record name, e.g. `EntityClassDefinition.AEGS_Avenger_Titan` |
| `class_name` | string | The Record name after `EntityClassDefinition.` |
| `issues` | array | Errors (without target) that do not stop the Entity: a part of its Loadout that cannot be read, an ambiguous class name, or the port limit |
| `ports` | array | One Port per port, see below for the order |

A **Port**:

| Field | Type | Meaning |
|-------|------|---------|
| `name` | string | The port name as the Loadout or the port definition spells it |
| `source` | string | `loadout` (the Entity's own default Loadout), `override` (the Loadout of the entry that attached the Entity, which replaces the Entity's own entry for this port), `port_default` (the default item of the port definition) or `unset` (a defined port that nothing names) |
| `definition` | object or `null` | The port's `SItemPortDef` in the Entity's Record; `null` when the Record defines no port of this name, e.g. a ship port that only its vehicle definition declares |
| `entity` | Node or `null` | The attached Entity |
| `error` | Error or `null` | Why the attachment cannot be resolved |

`entity` and `error` are both `null` for an empty port. A **definition** has `display_name` (a
localisation key), `flags` (as stored), `min_size` and `max_size` (numbers or `null`), `types`
(each with `type` and `sub_types`) and `transform`: `null` without an attachment helper,
otherwise `attachment` (`Bone`, `Entity`, `Skin` or `StatObj`), `helper` (the helper in the
parent's geometry), `offset_helper`, `position` (`[x, y, z]` in metres) and `rotation`
(`[x, y, z]` in degrees as stored).

An **Error** has `kind`, a stable code, `reason`, text for people, and for an unresolved port the
target: `reference` (the GUID) or `class_name`, the other `null`. The kinds are:

- at a port: `dangling_reference`, `invalid_reference`, `unknown_class`, `not_an_entity`,
  `unreadable_record`, `cycle` (the Entity is already attached further up the branch) and
  `too_deep` (more than 32 attached Entities below the root);
- among the `issues` of an Entity: `too_many_ports` (the tree reached 65,536 ports, so this
  Entity's further ports are left out; the tree never holds more), `ambiguous_class` (several
  Entities have the class name the entry names; the first in definition order is attached and
  the reason lists every candidate), `no_loadout_files` and `xml_loadout` (an XML Loadout or one
  it includes that is missing, corrupt, includes itself or nests deeper than 8 includes; what
  could be read is still listed).

New kinds may be added in later versions; a script should treat an unknown `kind` as an error it
cannot interpret rather than fail.

The tree is resolved from the Records ([ADR-0012](adr/0012-per-port-loadout-override.md)): an
Entity's ports are the entries of its default Loadout
(`SEntityComponentDefaultLoadoutParams`: a manual Loadout's entries, or an XML Loadout read from
the Archive with the Loadouts it includes) in Loadout order, each replaced in place by the entry
of the same port, ignoring case, in the Loadout given by the entry that attached the Entity, with
that Loadout's other entries appended; then the ports its `SItemPortContainerComponentParams`
defines and no Loadout names, in definition order. An entry names its Entity by Reference, or by
class name when it has none; of several Entities with one class name, the first in definition
order is attached. The same Archive always gives the same document.

## `p4kx dcb query <archive> <query> --json`

Without `--out`, the line is the Query result itself in its JSON format, as `--format json`
writes it ([query.md](query.md#json)):

```json
[{"record path":"libs/foundry/records/ships/avenger","mass":25000.5,"size":"Large"},…]
```

One object per row in record-path order, keyed by `record path` and then by each `select` path
in column order, with typed values. `--json` with `--format table` or `--format csv` and
without `--out` exits with code 2, because stdout would not hold JSON.

With `--out <file>`, the result in `--format` (`json` by default) goes to the file and stdout
gets the summary:

```json
{"archive": "…", "format": "csv", "output": "…\\ships.csv", "records": 3, "failed": 0}
```

`records` counts the rows written and `failed` the Records whose layout cannot be read; each of
them is also reported on stderr, and the exit code is 1 when `failed` is not 0.

## `p4kx catalog <archive> --out <file> --json`

```json
{"archive": "…", "out": "…", "schema_version": 1, "size": 1061001, "upload_limit": 64000000,
 "size_warning": null, "kinds": [{"kind": "manufacturers", "count": 1201, "skipped": 2,
 "complete": true}], "skipped": 2, "texts": [{"language": "en", "available": true,
 "resolved": 2240, "unresolved": 8, "placeholder": 154}, {"language": "de", "available": true,
 "resolved": 2132, "unresolved": 116, "placeholder": 154}], "unresolved": 124}
```

The catalog itself goes to `out` ([basetool-catalog.md](basetool-catalog.md)). `size` is its size
in bytes; `size_warning` is `near_limit` from 90 % of `upload_limit`, `over_limit` above it, and
`null` otherwise. `kinds` lists every kind in catalog order with the entities written, the
development leftovers `skipped` and whether the kind is `complete` (no `--filter`); `skipped` sums
them. `texts` lists per language whether the Archive holds its `global.ini` (`available`) and
counts per text occurrence, so a key that several names or descriptions share counts once for
each: `resolved` texts, `unresolved` texts (a key the file lacks or holds empty, or every key in a
language that is not available; `null` in the catalog) and `placeholder` texts, whose key stands
for no text (the same in every language, neither resolved nor unresolved). `unresolved` sums the
unresolved texts of every language.

## `p4kx channels --json`

```json
{"channels": [{"name": "LIVE", "build": Build, "source": "default", "archive": "…"}, …]}
```

`source` is `default` (below the default Library folder), `launcher` (below a Library folder from
the RSI Launcher log) or `manual`.

`p4kx channels add <path> --json` prints `{"action": "added", "kind": "folder", "path": "…"}`;
`action` is `added` or `already_added`, `kind` is `folder` or `archive`, and `path` is the
absolute path stored in the settings. `p4kx channels remove <path> --json` prints
`{"action": "removed", "path": "…"}`.

## `p4kx texture info <archive> <entry> --json`

Also `p4kx texture info <file> --json` for a header file on disk.

```json
{"texture": "Data\\Textures\\hull_ddna.dds", "format": "BC7_UNORM_SRGB", "width": 64,
 "height": 32, "depth": 1, "mip_count": 7, "array_size": 1, "faces": 1, "cubemap": false,
 "parts": 4, "alpha_tail": {"format": "BC4_UNORM", "parts": 4,
 "error": null}}
```

`texture` is the Entry path, or the path of the file on disk. `format` is the DXGI name of the
pixel format; legacy headers report their DXGI equivalent (`DXT1` is `BC1_UNORM`), and a format
this version does not know is described by its DXGI code, `FourCC` or bit masks. `depth` is 1
unless the texture is a volume, `faces` is 6 for a cubemap and 1 otherwise. `parts` is the number
of mip parts `.1` … `.N` the header needs, or `null` when the format's size is unknown.
`alpha_tail` is `null` without an alpha or smoothness tail `.a`; otherwise it gives the tail's
`format` and its number of `parts` `.1a` … `.Na`. A tail that cannot be read keeps the command
working: `format` and `parts` are then `null` and `error` says what is wrong.

## `p4kx texture merge <archive> <entry> --out <file> --json`

Also `p4kx texture merge <file> --out <file> --json` for a header file on disk.

```json
{"texture": "Data\\Textures\\hull_ddna.dds", "output": "…\\hull_ddna.dds", "parts": 4,
 "size": 2916}
```

`parts` is the number of mip parts merged and `size` the size of the written DDS file.

## `p4kx texture png <archive> <entry> --out <file> [--mip <level>] [--cross] [--tone-map] --json`

Also `p4kx texture png <file> --out <file> [--mip <level>] [--cross] [--tone-map] --json` for a
header file on disk.

```json
{"texture": "…\\hull_diff.dds", "output": "…\\hull_diff.png", "format": "png",
 "files": ["…\\hull_diff.png"], "mip": 1, "width": 32, "height": 16, "alpha_tail": false}
```

`output` is the `--out` path and `files` the files written: `output` itself, or for a cubemap
the six faces `<stem>_posx.png`, `_negx`, `_posy`, `_negy`, `_posz` and `_negz` next to it, or
with `--cross` `output` holding one horizontal cross. `format` is `png` for 8-bit RGBA PNG and
`exr` for OpenEXR with 32-bit float RGB, which a high-range BC6H texture is written as without
`--tone-map`; `files` then carry the extension `.exr` instead of the one of `output`. `mip` is the
exported level, 0 being the largest and the default; `width` and `height` are the size of each
written image. `alpha_tail` says whether an alpha or smoothness tail was merged into the alpha
channel. `--tone-map` on a texture that is not high-range writes the PNG as usual and prints a
note on stderr. A format without a decoder fails with exit code 1 and a
message naming the format; a level the texture does not have fails with exit code 2.
## `p4kx audio info <archive> <entry> --json`

Also `p4kx audio info <file> --json` for a WEM file on disk.

```json
{"sound": "Data\\Sounds\\wwise\\Media\\100.wem", "codec": "pcm", "codec_name": "PCM",
 "format_tag": 65534, "channels": 2, "sample_rate": 48000, "sample_count": 1200,
 "duration_seconds": 0.025}
```

`sound` is the Entry path, or the path of the file on disk. `codec` is one of `pcm`,
`ima_adpcm`, `vorbis`, `opus`, `ptadpcm`, `xwma`, `xma2`, `aac`, `dsp`, `hevag`, `atrac9` or
`unknown`; `codec_name` is its readable name and `format_tag` the raw tag of the `fmt` chunk.
`sample_count` is the number of frames (samples per channel) and `duration_seconds` the playing
time; both are `null` when the header does not state them and the codec does not allow computing
them.

## `p4kx audio wav <archive> <entry> --out <file> --json`

Also `p4kx audio wav <file> --out <file> --json` for a WEM file on disk.

```json
{"sound": "Data\\Sounds\\wwise\\Media\\100.wem", "output": "…\\100.wav", "channels": 2,
 "sample_rate": 48000, "sample_count": 1200, "size": 4844}
```

`sample_count` is the number of frames written and `size` the size of the written WAV file. A
codec without a decoder prints no JSON; it fails with exit code 1 and an error naming the codec.

## `p4kx audio ogg <archive> <entry> --out <file> --json`

Also `p4kx audio ogg <file> --out <file> --json` for a WEM file on disk.

```json
{"sound": "Data\\Sounds\\wwise\\Media\\300.wem", "output": "…\\300.ogg", "channels": 2,
 "sample_rate": 48000, "sample_count": 48000, "duration_seconds": 1.0, "codebooks": "aotuv",
 "size": 9876}
```

`sample_count` is the number of frames of the Ogg Vorbis stream, the granule position of its last
page, and `duration_seconds` its playing time. `codebooks` names the codebooks the setup header was
rebuilt with: `default`, `aotuv` or `inline`. `size` is the size of the written Ogg file. A codec
other than Wwise Vorbis, or a Vorbis WEM that no codebooks fit, prints no JSON; it fails with exit
code 1 and an error giving the reason.

## `p4kx bnk list <archive> <entry> --json`

Also `p4kx bnk list <file> --json` for a soundbank file on disk.

```json
{"soundbank": "Data\\Sounds\\wwise\\SSAM_Test.bnk", "version": 154, "bank_id": 3512036681,
 "wems": [{"id": 100, "size": 4852, "codec": "pcm", "codec_name": "PCM", "sample_count": 1200,
           "duration_seconds": 0.025, "prefetch": false, "error": null},
          {"id": 400, "size": 120, "codec": null, "codec_name": null, "sample_count": null,
           "duration_seconds": null, "prefetch": true, "error": null}]}
```

`version` and `bank_id` come from the `BKHD` section. `wems` lists every embedded WEM in
ascending ID order: `size` is its size in the bank, and `codec` to `duration_seconds` are as in
`p4kx audio info`. `prefetch` is `true` when the bank holds only the start of a streamed WEM, whose
whole file is the loose WEM of the same ID; its codec is then not read. `error` names why a WEM
cannot be read, e.g. Wwise plug-in media; the other fields are then `null`. A corrupt soundbank
prints no JSON; it fails with exit code 1 and an error naming the section.

## `p4kx bnk extract <archive> <entry> --out <folder> [--format wem|wav|ogg] --json`

Also `p4kx bnk extract <file> --out <folder> --json` for a soundbank file on disk.

```json
{"soundbank": "Data\\Sounds\\wwise\\SSAM_Test.bnk", "target": "…\\out", "format": "wav",
 "extracted": [{"id": 100, "file": "…\\out\\100.wav", "size": 9644, "prefetch": false}],
 "failed": [{"id": 300, "error": "… WEM 300: converting Wwise Vorbis to WAV is not supported"}]}
```

`format` is `wem`, `wav` or `ogg`, the extension of the files. `extracted` lists the WEMs written as
`<id>.<format>`, `failed` the WEMs that were not, with the reason and its causes joined by `: `;
both in ascending ID order. With `--format wav` a codec without a decoder and a prefetched start
fail; with `--format wem` a prefetched start is written as the bank holds it and marked with
`"prefetch": true`. With `--json`
failures are not repeated on stderr. The exit code is 1 when `failed` is not empty.

## `p4kx socpak list <archive> <container…> --json`

```json
{"archive": "C:\\…\\LIVE\\Data.p4k",
 "containers": ["Data\\ObjectContainers\\Ships\\AEGS\\ship_interior.socpak",
                "ship_interior\\modules\\cockpit.socpak"],
 "entries": [Entry, …]}
```

`containers` names the container Entry in the Archive, then each container inside the one before,
with the paths as they are stored. `entries` holds the Entries of the last container in directory
order; folder Entries are left out.

## `p4kx socpak extract <archive> <container…> --json`

The report of `p4kx extract … --json`, for the Entries of the last container.

## `p4kx chunks list <archive> <entry> --json`

Also `p4kx chunks list <file> --json` for a chunk file on disk.

```json
{"file": "Data\\Objects\\Test\\cube.cgf", "container": "#ivo", "version": 2304,
 "chunks": […,
            {"index": 2, "type": "Mesh", "type_id": 2459154904, "known": true, "version": 2305,
             "id": null, "byte_order": "little", "offset": 112, "size": 40},
            {"index": 3, "type": null, "type_id": 170165686, "known": false, "version": 2304,
             "id": null, "byte_order": "little", "offset": 152, "size": 8}]}
```

The example leaves out chunks 0 and 1 (`…`).
`container` is `#ivo` (version `0x900`) or `CrCh` (version `0x746`). `chunks` lists the chunk
table in file order; `index` is the position `p4kx chunks dump --chunk` takes. `type` is the name
of a known type and `null` for an unknown one, which `known` marks as well; `type_id` is the
32-bit hash of `#ivo` or the 16-bit ID of `CrCh`. `version` is the chunk's layout version, `id`
the chunk ID of `CrCh` and `null` in `#ivo`, and `byte_order` `little` or `big`, which only `CrCh`
chunks can be. `#ivo` stores no sizes: `size` is then the distance to the next greater offset of
any chunk, the chunk table or the end of the file, alignment padding included, so chunks that
share an offset share their span; the table may list chunks in any order. A file that is no chunk file, has
another container version or is truncated prints no JSON; it fails with exit code 3.

## `p4kx chunks dump <archive> <entry> --chunk <index> --out <file> --json`

Also `p4kx chunks dump <file> --chunk <index> --out <file> --json` for a chunk file on disk.

```json
{"file": "Data\\Objects\\Test\\cube.cgf", "out": "…\\mesh.bin",
 "chunk": {"index": 2, "type": "Mesh", "type_id": 2459154904, "known": true, "version": 2305,
           "id": null, "byte_order": "little", "offset": 112, "size": 40}}
```

`chunk` is the chunk written, as in `p4kx chunks list`; its `size` bytes are written unchanged.
An index the table does not have, or an output that is the chunk file itself, fails with exit
code 2.

## `p4kx extract … --json`

```json
{"target": "…", "extracted": 1, "skipped": ["Data\\readme.txt"],
 "failed": [{"path": "Data\\broken.xml", "error": "cannot read the Entry data: …"}],
 "cancelled": ["Data\\large.bin"], "archive_changed": false}
```

With `--convert` the report also holds `"converted": 1`, the number of Entries written converted
(CryXml Entries as XML, the Entries of converted textures and the DataForge Entry as Records);
they count among `extracted`. It also holds `"unconverted": [{"path":
"Data\\Textures\\hull_ddna.dds", "error": "cannot convert the texture: …"}]`, the textures by
their header Entry that could not be converted, whose raw Entries were written instead (except a
header whose path the merged DDS already took); the exit code is then 1. Without `--convert` both
fields are absent. When `--convert` converted the DataForge Entry, the report also holds
`records` with the fields `exported`, `skipped`, `failed`, `cancelled`, `error_values` and
`records_with_errors` of [`p4kx dcb export`](#p4kx-dcb-export-archive---json); `skipped` lists
the Record files relative to the folder of the DataForge Entry. Otherwise that field is absent.
`skipped` lists the Entries whose existing file in the target was kept under `--conflict`;
`failed` lists the Entries that were not written, with the reason and its causes joined by `: `;
`cancelled` lists the Entries not written because the run was cancelled. All three are in
Archive order. With `--json` failures are not repeated on stderr. `archive_changed` is `true`
when the Archive changed on disk after it was opened: the Extraction then stops, the Entries not
yet written are listed in `cancelled`, and the exit code is 4. Otherwise the exit code is 1 when
`failed` or `cancelled` is not empty.

## `p4kx compare <old> <new> --json`

The Comparison report, the same document `--report <file>` writes (there pretty-printed). Its
JSON Schema (draft 2020-12) is generated from the report types and checked in as
[`schemas/comparison-report.schema.json`](../schemas/comparison-report.schema.json); a test fails
when the two drift apart, and `P4KX_BLESS=1 cargo test -p p4kx-comparison --test report_schema`
regenerates it. The output is deterministic: Entries are ordered by path and Records by record
path, both ignoring case and separator kind, Records with the same record path by GUID. The exit
code is 0 without and 1 with differences on any compared level.

| Field | Type | Meaning |
|-------|------|---------|
| `format_version` | number | Version of the report shape, now `1` |
| `source.old`, `source.new` | object | `archive` (path), `channel` (string or `null`) and `build` (a [Build](#build) or `null`) of each side |
| `levels` | array | The levels compared, e.g. `["entries", "records"]` |
| `filter` | string or `null` | The Filter that selected the Entries by path and the Records by the record path of either side |
| `entries.counts` | object | `added`, `removed` and `changed` Entry counts |
| `entries.directories` | array | The same counts per top-level directory (`directory`, empty for the root), ordered ignoring case |
| `entries.changes` | array | One object per Entry: `path` (as the new side stores it, the old side for a removed Entry), `change` (`added`, `removed` or `changed`), and `old` and `new` with `size` and the checksum for the sides that have it: `crc32c` for an Archive Entry, `crc32` for an Entry of a plain ZIP archive |
| `records.counts` | object | `added`, `removed`, `moved`, `changed` and `unreadable` Record counts |
| `records.changes` | array | One object per Record: `guid`, `change` (`added`, `removed`, `moved`, `changed` or `unreadable`), `old` and `new` with `record_path`, `name` and `type` for the sides that have it and can be read, `fields`, and `errors` for an unreadable Record |
| `records.changes[].fields` | array | One object per changed field or array element, nested ones included: `path` (its location, e.g. `hull.plates[1].armor`), and `old` and `new` values for the sides that have it; empty for an added, removed or unreadable Record |
| `records.changes[].errors` | array | One object per side whose Record cannot be read: `side` (`old` or `new`) and `reason`; absent for every other Record |

`entries` is absent when the Entry level was not compared, `records` when the Record level was
not compared.

Records are joined by GUID. A Record is `moved` when its record path differs, ignoring case and
separator kind; its changed fields are listed as well. It is `changed` when its name, its type
or a field differs. It is `unreadable` when its record path, name, type or fields cannot be read
on a side; it is then not compared, and the Comparison goes on with the other Records.

The fields are compared down the whole Record tree. `path` is the location of the value in the
same notation as `at` in [Record XML](record-xml.md#child-elements): the field names from the
Record down separated by `.`, with `[index]` after an array field. The changes are listed depth
first: within one struct in field order of the new side and then the fields only the old side
has, each followed by the changes inside it. Fields are joined by name, so a field only one side
has is listed with only `old` or only `new`. Array elements are joined by index, so elements
added or removed at the end are listed with only `new` or only `old`; arrays are not matched by
key. A struct of another type, or a value of another kind (e.g. a scalar against a struct, or
null against a struct), is a change of its own, and the fields and elements of a struct or array
that only one side has follow it, each with only `old` or only `new`.

Each value is an object with `type` and `value`:

| `type` | `value` |
|--------|---------|
| `bool`, `int8` … `int64`, `uint8` … `uint64` | The number or boolean |
| `single`, `double` | The number; `NaN`, `INF` or `-INF` as a string for a float that is not finite |
| `string`, `locale` | The text; a localisation key as the key, e.g. `@item_Name` |
| `guid` | The GUID |
| `enum` | The option name |
| `reference` | The target: `guid`, `record_path` and `type`, and for a weak pointer `at`, its location inside the Record |
| `struct` | The name of its type; its fields follow as changes of their own |
| `array` | The number of elements; its elements follow as changes of their own |
| `null` | None: a null pointer or a null Reference has no `value` |
| `error` | The reason the value cannot be read; the rest of the Record is still compared |

Floats compare by their bits, every `NaN` equal to every other, so `-0.0` differs from `0.0`.
References compare by target GUID, so a target that moved is no change, and weak pointers, which
all share the GUID of their own Record, by their `at` as well. Values that cannot be read compare
by their reason.

## `p4kx compare <old> <new> --diff <entry> --json`

The line diff of one Entry, with three unchanged lines of context around each change:

```json
{"path": "Data\\Libs\\text.xml", "change": "changed", "differs": true, "content": "text",
 "old": {"size": 64, "crc32c": 3653534010, "encoding": "text"},
 "new": {"size": 83, "crc32c": 1515590036, "encoding": "text"},
 "hunks": [{"old_start": 1, "old_lines": 4, "new_start": 1, "new_lines": 5, "lines": [
   {"kind": "context", "text": "<Items>\n"},
   {"kind": "removed", "text": "  <Item name=\"a\" mass=\"1\"/>\n"},
   {"kind": "added", "text": "  <Item name=\"a\" mass=\"2\"/>\n"}]}]}
```

| Field | Type | Meaning |
|-------|------|---------|
| `path` | string | The Entry path as the new side stores it, the old side for a removed Entry |
| `change` | string | `added`, `removed`, `changed` or `unchanged`, by size and checksum from the central directories |
| `differs` | boolean | Whether the Entry differs: added, removed, changed in size or checksum, or with differing lines; it decides the exit code |
| `content` | string | `text`; `binary` when a side holds a NUL byte, or starts with a UTF-16 byte order mark but does not decode as UTF-16 without NUL characters; `too_large` when a side, or the XML converted from it, is larger than 128 MiB or has more than 4 194 304 lines |
| `old`, `new` | object or `null` | `size`, the checksum (`crc32c` for an Archive Entry, `crc32` for an Entry of a plain ZIP archive) and `encoding` of each side, `null` for a missing side; `encoding` is `text` (compared byte for byte), `utf-16le` or `utf-16be` (decoded after the byte order mark), the CryXml signature (`CryXmlB`, `CryXml`, `CRY3SDK`) the side was converted from, or `null` when it is not text |
| `hunks` | array | Empty unless `content` is `text`. Each hunk has `old_start` and `new_start` (one-based, as in a unified diff header, the line before for an empty side), `old_lines` and `new_lines` (counts) and `lines`, each with `kind` (`context`, `removed` or `added`) and `text` (the line with its terminator, for display with invalid UTF-8 replaced by U+FFFD; the last line of a side may have none) |

A missing side is diffed as empty text. Lines are compared by their bytes, so a change in bytes
that are not valid UTF-8 still shows as a hunk even where both `text` values look the same. The diff is a patience diff with a Myers fallback
where no line is unique; so that pathological pairs stay fast, a region that needs more than 1024
edits there, or every region once a fixed work budget is spent, is shown as removed and added as a
whole. An Entry whose bytes changed but whose lines (for CryXml: the converted XML) did not has
`change` `changed`, `differs` true and no hunks; without `--json` a note says so. The exit code
is 0 when `differs` is false, 1 when it is true, 2 when neither Archive has the Entry and 3 when an Entry cannot be read, does not
hold the size its central directory states, or holds corrupt CryXml. Memory stays bounded: each
side holds at most 128 MiB of text and about 9 bytes of bookkeeping per line.

## `p4kx cryxml <file> --out <file> --json`

```json
{"input": "…", "output": "…", "format": "CryXmlB", "byte_order": "little"}
```

`format` is the signature, `CryXmlB`, `CryXml` or `CRY3SDK`, or `text` for text XML written
unchanged; `byte_order` is `little`, `big`, or `null` for text XML. `--json` needs `--out`,
because the XML would otherwise share stdout with the JSON; without it the exit code is 2.

## `p4kx chf to-json <file> --out <file> --json` and `p4kx chf from-json <file> --out <file> --json`

```json
{"input": "…", "output": "…", "versions": {"female": 2, "male": 7}, "size": 21934}
```

`versions` are the inner versions of the CHF body; `size` is the number of bytes written, the
JSON text for `to-json` and always 4096 for `from-json`. `to-json --json` needs `--out`, because
the converted JSON would otherwise share stdout with the summary; without it the exit code is 2.

### The CHF JSON

`to-json` writes, and `from-json` reads, one indented object. `from-json` of an unchanged
`to-json` output gives back the original file byte for byte (ADR-0014); any other value gives a
new file with a correct CRC-32C. Unknown fields and values out of range are rejected with exit
code 2; a JSON file larger than 1 MiB is not read (exit code 3).

| Field | Type | Meaning |
|-------|------|---------|
| `character.versions` | object | `female` (supported: 2) and `male` (supported: 7, 8); from 8 on the body ends with a zero `u32` |
| `character.body_type`, `character.voice` | string | GUIDs in the form DataForge prints them; the voice is all zeros when none is chosen |
| `character.dna` | object or `null` | `null` in a CHF that holds only materials, as the game's presets do; otherwise `gender_hash`, `variant_hash`, `blends_per_part`, `header_unknown`, `max_head_id` and `face_parts`: one object per face part in stored order, `part` naming it (`eyebrow_left`, `eyebrow_right`, `eye_left`, `eye_right`, `nose`, `ear_left`, `ear_right`, `cheek_left`, `cheek_right`, `mouth`, `jaw`, `crown`, `neck`, then `part_<n>`) and `blends` with exactly `blends_per_part` objects `{value, head_id}` |
| `character.item_ports` | object or `null` | `null` when the CHF holds no ports; otherwise the root port: `port` (name hash), `item` (GUID), `tail_count` (a count the layout keeps, as read) and `children`, the same objects, omitted when empty; at most 32 levels |
| `character.item_port_tail` | number | The `u32` the layout keeps behind the port count, in a CHF without ports the number of materials; omitted when 0 |
| `character.materials` | array | One object per attachment: `attachment` (name hash), `base_material` (GUID), `flags` and `sub_materials`, each with `name` (hash), `textures` (`slot`, `texture` GUID), `floats` (`name`, `value`) and `colors` (`name`, `rgba` as four numbers 0 to 255) |
| `container.flags` | number | The two header bytes behind the magic |
| `container.frame` | string | The original zstd frame as hexadecimal; reused while it decompresses to the body written from `character`, otherwise the body is compressed anew |
| `container.padding` | string | The bytes behind the frame as hexadecimal, without trailing zeros; written only with the reused frame, a body compressed anew is followed by zeros |

Hashes are unsigned 32-bit numbers. A float is a JSON number, or the string `0x` with its eight
hexadecimal digits when it is not finite (`"0x7fc00000"` for the usual NaN); every float is
written back with exactly the bits it was read with. `container` and each of its fields may be
left out; without `frame` the body is always compressed anew.

