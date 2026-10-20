# SQL-like Query syntax with subtype selection

A Query reads like SQL: `from <Type>` and `at "<glob>"` select Records, at least one of them is
given, and `select` lists dot paths as columns, e.g.
`from EntityClassDefinition at "*/spaceships/*" select Size, Mass`. The later syntax fits into
this shape: `where` filters follow `select`, and `[n]`, `[*]`, `[TypeName]` and `->` are parts of
a dot path. `from <Type>` selects the Records of that struct and of every struct deriving from
it. The glob follows the rules of a Filter, type and field names are case-sensitive with a
suggestion for a near miss, keywords ignore case, and rows are ordered by record path and then
by Record definition order. `referencing "<GUID or record path>"` selects the Records that hold a
Reference to the named Record ([ADR-0029](0029-references-are-indexed-once-per-archive-from-record-trees.md)). The grammar is defined in [docs/query.md](../query.md). Decided by
the owner.

## Considered Options

- Selector first with keywords after (`EntityClassDefinition "*/spaceships/*" select Size`):
  shorter, but the optional parts are positional and harder to keep unambiguous.
- A path expression in the style of jq or StarBreaker (`EntityClassDefinition["*"].{Size}`):
  compact, but `[...]` collides with array indices and component selectors.
- `from <Type>` matching the exact struct only: simpler, but parent types such as a common item
  base would select nothing useful.
