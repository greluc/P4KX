# An Archive without valid end records is rebuilt only on request, by a bounded scan

Decided for issue #293, split out of #282. Builds on ADR-0025 and ADR-0148.

## Decisions

- **Opt-in.** `Archive::open` and every default behaviour are unchanged: a missing or damaged
  end of central directory (EOCD), ZIP64 end record or central directory fails to open.
  `Archive::open_with(path, OpenOptions::new().recover(true))` and the global CLI flag
  `--recover` switch the recovery on. The GUI does not offer it.
- **When it runs.** Only after the normal read failed with `NotAnArchive`, `Corrupt` or
  `Truncated`, and only for a file that does not end in the v2 `Jiji` marker. I/O errors, the
  v2 layout and an unsupported layout version keep their errors. A scan that finds no record
  returns the error of the normal read, so the message names the real cause.
- **The scan.** One forward pass over the last `MAX_RECOVERY_SCAN` (4 GiB) bytes of the file in
  4 MiB reads, looking for the central header signature `PK\x01\x02`. At most
  `MAX_RECOVERED_ENTRIES` (2^24) Entries are admitted and at most 2^24 signature hits are
  examined, so the work is linear in the window and the memory is the Entry list plus one
  buffer. A record that is accepted is skipped as a whole; a hit that is rejected costs one byte
  of progress.
- **What counts as a record.** The header parses with the same code as the normal read, so
  extra blocks, hash blocks and `HashBlockSize` issues behave identically (ADR-0148); the local
  header at its offset lies completely before the record, carries a local signature and has the
  same name. The name test rejects the records of stored containers that the data area holds,
  whose offsets are relative to the container. Sizes that reach past the end of the file are
  kept as `EntryIssue::DataOutsideFile`.
- **Checksums.** Recovered Entries are verified exactly as before: CRC-32C against the record, the
  stated SHA-256 against the stored bytes. When no recovered record carries a CIG block the
  Archive is read as a plain ZIP and the checksum is the ZIP CRC-32, as without ZIP64 records.
- **Reporting.** `EntryIssue::Recovered` is listed first, at Entry 0, and counted; the CLI
  prints it with the other issues as a warning. Recovered Entries are in file order and
  Entries whose records are lost are missing; there is no way to know how many.
- **Not recovered.** The v2 layout has no signature to scan for.
