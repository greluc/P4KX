# Malformed Entry fields are Entry issues, not open errors

Every byte of an Archive is untrusted, and one malformed field in 1.38 million directory records
should not make the whole Archive unreadable. Fields that concern a single Entry are therefore
reported as Entry issues; the open fails only when the directory itself is broken (end records,
trailer, sizes of the directory and name table, an unreadable header, an empty or unterminated path).

`Archive::issues()` returns `(position, EntryIssue)` pairs next to the Entries, in Entry order; the
`Entry` type is unchanged. A flagged Entry stays listed and fails with a typed error only when it
is read and its data is really unreadable.

## Decisions

- **Encryption.** The v1 CIG block 0x5002 and the v2 record field hold 0 (plain) or 1 (encrypted).
  Any other value is `UnknownEncryption` and the Entry is read as encrypted: decrypting plain data
  yields garbage that the checksum catches, while skipping the decryption of encrypted data would
  silently pass ciphertext on as content. A 0x5002 block that is not 6 bytes long (4 header, 2
  value) is `EncryptionBlockSize`; the value is still read when the block holds one, and a block too
  short for a value counts as encrypted.
- **Data range (v2).** An Entry whose data ends behind the start of the trailer is
  `DataOutsideFile`. Opening it fails with the existing `Corrupt` error; the other Entries read.
- **Non-UTF-8 paths.** `Entry::path` holds the path with invalid bytes replaced by U+FFFD, for
  display and lookup. The stored bytes are kept only for the flagged Entries, in the issue, and
  `Archive::raw_path` returns them. Extraction names such an Entry from the stored bytes: valid text
  as it is, `%` as `%25`, each invalid byte as `%` and two upper-case hex digits. Different bytes
  never share a name, which U+FFFD does not guarantee, and the result still passes the same
  traversal checks as every Entry path. A real file whose name literally spells such an escape
  could still collide; the Conflict policy then decides as for any two Entries with one path.
- **ZIP64 locator.** It is looked for right before the end record, and otherwise searched
  backwards through the end region already read; a candidate counts only when the ZIP64 end record
  signature stands at the offset it names.

## Considered Options

- Reject the Archive on the first malformed field: simplest, and what the v1 reader did for paths,
  but one bad record would hide the rest of a multi-gigabyte Archive.
- Treat unknown encryption values as plain, or as unreadable: the first passes ciphertext as
  content, the second loses an Entry that may decrypt.
- Decode non-UTF-8 paths with a code page as SharpZipLib does: the Archive does not state one, so
  the choice would be a guess, and the decoded names could still collide.
- Keep the raw bytes on every `Entry`: 16 bytes for each of 1.38 million Entries for a defect that
  does not occur in real Archives.
