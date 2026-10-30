# A dangling Reference stays an error value and carries the GUID of the missing Record

Decided for issue #885. Builds on ADR-0004 (Record XML with References) and ADR-0124 (the Record
header of Record XML and JSON).

## Context

A Reference whose GUID no Record has is written in Record JSON, Record XML and Query JSON as an
error value whose only content is the English reason, e.g.
`{"error": "the Reference refers to the Record 3cf6753c-…, which does not exist"}`. The LIVE 4.10.2
DataForge has 880 such values in 582 Records, and they are all the error values of a full export.
Consumers need the GUID itself (SCMDB lists the one of a crafting blueprint as
`productEntityClass`), and the reason is documented as text for people, not a stable code. The
schema `schemas/record-json.schema.json` allows an error object with `error` and nothing else.

## Decision

- **A dangling Reference stays an error value.** The value cannot be resolved to a Record, so it
  keeps the `error` key with the same reason, counts in `error_values` and `records_with_errors`,
  and every consumer that reads `error` keeps working.
- **It gains the GUID of the missing Record, in lower case, under `guid`:**
  - Record JSON and Query JSON: `{"error": "…", "guid": "3cf6753c-…"}`, `guid` after `error`.
  - Record XML: `<p4kx:error field="…" reason="…" guid="…"/>`, and the same attribute on an
    error element in an array.
  No other error value has `guid`, so its presence is what marks a dangling Reference.
- **The schema gains an optional `guid` on the error object.** Exports written before keep
  validating, and an error object with other keys still fails.
- **The declared type of the field is not written.** The value of a dangling Reference holds only
  the GUID; the struct a Reference field declares is a fact of the schema, the same for every
  Record of the type, and reading it would tie the lifetime of the DataForge to `ValueError`,
  which is a plain `Copy` error today.
- **The summary counts stay as they are.** A dangling Reference is still a value that is not
  there, and `p4kx dcb refs --unresolved` already lists the dangling References by target GUID.
  A separate count in the summaries of the export, the CLI and the window is a follow-up.

## Considered

- **A Reference object with `"missing": true`** (`{"guid": "…", "missing": true}`): reads like a
  link, but a consumer that reads `path` and `type` of every Reference object would meet an object
  without them, and the value would stop being an error, changing every count and every consumer
  that checks for `error`.
- **A stable error code for every error value** (`"kind": "dangling_reference"`): useful beyond
  this case, but a larger contract over every kind of error value; the `guid` key alone answers
  the need without fixing names for the others.
