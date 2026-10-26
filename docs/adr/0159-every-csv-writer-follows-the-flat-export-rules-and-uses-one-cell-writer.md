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
  What counts as text is decided by the writer that knows the column. `dcb query` neutralises
  string, locale and enum values, the headings and the record path; `dcb fields` the path and the
  kinds; `loc export` the key and the text. The flat value export keeps its own choice of
  ADR-0112 and does not neutralise the path, name and record-path cells.
- `--csv-raw` on `dcb query`, `dcb fields` and `loc export` writes the bytes these commands wrote
  before: no `'`, and no byte order mark for `dcb query` and `dcb fields`; `loc export` always
  wrote the mark and keeps it. With another format it is invalid input (exit code 2). The library
  has `ResultFormat::CsvRaw` and `TableFormat::CsvRaw` for it. The desktop app writes the default
  CSV.
- This breaks scripts that read or compare the default CSV. Before, `dcb query` and `dcb fields`
  wrote no byte order mark and no `'`, and `loc export` wrote the mark and no `'`. Now all three
  start with the mark, and a text that starts with one of the characters above gets the `'`. A
  script removes the `'` and the mark, or uses `--csv-raw`.

## Considered Options

- Neutralise only in the writers that exist today and leave the Query CSV unchanged: the Query
  result holds free text of the game data, which is the case the neutralisation is for.
- Neutralise every cell: a number such as `-5` would turn into text.
- A separate CSV crate: the writer is 20 lines, and `p4kx-dataforge` is the lowest layer that all
  writers already depend on (ADR-0003).
