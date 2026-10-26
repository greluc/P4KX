# Included object records have the length their layout word gives

Decided for issue #516, which follows up the Interior tracer (ADR-0081). Evidence: every `.soc`
file of every socpak of the LIVE Archive of build 4.10.2 (9860 files, 7678 `IncludedObjects`
chunks, 3.2 million placed models); the PTU Archive reads the same.

## Decision

ADR-0081 read the object table of an `IncludedObjects` chunk (type `0x0010`) as records of 208
bytes. That holds for most tables only. The first of the seven header words after the palette
paths is a layout word, and it fixes the length of a record of a placed model (kind 1):

| Layout word | Chunks | Record length |
| --- | --- | --- |
| 8, 9, 10 | 476 | 176 |
| 12 | 2 | 184 |
| 13 | 375 | 192 |
| 14 | 211 | 200 |
| 15 | 6614 | 208 |

The transform (`f64`, 3×4) at 64, the model index at 60 and the palette at 172 sit at the same
offsets in every layout. The 14 % of the chunks with another layout than 15 were read with the
wrong stride before: the first record was right, the next ones were garbage and the list ended at
the first bogus kind. A layout word that is not in the table makes the chunk unreadable
(`IncludedObjectsError::RecordLayout`) instead of being read with a guessed stride, unless the
table is empty.

Records of kind 16 are 136 bytes in every layout (408 thousand records); they place no model and
are skipped and counted (`IncludedObjects::skipped`). The scdatatools notes (168 bytes, 16 more
when `unknown3` is 0) match no table: no layout word gives 168.

Kinds 9 (5 chunks) and 26 (7 chunks) occur with no known length; they still end the list and are
reported as `unsupported_objects`. No other kind occurs.

A placement whose 3×3 part is not a rotation with a scale along the axes counts as `invalid`:
a row with zero or non-finite length, a cosine above 0.05 between two rows, or a determinant
below half the product of the row lengths. The LIVE Archive has no such placement (the cosine is
below 1e-14 in 3.2 million), so the check only guards against a wrong layout. The 601 invalid
objects of the Archive name a model the table does not list.

Containers declare their box in `minBounds` and `maxBounds` of the description
(`ObjectContainer::bounds`). The models and props of the Mantis body and of the Idris front
(`base_int_mdfront_main`, more than 1000 models) lie inside it to 0.01 m.

`ChildObjectContainers` do occur in LIVE socpaks: 36874 children in 3233 descriptions (not in
ship interiors), all with numeric `pos` and `rot`; `clioislands_arena` places its child at the
declared position.

## Considered Options

- Fall back to 208 bytes for a layout word that is not known: it keeps the old behaviour and
  silently misreads the table, which is what this issue removes.
- Derive the record length from the declared table length and a record count: the header has no
  record count (its first word is the layout word), so it cannot be derived.
