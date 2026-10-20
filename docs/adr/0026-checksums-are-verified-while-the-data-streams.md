# Checksums are verified while the data streams, on request

An Archive states a checksum for every Entry: the CRC-32C of the decoded data in `Data.p4k`, the
plain CRC-32 in a ZIP container. Reading an Entry checked only its size, so an Entry that is
corrupted, tampered with or decrypted wrongly (ADR-0025: an unknown encryption value is read as
encrypted) and still decodes to the right length went unnoticed.

`Archive::open_entry_verified` opens an Entry like `open_entry` and hashes the decoded bytes as
they leave the reader. At the byte that completes the Entry's size the checksums are compared; a
mismatch is `DataError::ChecksumMismatch { path, expected, actual }`, an `io::Error` of kind
`InvalidData` like every other decoding error, and the read that finds it yields none of its
bytes. Nothing is buffered and the data is not read twice. The verifier is
`Vec<Check>`, each `Check` an expected `Checksum` with its `ChecksumHasher`, and `Checksum` is
`#[non_exhaustive]`: a second kind of checksum, such as the SHA-256 of an Entry (#306), is a new
`Checksum` variant, a new `ChecksumHasher` state and one more `Check`, with the reader, the error
and every front-end unchanged.

## Decisions

- **Off by default.** Verification is opt-in: the issue asks that reading without it costs no
  extra time, and the CRC is a pass over every decoded byte. `p4kx verify` and `p4kx extract
  --verify` (also `p4kx socpak verify` and `socpak extract --verify`) switch it on;
  `ExtractOptions::verify` is the library switch. `open_entry` stays unchecked for every other
  reader (preview, Conversions that parse an Entry, the window).
- **Checked at the last byte, not at the end of the stream.** A caller that reads exactly the
  Entry's size with `read_exact` never asks for the end, so the comparison happens when the
  size is reached. An empty Entry is checked by its first read. A reader dropped before the end
  checks nothing, and a reader opened at an offset (`open_entry_at`) is never verified.
- **A mismatching Entry is not written.** An Extraction writes to a temporary file that replaces
  the destination only after the whole Entry was read; the mismatch is a failure of that Entry
  like any read error, so the temporary file is removed and an existing file is kept. The failure
  is `ExtractionError::ChecksumMismatch`, kept apart from `Read` so reports can count it
  (`ExtractionReport::checksum_mismatches`, `mismatched` in the JSON report). Every path that
  reads a whole Entry for a Conversion (CryXml, DataForge, sounds, soundbanks, the header and
  parts of a texture merge) opens it verified; a Conversion that only reads the start of an
  Entry checks nothing there.
- **`verify` is a service, not an Extraction.** `p4kx_extraction::verify` reads the Entries on
  the rayon pool into one 1 MiB chunk per worker and writes nothing. Per-Entry results are
  folded per worker and merged in Entry order, so memory grows with the number of failures, not
  with the 1.4 million Entries. Exit code 1 for a mismatch or an unreadable Entry, 4 when the
  Archive changed, like an Extraction.
- **CRC-32C from `crc32c`, CRC-32 from `crc32fast`.** Both are the newest stable releases and use
  the CPU's CRC and carry-less multiplication instructions with a portable fallback, so the
  check stays far below the cost of decompression and decryption. `crc32c` moves from the test
  dependencies to the dependencies of `p4kx-archive`, which now owns the hasher the
  `IfChanged` Conflict policy used privately.

## Considered Options

- Verify by default: the cost is a hash over every decoded byte of every read, which previews and
  Conversions do not need, and the issue rules it out.
- A second pass over the written file: doubles the reads of the largest Entries and cannot cover
  `verify`, which writes nothing.
- Compare only at end of stream: misses callers that stop at the declared size.
- Skip the mismatching Entry's file but keep it as `.bad`: an Entry that fails its checksum has
  no trusted content, and a leftover file would look like a result.
