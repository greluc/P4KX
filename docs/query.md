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
from AmmoParams at "*/vehicle/*" select speed, lifetime where speed >= 1500 and lifetime <= 3
from EntityClassDefinition at "*/spaceships/*" select Category, Invisible where Category = "Default" and not Invisible = true
at "*/spaceships/*" select Icon, Category where Icon like "default.*" and (Category contains "def" or Invisible = true)
```

- `from EntityClassDefinition` lists every Record of type `EntityClassDefinition` or of a struct
  deriving from it.
- `at "…"` lists the Records whose record path matches.
- `select Category, Invisible` adds one column per dot path behind the record path.
- `where speed >= 1500 and lifetime <= 3` keeps only the Records whose values meet the condition.

The opt-in tests against a real Archive run these examples.

## Result

The result is a table. Its first column is always the record path; then follows one column per
path of `select`, in the order written, headed by the path as the canonical form prints it.

There is one row per selected Record, ordered by record path (ignoring ASCII case, `\` and `/`
alike) and then by the Record's position in the DataForge. Records share record paths, so a
record path can appear in several rows.

Without `from`, a Record whose type has no scalar field of a selected name gives an empty cell;
with `from`, every selected type declares the fields of the named type. A value that cannot be
read shows its reason as `error: …`. A Record whose layout cannot be read gives no row and is
reported as failed; with `where`, so is a Record whose values cannot be read to decide the
filter.

## Grammar

```text
query      = selection [ "select" path { "," path } ] [ "where" condition ] ;
selection  = "from" name [ "at" string ]
           | "at" string ;
path       = name { "." name } ;
condition  = term { "or" term } ;
term       = factor { "and" factor } ;
factor     = "not" factor
           | "(" condition ")"
           | path compare value
           | path match string ;
compare    = "=" | "!=" | "<" | "<=" | ">" | ">=" ;
match      = "contains" | "starts" "with" | "like" ;
value      = number | string | "true" | "false" ;
number     = [ "-" ] digit { digit } [ "." digit { digit } ]
             [ ( "e" | "E" ) [ "+" | "-" ] digit { digit } ] ;
name       = bare-name | quoted-name ;
bare-name  = ( letter | "_" ) { letter | digit | "_" } ;    (not a keyword)
quoted-name = "`" char-but-backtick { char-but-backtick } "`" ;
string     = '"' { char-but-quote | '""' } '"' ;
```

- `letter` is an ASCII letter and `digit` an ASCII digit.
- Whitespace (any Unicode whitespace) may stand between any two tokens and is ignored.
- The clauses come in the order `from`, `at`, `select`, `where`; each appears at most once, and
  at least one of `from` and `at` is given.
- A Query is at most 65,536 bytes long.

### Keywords

`from`, `at`, `select`, `where`, `and`, `or` and `not` are keywords; they ignore case, so
`FROM` and `From` are `from`. A type or field whose name is a keyword, or is no bare name, is
written in backticks: `` `select` ``, `` `Size-X` ``, e.g. ``from Tag select `select` ``. A quoted
name holds any characters but a backtick and is never empty.

The word operators `contains`, `starts with` and `like` and the values `true` and `false` also
ignore case, but they are words only where an operator or a value stands: everywhere else they
are names, so ``from Tag where like = true`` compares the field `like`. A word in backticks is
always a name.

### Strings

A string holds any characters between double quotes; a doubled quote `""` stands for one quote,
e.g. `"Box ""large"""` is `Box "large"`. Nothing else is an escape: a backslash is a backslash.

### Names

Type and field names are case-sensitive and match the names DataForge declares exactly. An
unknown name is an error that names the closest known one, if any, e.g.
``did you mean `mass`?``.

- `from <name>` names a struct. It selects the Records of that struct and of every struct that
  derives from it, directly or through other structs.
- A path starts with a field of the Record's type, inherited fields included. With `from`, the
  fields are those of the named struct. Without `from`, each Record's own type is used, and a name
  that no struct declares is an error.
- Only fields that hold one scalar value can be selected or compared: booleans, integers, floats,
  strings, localisation keys, GUIDs and enum values. With `from`, a field that holds an array, a
  struct, a pointer or a Reference is an error. Without `from`, it is an error only when no
  struct declares the name as a scalar field; a type that declares it otherwise gives an empty
  cell. A path that continues behind a scalar field is an error.

### Record-path glob

The string of `at` matches record paths with the rules of a Filter: with `*` (any run of
characters, separators included) or `?` (exactly one character) it is a glob over the full
record path; otherwise it matches when it occurs anywhere in the record path. Both forms ignore
ASCII case and treat `/` and `\` alike. The record path has no `.xml` extension.

## Conditions

`where` keeps the Records whose values meet a condition. `not` binds tightest, then `and`, then
`or`: `a = 1 or b = 2 and not c = 3` means `a = 1 or (b = 2 and (not c = 3))`. Parentheses group
differently. Parentheses and `not` nest at most 32 deep.

### Comparisons

A comparison reads the value at its path and compares it with the value written after the
operator. Which values a field can be compared with depends on its type; any other pairing is an
error marked on the comparison, e.g. ``"`=` cannot compare field `dps` of `Weapon`, which holds
Double values, with a string"``.

| Field type | Value | Operators | Rule |
| --- | --- | --- | --- |
| integers (`Int8` … `UInt64`) | a number without fraction or exponent | `=` `!=` `<` `<=` `>` `>=` | by value, exactly, across all integer types |
| `Single`, `Double` | any number | `=` `!=` `<` `<=` `>` `>=` | by value as IEEE 754 floats, see below |
| `String`, `Locale` | a string | `=` `!=` `<` `<=` `>` `>=` | exactly: case-sensitive, ordered by Unicode code point |
| `Enum` | a string | `=` `!=` `<` `<=` `>` `>=` | by the option name, as for a string |
| `String`, `Locale`, `Enum` | a string | `contains` `starts with` `like` | ignoring ASCII case |
| `Guid` | a string `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx` | `=` `!=` `<` `<=` `>` `>=` | by value, ordered as the lower-case text form |
| `Bool` | `true` or `false` | `=` `!=` `<` `<=` `>` `>=` | `false` before `true` |

- An integer value beyond every integer type (more than about 39 digits) is an error.
- A number compared with a float field is first rounded to the field's precision, `Single` or
  `Double`, as the value would be stored, so `mass = 0.1` finds a `Single` field that holds 0.1.
  A number beyond the range of that precision is an error.
- Floats compare as IEEE 754 does: `-0` equals `0`, and NaN equals nothing, not even NaN, and is
  neither less nor greater than any number. So every comparison with NaN is false except `!=`,
  which is true.
- A GUID string with letters in either case is accepted; anything else is an error marked on the
  string.
- `contains` holds when the string occurs in the value, `starts with` when the value begins with
  it, and `like` when the whole value matches the string as a glob: `*` matches any run of
  characters and `?` exactly one. All three ignore ASCII case only; other characters match
  exactly. An empty string is contained in every value.

Without `from`, a comparison is an error only when no struct declares the field as a scalar
field that can be compared with the value; a Record whose type declares it otherwise, or not at
all, has no value for it.

### Missing values

A comparison is unknown when the Record has no value for its path: its type does not declare the
field (without `from`), declares it with a type the value cannot be compared with, or the value
cannot be read. An unknown comparison never matches, and the conditions combine as in SQL:
`not` of unknown is unknown, `false and unknown` is false, `true or unknown` is true, and any
other combination with unknown is unknown. A Record is kept only when the whole condition is
true, so `x = 1` and `not x = 1` together never select a Record without a value for `x`.

## Errors

A Query that cannot be parsed or compiled is rejected before any Record is read. The error names
the problem and its position: the CLI prints the line of the Query and marks the span with `^`:

```text
error: unknown field `mas` of `Item`; did you mean `mass`?
  from Item select mas
                   ^^^
```

`p4kx dcb query` exits with code 2 for an invalid Query, and with code 5 when the struct named
by `from` has a layout the DataForge cannot give, since that DataForge is corrupt.

## Canonical form

A parsed Query prints in one canonical form: keywords and word operators in lower case, one
space between tokens, `, ` between paths, no spaces around `.`, backticks only where a name
needs them, numbers as written, strings with each quote doubled, and parentheses only where
precedence needs them. Parsing the canonical form gives the same Query.
