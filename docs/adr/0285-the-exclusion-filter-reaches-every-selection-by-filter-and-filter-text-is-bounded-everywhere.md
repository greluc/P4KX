# The exclusion Filter reaches every selection by Filter, and Filter text is bounded everywhere

Decided for issue #887. Extends ADR-0256 and replaces two of its statements: that the changes of a
Comparison side are not narrowed by the exclusion field, and that the callers other than extract,
search, socpak extract and the GUI Extraction keep `Filter::new`.

## Decision

- **Comparison changes.** The Extraction dialog of the GUI shows the exclusion field for the
  changes of a Comparison side too. The changes the Comparison's own Filter selects are narrowed
  by it, with the rules and the texture rule of ADR-0256: a texture whose header an exclusion
  Filter matches is left out with its parts; a part it matches alone is left out of the written
  Entries, counted in the summary with a Conversion, and still read to build the texture. The
  rule is `Exclusion::leave_out`, the same code the Archive scopes use.
- **`--exclude` on the other commands that select Entries by Filter.** `p4kx grep`, `p4kx verify`,
  `p4kx socpak verify`, `p4kx socpak extract` and `p4kx convert` take a repeatable `--exclude`
  with the rules of `p4kx search`. `grep` applies the exclusion to what its Filter applies to:
  the record path of a Record, the key of a localisation text and the path of an Entry (also
  without `--filter`, where it narrows the default selection). `p4kx convert` applies it to the
  relative paths of the input folder, with the texture rule and the stderr note of
  `p4kx extract --convert`.
- **Without `--filter`.** `grep`, `verify`, `convert` and `socpak extract` select everything
  without a Filter, so `--exclude` alone is meaningful there and is not a usage error; `p4kx
  extract` keeps ADR-0256's rule that `--exclude` needs `--filter`, as it has no default
  selection. Where a selection is checked for being empty (`verify`, `convert`, `socpak
  extract`), a pair that leaves nothing is invalid input (exit 2) with the messages of ADR-0256;
  with no include Filter the message names `*`. `grep` reports no hits instead, as it does for a
  Filter that matches nothing.
- **Bounded Filter text everywhere.** Every entry point that takes Filter text from a person or
  a client uses `Filter::parse` (at most 1024 bytes and 32 wildcards) and reports a refusal as
  invalid input: the CLI (`grep`, `verify`, `compare`, `catalog`, `ships`, `dcb export`,
  `dcb fields`, `convert`, `socpak`) with exit 2, the MCP tools `search_entries`,
  `search_records` and the definition listings as a tool error. In the GUI a search or filter
  field whose text is refused turns red, shows the reason on hover and selects nothing; the
  Record export is not offered with a refused Filter. `Filter::new` remains for fixed text of the
  program and for tests.

## Consequences

- `SearchOptions` of `p4kx-search` carries an `Exclusion`. `convert_folder` of `p4kx-extraction`
  takes the selection made by `FolderSource::select` (include Filter and `Exclusion`) instead of
  a Filter; the GUI passes no Filter and an empty exclusion. `FolderSource::count_matching` is
  gone, as `select` also reports how many files matched.
- `Filter::check` validates the limits without building the Filter, for the GUI to call every
  frame; `Filter::parse` uses it.
- A part of a texture that only an exclusion Filter names is left out without its header being
  looked up, as in ADR-0256.
- `socpak list` has no Filter and so no exclusion.
