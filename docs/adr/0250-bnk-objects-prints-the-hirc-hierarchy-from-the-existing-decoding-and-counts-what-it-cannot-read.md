# `bnk objects` prints the HIRC hierarchy from the existing decoding and counts what it cannot read

Decided for issue #860, on top of ADR-0115 and ADR-0158 (the `HIRC` decoding) and in the manner of
ADR-0133 (unsupported is not corrupt).

## Decision

- **One parser.** `p4kx bnk objects` prints the objects `p4kx-audio` already decodes. No second
  reader of the `HIRC` section exists. `p4kx-audio` gains only accessors on `HircObject`:
  `layout()`, `parent_id()` and `children()`, plus `HircKind::slug()` and
  `MusicPlaylist::item_depths()`.
- **Four layouts, counted over the whole bank.** `HircLayout` tells how far an object is read:
  `Decoded`, `Undecoded` (a type with no decoder, e.g. actor-mixers, buses and states),
  `Unsupported` (a decoded type in a bank version whose layout is unknown, either
  `UnsupportedObjectLayout` or a body kept raw because of the version) and `Malformed` (the body
  does not hold its fields). The summary counts all four for the whole bank, whatever `--type`,
  `--id` and the page select, so a filtered view still shows what was not read. None of them fails
  the command: the exit code stays 0 for a readable bank.
- **Bounded output.** Objects come in bank order, `--limit` (1 to 10 000, default 100) at a time
  from `--offset`; the summary gives the next offset. The size of one object's listing is bounded
  by its body, which the parser already validated against the section; nothing is allocated from a
  declared size in the new code.
- **Parent 0 is no parent.** A node at the top has parent ID 0 in the bank; the output shows it
  as none (`-` in text, `null` in JSON), as it does for objects without a parent field.
- **A music track's children are its sources' media IDs**, not object IDs, because a source is
  not an object of the section.
- **The JSON shape** is documented in `docs/cli-json.md` and fixed by a snapshot test; the command
  has no JSON Schema file, like `bnk list`.

## Consequences

- The playlist loop fields are printed as the bank stores them (`loops`, `loop_min`,
  `loop_max`); the output does not interpret how they combine.
- Objects of banks other than version 154 are listed as `unsupported`, so a newer bank lists its
  objects with type, ID and status but no parent, children or detail.
