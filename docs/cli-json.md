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
 "parts": 4}
```

`texture` is the Entry path, or the path of the file on disk. `format` is the DXGI name of the
pixel format; legacy headers report their DXGI equivalent (`DXT1` is `BC1_UNORM`), and a format
this version does not know is described by its DXGI code, `FourCC` or bit masks. `depth` is 1
unless the texture is a volume, `faces` is 6 for a cubemap and 1 otherwise. `parts` is the number
of mip parts `.1` … `.N` the header needs, or `null` when the format's size is unknown.

## `p4kx texture merge <archive> <entry> --out <file> --json`

Also `p4kx texture merge <file> --out <file> --json` for a header file on disk.

```json
{"texture": "Data\\Textures\\hull_ddna.dds", "output": "…\\hull_ddna.dds", "parts": 4,
 "size": 2916}
```

`parts` is the number of mip parts merged and `size` the size of the written DDS file.

## `p4kx extract … --json`

```json
{"target": "…", "extracted": 1, "skipped": ["Data\\readme.txt"],
 "failed": [{"path": "Data\\broken.xml", "error": "cannot read the Entry data: …"}],
 "cancelled": ["Data\\large.bin"], "archive_changed": false}
```

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
regenerates it. The output is deterministic: Entries are ordered by path ignoring case and
separator kind. The exit code is 0 without and 1 with differences.

| Field | Type | Meaning |
|-------|------|---------|
| `format_version` | number | Version of the report shape, now `1` |
| `source.old`, `source.new` | object | `archive` (path), `channel` (string or `null`) and `build` (a [Build](#build) or `null`) of each side |
| `levels` | array | The levels compared, e.g. `["entries"]` |
| `filter` | string or `null` | The Filter that selected the Entries on both sides |
| `entries.counts` | object | `added`, `removed` and `changed` Entry counts |
| `entries.directories` | array | The same counts per top-level directory (`directory`, empty for the root), ordered ignoring case |
| `entries.changes` | array | One object per Entry: `path` (as the new side stores it, the old side for a removed Entry), `change` (`added`, `removed` or `changed`), and `old` and `new` with `size` and `crc32c` for the sides that have it |

`entries` is absent when the Entry level was not compared.

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
| `change` | string | `added`, `removed`, `changed` or `unchanged`, by size and CRC-32C from the central directories |
| `differs` | boolean | Whether the Entry differs: added, removed, with differing lines, or not text and changed; it decides the exit code |
| `content` | string | `text`; `binary` when a side holds a NUL byte; `too_large` when a side, or the XML converted from it, is larger than 128 MiB |
| `old`, `new` | object or `null` | `size`, `crc32c` and `encoding` of each side, `null` for a missing side; `encoding` is `text`, the CryXml signature (`CryXmlB`, `CryXml`, `CRY3SDK`) the side was converted from, or `null` when it is not text |
| `hunks` | array | Empty unless `content` is `text`. Each hunk has `old_start` and `new_start` (one-based, as in a unified diff header, the line before for an empty side), `old_lines` and `new_lines` (counts) and `lines`, each with `kind` (`context`, `removed` or `added`) and `text` (the line with its terminator; the last line of a side may have none) |

A missing side is diffed as empty text; text with a UTF-16 byte order mark is decoded, other text
is read as UTF-8 with invalid sequences replaced. The diff is a patience diff with a Myers fallback
where no line is unique; so that pathological pairs stay fast, a region that needs more than 1024
edits there, or every region once a fixed work budget is spent, is shown as removed and added as a
whole. An Entry whose bytes changed but whose text (for CryXml: the converted XML) did not has
`change` `changed`, `differs` false and no hunks; without `--json` a note says so. The exit code is 0 when `differs` is false, 1
when it is true, 2 when neither Archive has the Entry and 3 when an Entry cannot be read or holds
corrupt CryXml.

## `p4kx cryxml <file> --out <file> --json`

```json
{"input": "…", "output": "…", "format": "CryXmlB", "byte_order": "little"}
```

`format` is the signature, `CryXmlB`, `CryXml` or `CRY3SDK`, or `text` for text XML written
unchanged; `byte_order` is `little`, `big`, or `null` for text XML. `--json` needs `--out`,
because the XML would otherwise share stdout with the JSON; without it the exit code is 2.
