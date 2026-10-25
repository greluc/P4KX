# A malformed hash block is an Entry issue, and a checksum mismatch is never replaced by a read error

Decided for issue #364, the follow-ups of the review of #362 (ADR-0034). Builds on ADR-0025,
ADR-0026 and ADR-0034.

## Decisions

- **A `0x5003` block that is not 36 bytes long is `EntryIssue::HashBlockSize`.** The block holds
  a 4-byte header and the 32-byte SHA-256. A block of another length states no digest
  (`Entry::sha256` is `None`) and is recorded with its stored length, header included, as ADR-0025
  does for the `0x5002` encryption block. An all-zero 32-byte digest is still "no digest" and not
  an issue: the synthetic fixtures use it.
- **The last of several `0x5003` blocks of one record decides**, whether it is well-formed or not.
  A later malformed block therefore replaces an earlier valid digest with none. The issue
  describes the malformed block, not the Entry: an Entry whose malformed block is followed by a
  valid one has a digest and still carries the issue. An Entry has at most one `HashBlockSize`
  issue.
- **The CRC-32C mismatch comes first and ends the check.** When the checksum of the decoded data
  already differs at the last byte, the stored bytes the SHA-256 would still have to read (AES
  padding, a compression trailer) are not read. A failure of that read can no longer replace the
  mismatch with an I/O error. The report shows the CRC mismatch only; the SHA-256 is compared
  when the CRC matches.
- **Key check without the SHA-256.** `Archive::verify_key` checks the decoded data against the
  CRC only, so a wrong key stays a wrong key whatever the stored digest says.
- **v2 digests stay unverified.** No v2 Archive is available to confirm that the 32 bytes after the
  encryption field are the SHA-256 of the stored bytes (ADR-0034). Verification is switched on by
  setting `sha256_checked` for the v2 records together with a real-Archive test; this is tracked
  in #645. `Entry::is_sha256_verified` tells whether a verified read checks the digest.
- **Encrypted Entries.** The keyed real-Archive test now asserts that the encrypted samples state a
  SHA-256 and that their verified reads passed. The measurement of ADR-0034 for encrypted Entries
  stays the ciphertext hash; a key-free `p4kx verify` mode is not added, since an Entry cannot be
  opened without the key and the stored bytes are not a public reader.

## Cost

`bench_verification_cost` in `p4kx-archive` and `bench_verify` in `p4kx-extraction` (real
Archive, LIVE, Ryzen 9 9950X3D, release build, warm file cache) measure what the SHA-256 adds; the
numbers and how each was measured are in `docs/verify.md`. The SHA-256 adds about 13 % over a
single-threaded read that computes only the CRC-32C (one process, median of five runs). A verify
of 8.2 GiB on all cores took 2.03 s with the stored-bytes check switched off by a local,
uncommitted change and 1.88 s and 2.14 s with it (three runs each, one process per figure), so the
difference is within the spread and a separate switch is still not needed.

## Considered Options

- Keep draining after a CRC mismatch and ignore the drain's error: reads bytes that cannot change
  the report.
- Treat a `0x5003` block of another length as an open error: one bad record would hide the
  Archive (ADR-0025).
- The first of several hash blocks wins: the encryption block is read by the last one as well.
