# Every CSV writer follows the flat export rules and uses one cell writer

Decided for issue #389, on top of ADR-0112 (the flat value export) and ADR-0049 (field
statistics).

## Decision

- `write_csv_cell` and `CSV_BYTE_ORDER_MARK` of `p4kx-dataforge` (`flat_cells.rs`) are the only code
  that quotes and neutralises a CSV cell. `dcb query` and the Query export of the desktop app
  (`ResultWriter`), `dcb fields`, `loc export` and the flat value export use it; no writer keeps a
  quoting function of its own.
- The rules are those of ADR-0112 for all of them: UTF-8 with a byte order mark, CRLF rows after
  RFC 4180, and a leading `'` before a text cell that starts with `=`, `+`, `-`, `@`, a tab or a
  carriage return. Numbers, booleans, GUIDs and counts are never changed, so `-0.25` stays a number.
  What counts as text is decided by the writer that knows the column: a string, locale or enum
  value, a heading, a path, a kind, a localisation key and a localised text.
- `--csv-raw` on `dcb query`, `dcb fields` and `loc export` writes the exact bytes: no byte order
  mark and no `'`. With another format it is invalid input (exit code 2). The library has
  `ResultFormat::CsvRaw` and `TableFormat::CsvRaw` for it. The desktop app writes the default CSV.
- The default CSV of `dcb query`, `dcb fields` and `loc export` changes: it starts with a byte order
  mark, and a text that starts with one of the characters above gets the `'`, a localisation key
  that starts with `@` included. A script that reads the default CSV removes the `'` and
  the mark, or uses `--csv-raw`.

## Considered Options

- Neutralise only in the writers that exist today and leave the Query CSV unchanged: the Query
  result holds free text of the game data, which is the case the neutralisation is for.
- Neutralise every cell: a number such as `-5` would turn into text.
- A separate CSV crate: the writer is 20 lines, and `p4kx-dataforge` is the lowest layer that all
  writers already depend on (ADR-0003).
