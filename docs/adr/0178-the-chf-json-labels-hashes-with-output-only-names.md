# The CHF JSON labels hashes with output-only names

ADR-0014 keeps the raw `u32` hashes in the CHF JSON, and they stay the only authority. A reader
could not tell which port, material slot or parameter a hash stands for, so `p4kx chf to-json`
writes the name beside every hash it can resolve.

- **Keys:** `gender_hash_label`, `port_label`, `attachment_label` and `name_label` follow the hash
  they name in the same object. A hash that no name matches has no label; nothing is guessed.
- **Output only:** each type that carries a hash has a `label` field, serde-renamed to its key,
  read as a string and dropped, never written, and ignored by equality (`Label`, a unit type). So
  a labelled JSON gives the same bytes as the plain one, an edited hash with a stale label is
  written as the hash, a label in any other object and every other unknown field are still
  refused, and the errors of serde point at the line and column of the file as given.
- **Names:** `NameTable` resolves each field by its own function (`docs/chf-hashes.md`). The
  built-in table holds the names found by analysing the shipped Characters: the item ports, the
  seven material slots by the slot formula, the two rigs, `BodyColor`, `EyeColor` and a few
  parameters. With `--archive` the Archive adds the item ports of every item the Character wears
  (the `Ports` of its Entity Record), and the sub-material names and public parameters of the
  base material libraries it chooses. On the LIVE Archive of build 4.10.2 that names 2855 of 4345
  hashes of the 41 shipped CHF files, against 1043 for the built-in table; both item port hashes
  the built-in table lacked are named.
- **Summary:** `to-json` counts the hashes named and unresolved, and lists the distinct unresolved
  ones: on stderr when the JSON goes to stdout, after the conversion line with `--out`, and as
  `named`, `unresolved` and `unresolved_hashes` in the `--json` summary.
- **Bounds:** the table keeps at most 65536 names of a kind, none longer than 256 bytes, because
  the names come from the Archive.

The two hair colour parameters stay unlabelled: their hashes are inferred from the shipped
Characters, and no name is known for them.

## Considered Options

- Stripping the keys from the text before the typed parse: it gives the same result, but the error
  positions then refer to the stripped text, not to the file the user edited.
- A separate name file: the reader would have to join two files.
- Replacing the hash with the name: the hash would no longer be authoritative, and an unknown name
  would have to be hashed silently on import, which this tool refuses to do.
