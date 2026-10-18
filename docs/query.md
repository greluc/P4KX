# Queries

A Query selects Records of DataForge and values inside them. It is run by
`p4kx dcb query <archive> "<query>"` and by the `Query` API of `p4kx-dataforge`. The syntax is
SQL-like ([ADR-0009](adr/0009-sql-like-query-syntax.md)).

## Examples

```text
from EntityClassDefinition
at "libs/foundry/records/entities/spaceships/*"
from Ship at "*/ships/*" select crew, mass
from Item select mass, size
at "*/weapons/*" select size
from `select` select `from`
```

- `from EntityClassDefinition` lists every Record of type `EntityClassDefinition` or of a struct
  deriving from it.
- `at "…"` lists the Records whose record path matches.
- `select crew, mass` adds one column per dot path behind the record path.

## Result

The result is a table. Its first column is always the record path; then follows one column per
path of `select`, in the order written, headed by the path as the canonical form prints it.

There is one row per selected Record, ordered by record path (ignoring ASCII case, `\` and `/`
alike) and then by the Record's position in the DataForge. Records share record paths, so a
record path can appear in several rows.

A path that names a field the Record's type does not declare gives an empty cell; this happens
only without `from`, since every type `from` selects declares the fields of the named type. A
value that cannot be read shows its reason as `error: …`.

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
`` `Size-X` ``. A quoted name holds any characters but a backtick and is never empty.

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
  localisation keys, GUIDs and enum values. A field that holds an array, a struct, a pointer or a
  Reference is an error, and so is a path that continues behind a scalar field.

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
