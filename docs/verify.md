# Verifying Entry checksums

`p4kx verify <archive> [--filter <filter>]` reads every Entry (or those the Filter matches),
decrypts and decompresses it, and compares the CRC-32C of the decoded bytes with the one the
Archive directory states. `p4kx extract --verify` does the same for each Entry it writes: an
Entry that does not match is not put in place and is counted as a checksum mismatch.
`p4kx socpak verify` and `socpak extract --verify` check a container against the plain CRC-32 of
its directory. Reading is unchecked unless one of these is asked for (ADR-0026).

An Entry that the Skip Conflict policy leaves alone because its file exists is never read, so it is
not verified; under IfChanged an unchanged file matched the stored CRC-32C when it was compared.
A texture exported as PNG or EXR reads its header Entry to the end through the verified reader, so
a mismatch there is reported although the Conversion parses only part of it.

The checksum is computed on the chunks that leave the reader, so a run reads and decodes the
Archive once, holds at most one 1 MiB chunk per worker and writes nothing. The CRC-32C comes from the
`crc32c` crate (hardware instructions on x86-64 and AArch64, portable fallback), the CRC-32 from
`crc32fast`.

## Exit codes and output

| Result | Exit code |
|--------|-----------|
| every Entry matched | 0 |
| a mismatch, or an Entry that cannot be read | 1 |
| a Filter that matches no Entry | 2 |
| the Archive cannot be opened | 3 |
| the Archive changed on disk during the run | 4 |

Text output prints `mismatch <path>: <computed> computed, <stored> stored` and
`failed <path>: <reason>` to stderr, then a summary line on stdout. `--json` prints the document
described in [cli-json.md](cli-json.md#p4kx-verify-archive---filter-filter---json).

## Run time

| Archive | Entries | Size | Machine | `p4kx verify` | Result |
|---------|---------|------|---------|---------------|--------|
| LIVE `Data.p4k` | 1,379,308 | 162 GB (file size of the Archive) | Ryzen 9 9950X3D, 16 cores, 32 threads | 50 to 57 s (three runs, release build) | 0 mismatches, 0 failed |

For scale: a review of an external extractor that checked only sizes measured about 27 minutes for
a full scan of LIVE 4.10 (1,362,047 Entries). The runs above were bound by the disk at about
3 GB/s: the 150.9 GiB Archive does not fit into the 94 GiB of RAM, so the file cache could not hold
it between runs.

## Real-Archive tests

`cargo test --release --workspace --locked -- --ignored` (see CLAUDE.md) includes
`verified_reads_of_every_compression_and_encryption_state_find_no_mismatch` in `p4kx-archive`,
which reads a sample of each compression and encryption kind with `Archive::open_entry_verified`,
and `a_sample_of_a_real_archive_verifies_without_a_mismatch` in `p4kx-extraction`, which runs
`verify` over every 997th Entry.
