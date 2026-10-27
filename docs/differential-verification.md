# Differential verification against unp4k, unforge and StarBreaker

The real-Archive tests check plausibility. The tests in `crates/p4kx-extraction/tests/reference/`
check P4KX against three independent tools, so that an error both of P4KX and of its own tests
cannot hide ([ADR-0234](adr/0234-differential-verification-reads-reference-output-from-folders.md)).
Each test is `#[ignore]`d, reads a folder of reference output the owner produced beforehand and
returns early with a note when its variable is unset. No test runs a foreign tool, CI downloads
nothing, and no reference output or game data is committed.

| Variable | Folder | Test | Comparison |
|----------|--------|------|------------|
| `P4KX_REF_UNP4K` | extracted by unp4k, paths as in the Archive | `unp4k::every_entry_unp4k_extracted_is_byte_identical` | bytes of every file against the decoded Entry, streamed in 64 KiB blocks; counts per compression and encryption kind |
| `P4KX_REF_UNFORGE` | `CryXml` converted by unforge, `<stem>.xml` next to its source | `unforge::every_cryxml_tree_unforge_converted_is_identical` | element trees: tag, attributes in order, trimmed text, children |
| `P4KX_REF_STARBREAKER` | `starbreaker dcb extract --format json` | `starbreaker::every_record_starbreaker_wrote_has_the_same_values` | per Record: GUID, name, tag, then every field by name, kind and value |

The real Archive and its key come as for every real-Archive test (`P4KX_DATA_P4K`, the stored
key). `tools\real-archive.ps1` sets the three reference variables when the folders exist. Run with
`cargo test --release -p p4kx-extraction --test reference --locked -- --ignored --nocapture
--test-threads 1`.

## Producing the reference folders

Use the official releases of the tools (unp4k and unforge 4.0.87 from dolkensp/unp4k, StarBreaker
CLI 0.3.2 from diogotr7/StarBreaker) and the same `Data.p4k` as the test. Extract only what the
comparison needs; the whole Archive is not required.

```text
unp4k.exe <Data.p4k> *.dcb              (in the folder for P4KX_REF_UNP4K)
unp4k.exe <Data.p4k> *.chrparams
unp4k.exe <Data.p4k> *.cdf
unforge.cli.exe <copy of the .chrparams and .cdf files>
starbreaker.exe dcb extract --dcb Data\Game2.dcb --format json --output <folder>
```

unp4k must write the raw bytes; the folder must keep the Archive paths below its root. The folder
of unforge keeps the sources next to the converted `.xml` files; only the `.xml` files are
compared, each against the Entry of its source. StarBreaker exports the main Record of each file
unless `--all` is given; Records without a file are counted as missing in the reference and do not
fail the test.

## Output and failing

Each test prints one summary line: compared, equal, different, allowed, missing in the reference
and missing in the Archive, then one line per allowed reason and at most 25 examples of
differences (400 characters each). The test fails on any difference that is not on the
allow-list and on any reference file with no Entry or Record in the Archive.

The allow-list holds the known behaviour of the reference tools; each use is counted under its
reason:

- unp4k: trailing zero bytes of an encrypted Entry, which unp4k trims after decryption.
- unforge: an attribute value `BUGGED`, and text that is P4KX's UTF-8 read byte by byte.
- StarBreaker: `null` for a weak pointer in a Record whose weak pointers sit in derived types
  (#262); `null` for a value P4KX reports as an error, as an unknown Reference GUID; a boolean
  that P4KX reports as an error and StarBreaker reads as true.

A difference beside an allowed one still fails. Numbers are compared at the precision of the
field P4KX read: a single as `f32`, a double exactly, an integer exactly, `-0.0` equal to `0.0`.
Fields are compared after sorting by name, ignoring case. A Record of the same file that
StarBreaker embeds where it is referenced is compared as a Reference by GUID, not by content.
`crates/p4kx-extraction/tests/reference/starbreaker_json.rs` is the one place that encodes the
JSON shape of StarBreaker.

## Mutation self-tests

Without any variable, `mutation_bytes`, `mutation_xml` and `mutation_value` change a copy of
synthetic reference data and require every change to be detected: a byte changed, deleted or
appended at any block; a tag, attribute, text or child renamed, changed, swapped, deleted or
invented; an integer plus one, a single and a double by the smallest step, a boolean flipped,
texts swapped, an enum option, GUID or Reference changed, a value of another kind, an array
element deleted or invented, a field deleted or invented, and the Record name, GUID and tag
changed. They also check that `-0.0`, the short text of a single and whitespace around text are no
difference, and that each allow-list entry applies only to its case.

## First run on LIVE 4.10, 2026-10-27

Reference production (one machine, tools 4.0.87 and 0.3.2):

| Step | Time | Result |
|------|------|--------|
| unp4k `*.dcb` | 4 s | `Data\Game2.dcb`, 336,772,966 bytes |
| unp4k `*.chrparams` and `*.cdf` | 10.6 s | 4,151 and 1,690 files |
| unforge | 23.9 s | 5,728 of 5,841 files converted |
| StarBreaker `dcb extract --format json` | 34.5 s | 62,736 files, 3,069 MB, main Records only |

P4KX against it (release build):

| Test | Compared | Equal | Different | Allowed | Time |
|------|----------|-------|-----------|---------|------|
| unp4k bytes | 5,842 | 5,842 | 0 | 0 | 1.1 s |
| unforge trees | 5,728 | 5,728 | 0 | 0 | 0.8 s |
| StarBreaker values | 62,736 | 62,162 | 0 | 574 | 3.3 s |

- The 5,842 Entries are all Zstandard and unencrypted. The encrypted allowance was not exercised;
  it is covered by the mutation self-test only.
- The StarBreaker allowances: 534 values P4KX reports as errors that StarBreaker writes as
  `null`, and 40 weak pointers it writes as `null`.
- 54,973 of the 117,709 Records have no file in the reference: StarBreaker writes the main Record
  of each file only.
- Opening `Game2.dcb` (read, decrypt and decompress the Entry, open the DataForge) takes 0.30 s.
  Building the Record tree of all 117,709 Records takes 0.29 s on all cores, with none failing;
  that is the tree, not the writing of JSON files.
- The first comparison run found one error of the test itself, not of P4KX: StarBreaker embeds
  the Record of the same file where it is referenced, which the comparison now reads as a
  Reference.
