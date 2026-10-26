# The CHF diff pairs fields by hash and reports paths in the JSON keys

Decided for issue #315. Builds on ADR-0014 and ADR-0178.

## Decision

- **The diff compares the typed Character, not the bytes or the JSON text.** `diff_chf` in
  `p4kx-character` walks two `Chf` values and returns `ChfChange { path, raw_path, old, new }`.
  `p4kx chf diff` prints them as text or, with `--format json` or the global `--json`, as one line.
- **Paths use the keys of the CHF JSON** (`dna.face_parts.neck.blends[2].value`), without the
  leading `character.`. A hash segment is spelled with its name from the `NameTable` (ADR-0178)
  when one is known, and `raw_path` keeps `0x` hashes, so scripts do not depend on the names.
  Parameters, like ports and materials, are named by their hash, never by their index.
- **Lists are paired by hash, not by index:** item port children, materials, sub-materials, floats
  and colours by their hash, textures by slot. Items with equal content are paired first, the rest
  of a repeated hash in order, and a repeat is shown as `#n`. An inserted entry then yields one
  change, not a shifted cascade. Face parts and blends stay positional: their order is the layout.
- **A different order is reported as an `order` change** at the list, with the labels of the
  paired items in the old and in the new order, because the files differ even though every field
  matches and the tool does not know that the game ignores the order.
- **The container is not compared except `flags`.** The zstd frame and padding are a coding of the
  body; two files of one character compressed differently are not different characters.
  `identical` in the JSON means: no change listed.
- **Exit code 1 when the files differ, 0 when they do not,** as `compare`, `compare --schema` and
  `compare --diff` do (`DIFFERENCES`); a file that is no CHF exits with 3.
- **Names of both files:** `--archive` opens the Archive once and merges the names of the items
  both characters wear (`NameTable::merge`; existing names win).

## Considered Options

- Diffing the two `to-json` texts line by line: it reports index-shifted noise for any inserted
  port or material and cannot name a field.
- Exit code 0 for a difference, with `identical` in the JSON: it breaks the convention of the other
  diff commands, which scripts rely on.
