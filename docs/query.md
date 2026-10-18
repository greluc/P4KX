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
from EntityClassDefinition at "libs/foundry/records/entities/spaceships/*" select Components[VehicleComponentParams].vehicleName, Components[VehicleComponentParams].crewSize where Components[VehicleComponentParams].crewSize >= 4
from EntityClassDefinition at "*/ships/weapons/*" select Components[SAttachableComponentParams].AttachDef.Size, Components[SAttachableComponentParams].AttachDef.Type where Components[SAttachableComponentParams].AttachDef.Size >= 7
```

- `from EntityClassDefinition` lists every Record of type `EntityClassDefinition` or of a struct
  deriving from it.
- `at "…"` lists the Records whose record path matches.
- `select Category, Invisible` adds one column per dot path behind the record path.
- `where speed >= 1500 and lifetime <= 3` keeps only the Records whose values meet the condition.
- `Components[VehicleComponentParams].crewSize` reads the field `crewSize` of the components of
  type `VehicleComponentParams`, wherever they stand in the array `Components`
  ([Paths into nested data](#paths-into-nested-data)).

The opt-in tests against a real Archive run these examples.

## Result

The result is a table. Its first column is always the record path; then follows one column per
path of `select`, in the order written, headed by the path as the canonical form prints it.

There is one row per selected Record, ordered by record path (ignoring ASCII case, `\` and `/`
alike) and then by the Record's position in the DataForge. Records share record paths, so a
record path can appear in several rows.

A cell holds the one value its path reaches, or for a path through `[*]` or `[TypeName]` a list
of every value it reaches ([Several values](#several-values)). A null pointer or an index out of
range on the way gives `null`. Without `from`, a Record whose type does not resolve the path,
e.g. has no field of the first name, gives an empty cell; with `from`, every selected type
declares the fields of the named type. A value that cannot be read shows its reason as
`error: …`. A Record whose layout cannot be read gives no row and is
reported as failed; with `where`, so is a Record whose values cannot be read to decide the
filter.

## Grammar

```text
query      = selection [ "select" path { "," path } ] [ "where" condition ] ;
selection  = "from" name [ "at" string ]
           | "at" string ;
path       = name { segment } ;
segment    = "." name
           | "[" index "]"
           | "[" "*" "]"
           | "[" name "]" ;
index      = digit { digit } ;
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
- A path has at most 64 segments behind its first name.
- An index is a whole number from 0 to 4,294,967,295 without sign, fraction or exponent;
  leading zeros are allowed, so `[007]` is `[7]`.

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
- A name behind `.` is a field of the struct the path has reached, and a name in brackets is a
  struct; both follow [Paths into nested data](#paths-into-nested-data).

### Record-path glob

The string of `at` matches record paths with the rules of a Filter: with `*` (any run of
characters, separators included) or `?` (exactly one character) it is a glob over the full
record path; otherwise it matches when it occurs anywhere in the record path. Both forms ignore
ASCII case and treat `/` and `\` alike. The record path has no `.xml` extension.

## Paths into nested data

A path walks from a field of the Record into the values it holds, segment by segment, and has to
end at a scalar: a boolean, integer, float, string, localisation key, GUID or enum value.

| Segment | Behind | Reaches |
| --- | --- | --- |
| `.name` | an inline struct, or a strong pointer | the field `name` of the struct, or of the struct the pointer owns |
| `[n]` | an array | the element at index `n`, counted from 0; `null` when the array has `n` or fewer elements |
| `[*]` | an array | every element, in stored order |
| `[TypeName]` | an array of inline structs or strong pointers | every element whose struct is `TypeName` or derives from it, in stored order |

- Behind a strong pointer the fields are those of the struct the field declares, also when the
  pointer owns an instance of a struct deriving from it; behind `[TypeName]` they are those of
  `TypeName`, so `Components[VehicleComponentParams].crewSize` reads a field the base type of the
  components does not have.
- A null pointer reaches `null`, and so does a path through it; so does an index out of range.
  `[TypeName]` skips null pointers, as they have no type.
- A pointer to an instance of a struct that does not derive from the declared one, an array that
  lies outside its values or a pointer to a missing instance reaches an error value, shown as
  `error: …` at its place.
- References and weak pointers are links, not owned values: a path cannot continue behind them,
  and since they are no scalars, it cannot end at them either.

These are compile errors, marked on the segment or name that causes them:

- a field the struct does not declare, or a `TypeName` no struct has, with a suggestion;
- `.name` behind a scalar, behind an array (select its elements first) or behind a Reference or
  a weak pointer;
- `[n]`, `[*]` or `[TypeName]` behind a value that is no array, and `[TypeName]` behind an array
  of scalars or links;
- a `TypeName` that neither is nor derives from the struct of the elements, nor is one of its
  bases, so no element can have it;
- a path that ends at a struct, an array, a pointer or a link.

Without `from`, the path is resolved for each Record's own type: a type that does not resolve it
gives an empty cell and no value to compare, and the path is a compile error only when it
resolves in no type; the error is then the one of the first struct that declares its first
field.

### Several values

A path through `[*]` or `[TypeName]` can reach any number of values, so its cell is always a
list, also when it reaches one value or none: the values in the order the path reaches them,
nested arrays flattened, e.g. `Components[*].name` of a ship with a null first component is
`[null, Wall]`. A null or error value stays at its place, so two lists over the same array line up
element by element. A path without `[*]` and `[TypeName]` always gives one value, never a list.

In a table and in CSV a list is written as its values in their text form, separated by `, ` in
brackets, e.g. `[250.5, 1200]` or `[]`. The values are not quoted, so a string holding `, ` or
`]` reads ambiguously there; scripts use JSON, where a list is an array of the values, each typed
as a single value would be.

A path reaches at most 65,536 array elements in one Record (`MAX_PATH_VALUES`), counted over all
its brackets; `[n]` counts one element and `[*]` and `[TypeName]` every element of their array.
A path that would reach more has the error value `error: the path reaches more than 65536 array
elements` in its cell. A comparison over it is unknown, unless a value the path reached before
the limit already holds.

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

- An integer value beyond every integer type (more than about 39 digits) is an error. A value
  outside the range of the field's own type is no error: `size8 = 300` just never holds, and
  `count > -1` always holds for an unsigned field.
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

The type that counts is the one of the scalar the path ends at, e.g. `Double` for
`Components[Weapon].damage`; the message names the whole path. Without `from`, a comparison is
an error only when the path resolves in no type to a scalar that can be compared with the value;
a Record whose type resolves it otherwise, or not at all, has no value for it.

### Several values

A comparison over a path through `[*]` or `[TypeName]` holds when **any** of the values the path
reaches holds: `Components[Weapon].damage > 1000` keeps a ship with at least one weapon above
1000. It is false when every value compares false, also when the path reaches no value at all,
and unknown when none holds but one is unknown, e.g. null. So `not Components[*].name = "x"`
keeps the Records where no component is named `x`, and a condition that has to hold for **all**
values is written as the negation of its opposite: `not Components[Weapon].damage <= 1000` keeps
the ships whose every weapon does more than 1000 damage, ships without weapons included.

### Missing values

A comparison is unknown when the Record has no value for its path: its type does not resolve the
path (without `from`), the path ends at a type the value cannot be compared with, reaches `null`
through a null pointer or an index out of range, or the value cannot be read. An unknown
comparison never matches, and the conditions combine as in SQL:
`not` of unknown is unknown, `false and unknown` is false, `true or unknown` is true, and any
other combination with unknown is unknown. A Record is kept only when the whole condition is
true, so `x = 1` and `not x = 1` together never select a Record without a value for `x`.

A Record whose layout or instance cannot be read is different: no value of it can be read, so
the condition cannot be decided, and the Record is reported as failed rather than silently
dropped, as for `select`.

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
space between tokens, `, ` between paths, no spaces around `.` and inside or before brackets,
indices without leading zeros, backticks only where a name needs them, numbers as written,
strings with each quote doubled, and parentheses only where precedence needs them, e.g.
``from Ship select Components[Weapon].damage, tags[0], `a b`[*]``. Parsing the canonical form gives the same Query.
