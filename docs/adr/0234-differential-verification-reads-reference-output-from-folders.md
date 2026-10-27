# Differential verification reads reference output from folders

Decided for issue #264. Evidence: LIVE 4.10 against unp4k and unforge 4.0.87 and StarBreaker CLI 0.3.2.

## Decision

- P4KX is compared with unp4k, unforge and StarBreaker by `#[ignore]`d tests in
  `crates/p4kx-extraction/tests/reference/`. A test reads a folder of output the owner produced
  beforehand, named by `P4KX_REF_UNP4K`, `P4KX_REF_UNFORGE` or `P4KX_REF_STARBREAKER`, and returns
  early when its variable is unset. No test runs a foreign tool, and CI downloads nothing.
- The comparison is structural, not textual: bytes block by block for Entries, element trees
  (tag, attributes in order, trimmed text, children) for `CryXml`, and values per Record (field
  name, kind, value) for DataForge. A number is compared at the precision of the field P4KX read:
  a single is compared as `f32`, `-0.0` equals `0.0`, an integer exactly.
- A difference fails the test unless it is on a documented allow-list, each entry with its reason
  and counted in the summary. The allow-list is the known behaviour of the reference tools:
  trailing zero bytes unp4k trims from encrypted Entries, `BUGGED` attributes and Latin-1
  decoding of unforge, weak pointers, unresolved References and lenient booleans of StarBreaker.
  A difference beside an allowed one still fails.
- Every comparison has a mutation self-test on synthetic data that changes a copy of the
  reference by +1 and by the smallest step, swaps texts, flips booleans, deletes and invents
  rows, fields, attributes and children, and requires each change to be detected.
- No reference output and no game data are committed.
