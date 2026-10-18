# Queries

A Query selects Records of DataForge and values inside them. It is run by
`p4kx dcb query <archive> "<query>"` and by the `Query` API of `p4kx-dataforge`. The syntax is
SQL-like ([ADR-0009](adr/0009-sql-like-query-syntax.md)).

## Examples

```text
from EntityClassDefinition
at "libs/foundry/records/entities/spaceships/*"
from EntityClassDefinition at "*/spaceships/*" select Category, Invisible
at "*/spaceships/*" select Icon
```

- `from EntityClassDefinition` lists every Record of type `EntityClassDefinition` or of a struct
  deriving from it.
- `at "…"` lists the Records whose record path matches.
- `select Category, Invisible` adds one column per dot path behind the record path.

The opt-in tests against a real Archive run these examples.

## Result

The result is a table of rows and columns, written as a table, CSV or JSON (see
[Output formats](#output-formats)). Its first column is always the record path, headed
`record path`; then follows one column per path of `select`, in the order written, headed by the
path as the canonical form prints it.

There is one row per selected Record, ordered by record path (ignoring ASCII case, `\` and `/`
alike) and then by the Record's position in the DataForge. Records share record paths, so a
record path can appear in several rows.

Without `from`, a Record whose type has no scalar field of a selected name gives an empty cell;
with `from`, every selected type declares the fields of the named type. A value that cannot be
read shows its reason as `error: …`. A Record whose layout cannot be read gives no row and is
reported as failed.

## Output formats

`p4kx dcb query <archive> "<query>" [--format table|csv|json] [--out <file>]` writes the result
in one of three formats; the default is `table`, or `json` with `--json`. All three have the
same columns and rows in the same order. No heading of a `select` path is `record path`,
because a name with a space is written in backticks.

The result is written page by page: the rows of one page are built in parallel and written
before the next page is built, so the result is never held in memory as a whole. With `--out`
it goes to a temporary file next to the target, which is renamed to the target once complete
and replaces an existing file; stdout then gets one summary line such as
`3 Records written to result.csv`, or a JSON summary with `--json` (see
[cli-json.md](cli-json.md)). A Record whose layout cannot be read gives no row in any format;
it is reported on stderr and the exit code is 1. An `--out` that is the Archive itself is
rejected with exit code 2, and so is `--json` with another format and without `--out`.

### Table

Columns separated by two spaces and padded to the widest text of the column over all rows, a
line of `-` under the headings, and on stdout a last line with the number of Records. Line
breaks and tabs in a value are shown as `\n`, `\r` and `\t`, other control characters as
U+FFFD. The widths are measured in a first pass over the pages, so the Records are read twice.
The table is for reading; scripts use CSV or JSON.

### CSV

CSV after [RFC 4180](https://www.rfc-editor.org/rfc/rfc4180):

- UTF-8 without byte order mark; every row, the header row included, ends with CRLF.
- The header row holds `record path` and then each `select` path in canonical form.
- One row per Record with exactly one field per column.
- A field holding `,`, `"`, CR or LF is enclosed in `"`, and each `"` in it is doubled; every
  other field is written as it is. A line break inside a value stays inside the quoted field.
- A value is written in its Record XML text ([record-xml.md](record-xml.md#values)): `true`,
  `25000.5`, `NaN`, the enum option, the GUID in lower case. A value that cannot be read is
  `error: <reason>`. An empty cell (a Record without the field) is an empty field, as is an
  empty string.
- Values are written unchanged: a text starting with `=`, `+`, `-` or `@` stays as it is, so a
  spreadsheet may read it as a formula.

```text
record path,name,crew,mass
libs/items/box,Box,,-0.25
libs/ships/avenger,"Avenger, ""Titan""",2,25000.5
```

### JSON

One line of compact JSON ending in a line feed: an array with one object per row, in row
order. Each object has the key `record path` and one key per `select` path, in column order; a
path selected twice appears once, as its values are the same. The values are typed:

| Field type | JSON |
|------------|------|
| bool | `true` or `false` |
| int8 … int64, uint8 … uint64 | A number, exact for every 64-bit value |
| float, double | A number in its Record XML text (`25000.5`, `1e21`, `-0`); `NaN`, `INF` and `-INF` as strings |
| string, localisation key, enum | A string: the text, the key as stored, the option name |
| GUID | A string, lower case, `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx` |
| a value that cannot be read | An object `{"error": "<reason>"}` |
| an empty cell | `null` |

Strings escape `"` and `\`, line feed, carriage return and tab as `\n`, `\r` and `\t`, and the
other control characters below U+0020 as `\u00XX`; every other character is written as UTF-8.

```json
[{"record path":"libs/items/box","name":"Box","crew":null,"mass":-0.25},{"record path":"libs/ships/avenger","name":"Avenger, \"Titan\"","crew":2,"mass":25000.5}]
```

## Grammar

```text
query      = selection [ "select" path { "," path } ] ;
selection  = "from" name [ "at" string ]
           | "at" string ;
path       = name { "." name } ;
name       = bare-name | quoted-name ;
bare-name  = ( letter | "_" ) { letter | digit | "_" } ;    (not a keyword)
quoted-name = "`" char-but-backtick { char-but-backtick } "`" ;
string     = '"' { char-but-quote } '"' ;
```

- `letter` is an ASCII letter and `digit` an ASCII digit.
- Whitespace (any Unicode whitespace) may stand between any two tokens and is ignored.
- The clauses come in the order `from`, `at`, `select`; each appears at most once, and at least
  one of `from` and `at` is given.
- A Query is at most 65,536 bytes long.

### Keywords

`from`, `at`, `select`, `where`, `and`, `or` and `not` are keywords; they ignore case, so
`FROM` and `From` are `from`. `where`, `and`, `or` and `not` are reserved for filters. A type or
field whose name is a keyword, or is no bare name, is written in backticks: `` `select` ``,
`` `Size-X` ``, e.g. ``from Tag select `select` ``. A quoted name holds any characters but a
backtick and is never empty.

### Names

Type and field names are case-sensitive and match the names DataForge declares exactly. An
unknown name is an error that names the closest known one, if any, e.g.
``did you mean `mass`?``.

- `from <name>` names a struct. It selects the Records of that struct and of every struct that
  derives from it, directly or through other structs.
- A path starts with a field of the Record's type, inherited fields included. With `from`, the
  fields are those of the named struct. Without `from`, each Record's own type is used, and a name
  that no struct declares is an error.
- Only fields that hold one scalar value can be selected: booleans, integers, floats, strings,
  localisation keys, GUIDs and enum values. With `from`, a field that holds an array, a struct, a
  pointer or a Reference is an error. Without `from`, it is an error only when no struct declares
  the name as a scalar field; a type that declares it otherwise gives an empty cell. A path that
  continues behind a scalar field is an error.

### Record-path glob

The string of `at` matches record paths with the rules of a Filter: with `*` (any run of
characters, separators included) or `?` (exactly one character) it is a glob over the full
record path; otherwise it matches when it occurs anywhere in the record path. Both forms ignore
ASCII case and treat `/` and `\` alike. The record path has no `.xml` extension. A string cannot
contain `"`.

## Errors

A Query that cannot be parsed or compiled is rejected before any Record is read. The error names
the problem and its position: the CLI prints the line of the Query and marks the span with `^`:

```text
error: unknown field `mas` of `Item`; did you mean `mass`?
  from Item select mas
                   ^^^
```

`p4kx dcb query` exits with code 2 for an invalid Query.

## Canonical form

A parsed Query prints in one canonical form: keywords in lower case, one space between tokens,
`, ` between paths, no spaces around `.`, and backticks only where a name needs them. Parsing the
canonical form gives the same Query.
