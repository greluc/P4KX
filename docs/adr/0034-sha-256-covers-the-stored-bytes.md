# The SHA-256 of an Entry covers its stored bytes

Besides the CRC-32C, an Archive directory states a 32-byte hash for every Entry: the data of the
CIG extra block `0x5003` in the v1 layout, the 32 bytes after the encryption field in a v2
record. unp4k_rs reads it as a SHA-256 (#306). It was unknown what the digest covers: the decoded
data, the stored data, or something else.

## What the digest covers

Measured on `Data.p4k` of LIVE 4.10, PTU, EPTU, HOTFIX and TECH-PREVIEW (v1 layout; 1,365,248 to
1,406,830 Entries each): every Entry states a non-zero 32-byte digest, and on 9 sampled Entries of
every channel it is the **SHA-256 of the stored bytes**, the `compressed_size` bytes at the start
of the data, exactly as they lie in the file:

| Kind | SHA-256 of the stored bytes | SHA-256 of the decoded data |
|------|-----------------------------|-----------------------------|
| stored, unencrypted | matches | matches (same bytes) |
| zstd, unencrypted | matches | differs |
| zstd, encrypted | matches (the ciphertext with its padding, no key needed) | not measured (needs the key) |

The decoded data is covered by the CRC-32C; the SHA-256 adds a second, cryptographic check of the
bytes before decryption and decompression. Because it hashes the ciphertext, it is independent of
the Archive key: a wrong key leaves it intact and fails the CRC-32C.

No v2 Archive was available. The v2 record's 32 bytes after the encryption field are read and
exposed by `Entry::sha256` but not verified until a real v2 Archive confirms the layout (owner's
decision; `docs/verify.md`).

## Decisions

- **`Checksum::Sha256([u8; 32])`.** The new variant, `ChecksumHasher` state and `Check` that
  ADR-0026 prepared. `Checksum::value` returns `Option<u32>` (none for a digest), and
  `Checksum::hex` gives the 8 or 64 lower-case hex digits that the text and JSON reports use. The
  reader, `DataError::ChecksumMismatch` and the front-ends are otherwise unchanged; the JSON
  `kind` of such a mismatch is `sha256`.
- **Verification covers v1 only.** `open_entry_verified` checks the SHA-256 of an Entry of the v1
  layout; the digest of a v2 Entry is returned by `Entry::sha256` and not checked.
- **`Entry::sha256`** returns the digest the directory states. An all-zero digest, or a `0x5003`
  block of another length than 32 bytes, states none (`None`): a real SHA-256 of zeros is out of
  reach, and the synthetic fixtures use zeros for "no digest". `Entry::checksum` stays the CRC.
  A block of another length is also an Entry issue (ADR-0148).
- **Hashed on the stored side of the reader.** `Archive::open_entry_verified` hashes the stored
  bytes in a tap below decryption and decompression, so the Entry is still read once. At the byte
  that completes the decoded size, the bytes the decoder did not consume (AES padding, a
  compression trailer) are read from the file and hashed too, and then the CRC and the SHA-256 are
  compared, the CRC first. The tap and its progress are shared through an `Arc<Mutex<_>>`; a
  verified read takes the lock once per chunk the decoder pulls.
- **Off by default, and free when off.** Only `open_entry_verified` installs the tap, so
  `open_entry` and every other reader do exactly what they did. With `p4kx verify` and
  `--verify` the digest is checked whenever the Entry states one; there is no separate switch,
  because it is the same opt-in as the CRC and costs one more pass over the stored bytes.
- **`sha2` 0.11.1.** The newest stable release, which selects the SHA-NI (x86-64) or ARMv8 SHA-2
  instructions at run time and falls back to portable code; no feature is needed.

## Considered Options

- Verify v2 on the same assumption: rejected, because a wrong assumption would fail every v2
  Entry and make `extract --verify` write nothing; the CRC-32C still covers the decoded data.
- Hash the decoded data: does not match what the Archive states.
- A second read of the stored bytes after the decode: doubles the reads of the largest Entries.
- A second `Verifier` field instead of the tap: the digest covers bytes that never reach the
  verifier, so it would have to see the stored stream anyway.
- A separate `--sha256` switch: the checks differ only in cost, and one opt-in keeps the exit
  codes and reports simple.
