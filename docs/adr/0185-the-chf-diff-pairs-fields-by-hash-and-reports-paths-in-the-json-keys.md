# The CHF diff pairs fields by hash and reports paths in the JSON keys

Decided for issue #315. Builds on ADR-0014 and ADR-0178.

## Decision

- **The diff compares the typed Character, not the bytes or the JSON text.** `diff_chf` in
  `p4kx-character` walks two `Chf` values and returns `ChfChange { path, raw_path, old, new }`.
  `p4kx chf diff` prints them as text or, with `--format json` or the global `--json`, as one line.
- **Paths use the keys of the CHF JSON** (`dna.face_parts.neck.blends[2].value`), without the
  leading `character.`. A hash segment is spelled with its name from the `NameTable` (ADR-0178)
  when one is known, and `raw_path` keeps `0x` hashes, so scripts do not depend on the names.
- **Lists are paired by hash, not by index:** item port children, materials, sub-materials, floats
  and colours by their hash (the n-th repeat of a hash with the n-th, shown as `#n`), textures by
  slot. An inserted or reordered entry then yields one change, not a shifted cascade. Face parts
  and blends stay positional: their order is the layout.
- **The container is not compared except `flags`.** The zstd frame and padding are a coding of the
  body; two files of one character compressed differently are not different characters.
- **Exit code 0** for identical and different files alike; `identical` in the JSON says which.
- **Names of both files:** `--archive` opens the Archive once and merges the names of the items
  both characters wear (`NameTable::merge`).

## Considered Options

- Diffing the two `to-json` texts line by line: it reports index-shifted noise for any inserted
  port or material and cannot name a field.
- Exit code 1 for a difference, as `compare` does: here the list of differences is the requested
  result, not a failed run, and `identical` in the JSON already carries it.
