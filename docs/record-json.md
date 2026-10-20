# Record JSON

Record JSON is the JSON form of one Record of DataForge, written by
`p4kx dcb export --format json|ndjson`, by the `RecordJson` writer of `p4kx-dataforge` and by the
Record export of `p4kx-export`. It serialises the canonical Record tree, the same tree as
[Record XML](record-xml.md), with the same structure: owned children are nested, References are
links and never embedded ([ADR-0004](adr/0004-own-record-xml-with-references.md)). Every value of
the Record XML is in the JSON and the other way round.

The JSON Schema (draft 2020-12) is
[`schemas/record-json.schema.json`](../schemas/record-json.schema.json); a test validates every
Record the export writes against it. The same Record always gives the same bytes.

## Example

The Record of the [Record XML example](record-xml.md#example):

```json
{
  "$type": "Vehicle",
  "$guid": "cbee34fd-6441-4000-b99f-924f7f26e19f",
  "$path": "libs/foundry/records/vehicles/cart",
  "mass": 250.5,
  "label": "@vehicle_Cart",
  "maker": {
    "guid": "50e8a9f6-f956-4000-8ace-7c0ca9a91f5d",
    "path": "libs/foundry/records/makers/woodworks",
    "type": "Manufacturer"
  },
  "wheels": [
    {
      "$type": "Wheel",
      "size": 16
    },
    {
      "$type": "Wheel",
      "size": 17
    }
  ],
  "spare": {
    "$type": "Wheel",
    "size": 15
  },
  "trailer": null,
  "front": {
    "guid": "cbee34fd-6441-4000-b99f-924f7f26e19f",
    "path": "libs/foundry/records/vehicles/cart",
    "type": "Wheel",
    "at": "wheels[0]"
  },
  "tags": [
    "wood",
    "old"
  ],
  "path": "hangar",
  "name": {
    "error": "offset 4294967295 in the text table is not a valid string"
  }
}
```

## Document

- UTF-8 without byte order mark, LF line endings.
- One object per Record. `--format json` writes it pretty: two spaces of indent per level, one key
  or element per line, `"key": value`, `[]` and `{}` for empty arrays and objects, and a final line
  ending. NDJSON writes it compact: one line without any whitespace, ended by a line feed.
- Strings escape `"` and `\`, line feed, carriage return and tab as `\n`, `\r` and `\t`, and the
  other control characters below U+0020 as `\u00XX` (lower-case hex); every other character is
  written as UTF-8. Scalars and errors are written exactly as in the
  [JSON output of Queries](query.md#json).

## Record and struct objects

A Record object holds, in this order:

1. `$type`: the Record's type (its struct), e.g. `EntityClassDefinition`.
2. `$guid`: the Record's GUID, lower case, `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`.
3. `$path`: the record path without `.xml`, as DataForge stores it.
4. One key per field, in the order of the Record XML: first the fields Record XML writes as
   attributes, then the others, each group in declaration order (the fields of the root ancestor
   first). A field is an attribute in Record XML when its value is a scalar and its name is an
   attribute name there ([Names](record-xml.md#names)).

A nested struct (an inline class or the struct a strong pointer owns) is an object with `$type`,
its actual type, followed by its fields under the same rules. `$guid` and `$path` appear only on
the Record object.

A field is written under its name unless the name starts with `$` or an earlier field of the
same struct, in declaration order, has the same name. Such a field is written as
`"$<n>": {"name": "…", "value": …}`, with `n` its position among the struct's fields in
declaration order, counting from 0, so no value is lost or mistaken for another. No field is ever
written under its own name when that starts with `$`: `$type`, `$guid`, `$path` and `$<n>` are the only
keys that do.

## Values

| Value | JSON |
|-------|------|
| bool | `true` or `false` |
| int8 … int64, uint8 … uint64 | A number, exact; values beyond ±2^53 lose precision in readers that use doubles |
| float, double | A number in the text Record XML uses: the shortest text that reads back to the same value, with an exponent (`1e21`, `1.5e-7`) from 1e21 and below 1e-6 in magnitude, `-0` for negative zero. NaN and the infinities, which JSON has no number for, are the strings `"NaN"`, `"INF"` and `"-INF"` |
| string | A string |
| localisation key | The key as stored, e.g. `"@item_Name"`; with `--lang` an object, see [Resolved texts](#resolved-texts) |
| GUID | A lower-case string `"xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"` |
| enum | The name of the option as a string |
| struct | A struct object, see above |
| array | An array of the element values in stored order |
| Reference | `{"guid": "…", "path": "…", "type": "…"}` with the GUID, record path and type of the target Record; the target's content is never written |
| weak pointer | A Reference into the Record that holds it, with the same `guid` and `path`, the type of the target and its location in `at`, e.g. `"at": "Components[23].AnimationJointControls.variables[0]"`; one to the Record's own instance has no `at` (see [Record XML](record-xml.md#child-elements)) |
| null pointer or null Reference | `null` |
| error | `{"error": "…"}` with the reason, at the place of the value that cannot be read |

A struct object always has `$type`, a Reference never; an error has only `error`. The errors and
the Records that are not written at all are those of [Record XML](record-xml.md#errors).

## Resolved texts

`p4kx dcb export --lang <language>` writes the text of every localisation key in the `global.ini` of
that language. Without `--lang` a key is the plain string above and the output is byte for byte
the output described above, free of any language. With `--lang` a localisation key that is no
placeholder (`@LOC_EMPTY`, `@LOC_PLACEHOLDER`, `@LOC_UNINITIALIZED`, `@blank_space`) and not empty
is an object in the place of the string, in fields and in arrays alike:

```json
{"key": "@item_Name", "text": "Crate"}
{"key": "@item_Gone", "missing": true}
```

`text` is the text with the `\n` escapes of the `global.ini` turned into line feeds. `missing` is
`true` when the language has no text for the key: no `global.ini` line, or an empty text. An object
with `key` has exactly one of `text` and `missing`; it is told from a struct (`$type`), a Reference
(`guid`) and an error (`error`) by its `key`. A placeholder or empty key stays the plain string: it
stands for no text and is not a missing translation. The
[JSON Schema](../schemas/record-json.schema.json) allows both forms. The same Records and the same
language always give the same bytes.

## Files

`--format json` writes one file per Record below the target folder under the rules of
[Record XML](record-xml.md#files) with `.json` instead of `.xml`: `<record path>.json` for a
Record alone at its record path, `<record path>/<Record name>.json` for Records that share one
([ADR-0008](adr/0008-record-files-by-record-path.md)). `--conflict` applies to every file.

`--format ndjson` writes the one file `records.ndjson` into the target folder: one compact Record
object per line, ordered by record path ignoring ASCII case with `/` and `\` alike, then by Record
name ignoring ASCII case, then in definition order, the order of `--combined`; each selected
Record once. The Records are built in parallel batches of at most 4096 Records and about 64 MiB
and streamed into a temporary file, which replaces `records.ndjson` only when complete; a
cancelled run leaves no file. `--conflict` applies to `records.ndjson`: `skip` keeps an existing file
without building any Record, `if-changed` keeps it when size and CRC-32C equal the new content.
