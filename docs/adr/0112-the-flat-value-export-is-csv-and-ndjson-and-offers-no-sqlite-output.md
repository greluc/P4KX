# The flat value export is CSV and NDJSON and offers no SQLite output

Decided for issue #268, on top of ADR-0004 (the canonical Record tree) and ADR-0008 (record files).

## Decision

- The flat value export (`docs/record-flat.md`) writes one file of one row per leaf value, as CSV
  or as NDJSON. It is a form of the Record export of `p4kx-export` (`RecordForm::FlatCsv`,
  `FlatCsvRaw`, `FlatNdjson`), built from the Record tree by `FlatValues` of `p4kx-dataforge`, and
  it streams through the same batches and the same temporary file as `records.ndjson`.
- No SQLite output is offered. Both files load into SQLite with its own tools (`.import --csv`, or
  a few lines of script for NDJSON), which gives a database of the user's own schema and indexes
  at no cost to P4KX. A SQLite writer would add a native dependency (`rusqlite` with a bundled
  C library) to every build, a schema P4KX would then have to keep stable, and the localisation
  and Reference tables the user can build from the columns already there.
- Should an SQLite output come later, `p4kx-export` owns it: it is a form of the Record export
  and needs the services the export already uses, which the format crate `p4kx-dataforge` must not
  depend on (ADR-0003). The new ADR then decides the schema and the dependency.
- The CSV is made for spreadsheets by default: UTF-8 with a byte order mark so that Excel reads
  the encoding right, CRLF rows after RFC 4180, and a leading `'` before a `string`, `locale` or
  `enum` value and a `text` that starts with `=`, `+`, `-`, `@`, a tab or a carriage return, so that
  a spreadsheet does not run it as a formula. Numbers are never changed. `--csv-raw` writes the
  exact bytes of the values, without the byte order mark and without the `'`, for scripts and
  databases. NDJSON is never changed.
- A `reference` row carries the GUID of the target in `value` and its record path in an eighth
  column `target_path`; the location a weak pointer has inside the Record is not written.
- Empty values get no row: an empty string, localisation key or enum option, a null pointer or
  Reference, an empty array, and a value that cannot be read. A Reference to a Record that does
  not exist is the exception: it keeps a `reference` row with the GUID it names and an empty
  `target_path`, so the link is not lost. A real `0` has a row.
- A field name that starts with `$` or repeats an earlier name of its struct is written `$<n>` in
  the field path, `n` being its position among the fields of the struct, as in Record JSON.

## Consequences

- Every localisation key of the default CSV starts with `'@`, because every key starts with `@`.
  A script that reads the default CSV removes the `'`, or uses `--csv-raw`.
- A flat export cannot be combined with `--combined` or `--tags`, and the desktop app offers the
  three flat forms next to JSON and NDJSON.

## Considered Options

- A `value` column that holds the target record path for a Reference: the GUID is the stable
  identifier, and a record path can change between builds.
- Neutralising every cell: it would turn `-5` into text, and the columns that hold no free text
  cannot start a formula.
- A SQLite file with the Records, paths, values, References and localisation (the issue's
  question): rejected above; the cost is a dependency and a schema, the gain a convenience that
  the two files already give.
