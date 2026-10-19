# DataForge format notes

What `p4kx-dataforge` assumes about details of `Data\Game2.dcb` that no tool documents, what it
tolerates when they drift, and what the installed Channels show. The reader reads versions 6 and
8; every byte is untrusted, so a deviation is a typed error, an error value at its place or an
issue, never a panic and never silently wrong data (ADR-0004).

The findings below come from the real-Archive canary
(`crates/p4kx-extraction/tests/real_archive/dataforge_canary.rs`, run with `--ignored` and
`P4KX_DATA_P4K`, and `P4KX_DATA_P4K_B` for a second Archive), which prints `DataForge::survey` and
asserts what is listed as an invariant. Reference tools are
[unforge](https://github.com/dolkensp/unforge) and StarBreaker (both MIT), read for comparison only.

## Tolerated and reported

`DataForge::issues` lists the first 1024 deviations the reader tolerated in table order, and
`DataForge::issue_count` counts all of them. A database without drift has none.

| Detail | Reader | Issue |
| --- | --- | --- |
| Bits above the low byte of a property's conversion type | Masked with `& 0xFF`, as unforge does. A flag in the high byte no longer turns a single value into an 8-byte array slot, which would fail the layout of the whole struct. | `ConversionFlags`, once per property |
| Record name offset `0xFFFFFFFF` | `Record::name` is empty; the Record builds. StarBreaker resolves the offset to the empty string too. | `RecordWithoutName` |
| Record tag offset `0xFFFFFFFF` (version 8) | `Record::tag` is `None`, as for version 6. StarBreaker omits the tag. | `RecordWithoutTag` |

Any other string offset that is not the start of a terminated UTF-8 string stays an error:
`InvalidString` for a name, a record path or a struct name, and an error value for a field.

## Kept strict

- **Pointer instance index.** A pointer is two little-endian `u32`: the struct and the instance.
  unforge reads `u32 + u16 + u16 padding`, which cannot address the instances beyond 65535 that
  the installed Channels have (see below). The reader keeps the full
  `u32`. A pointer whose target does not exist is `InvalidPointer` at its field, however the index
  came about, so a changed pointer layout cannot yield a wrong target silently.
- **Instance data fills the input exactly.** unforge only asserts it. A longer or shorter tail
  is more likely a truncated or extended file than a harmless variation, so `open` fails with
  `InstanceSize`.
- **Struct layout.** The fields of a struct must fill its declared size exactly (`StructLayout`).

## Unknown fields

- The two header words after the version (offsets 0x08 and 0x0C), which unforge and StarBreaker
  skip, are reported by `DataForge::survey` as `header_words`.
- The last 16 bits of a Record row, which unforge calls `RecordSize`, are not read for anything;
  the survey compares them with the size of the Record's struct (`TrailingSurvey`).

## Findings on the installed Channels

| | LIVE | PTU | EPTU | HOTFIX | TECH-PREVIEW |
| --- | --- | --- | --- | --- | --- |
| DataForge version | 8 | 8 | 8 | 8 | 8 |
| Records | 117,709 | 117,231 | 116,863 | 117,709 | 120,348 |
| Header words 0x08 / 0x0C | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 |
| Records whose trailing `u16` equals the struct size | all | all | all | all | all |
| Records with name or tag offset `0xFFFFFFFF` | 0 | 0 | 0 | 0 | 0 |
| Properties with conversion flags | 0 | 0 | 0 | 0 | 0 |
| Null pointers (both words `0xFFFFFFFF`) | 4,486,071 | 4,429,410 | 4,410,658 | 4,486,071 | 4,614,257 |
| Pointers to an instance below 65536 | 3,208,390 | 3,185,105 | 3,172,969 | 3,208,390 | 3,332,112 |
| Pointers to an instance at or above 65536 | 659,270 | 635,441 | 625,245 | 659,270 | 780,894 |
| Pointers that name no instance | 0 | 0 | 0 | 0 | 0 |
The pointer row corrects the review the canary was written from: the high 16 bits of the
instance index are not always zero. Several hundred thousand pointers address instances beyond
65535, so masking or truncating the index to 16 bits, as unforge does, would read wrong data.
Because of that the canary asserts that every pointer is null or names an instance, not a zero
high half.

The canary asserts for every Channel it is run against: no dangling pointer, no conversion flag,
every struct layout readable, both header words zero and the trailing `u16` of every Record row
equal to its struct size. A failure means the format drifted and the notes above need a look.
