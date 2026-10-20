# The Archive key comes from the user, never from the code

A legal review of the tool (2026-10-20) found that shipping the AES-128 key of `Data.p4k` inside the
tool comes close to distributing a tool that circumvents a technical protection measure (§ 95a (3)
UrhG, Art. 6 InfoSoc Directive), whether or not the key is publicly known. The owner decided that
the key is no longer part of the code, the documentation or the release binaries: the user enters
it.

## Decision

- **A value, not a constant.** `p4kx-foundation` has `ArchiveKey`, parsed from 32 hexadecimal digits
  with a typed `ArchiveKeyError` that never repeats the input. Its `Debug` is redacted and it has
  no `Display`. `Archive::open_with_key` or `Archive::set_key` give an Archive its key, and the
  containers opened from it share it, also those opened already. The key is handed to a reader once
  when an Entry is opened, so the per-block read path is unchanged.
- **Without a key an Archive still works.** It opens, lists, searches and compares, and reads every
  unencrypted Entry. An encrypted Entry fails with `ArchiveError::KeyRequired`, which every
  front-end shows in its own words: a message with a hint and exit code 7 on the command line, a
  note in the preview and a key dialog in the window, an error text with the same hint in the MCP
  server.
- **A wrong key is detected, without shipping a hash.** The first time an encrypted Entry is
  opened with a key, and whenever a front-end calls `Archive::verify_key`, the smallest encrypted
  Entry with a CRC-32C in its directory (at most 16 MiB stored) is decrypted and decompressed and
  its CRC-32C compared with the directory's. A mismatch, or data that does not decompress, is
  `ArchiveError::WrongKey`; the verdict is kept with the key, so a wrong key fails once and not
  again for every Entry. An Archive without such an Entry cannot refute a key and accepts it
  (`KeyCheck::Unchecked`). Nothing derived from the key is stored anywhere.
- **Where the key comes from.** The command line takes `--key <hex>`, then the environment
  variable `P4KX_ARCHIVE_KEY`, then the key stored in the user's profile; a source that is set but
  holds no key is an error, not a fallback. The MCP server takes the same three, reading the
  stored one on every call. The window takes the variable, then the stored key, and asks in a
  dialog when an encrypted Entry is first needed, and from the Archive menu.
- **Where the key is stored.** `p4kx-settings` has `KeyStore`: the file `archive.key` next to
  `settings.toml` in the platform config directory, one line of 32 digits. It is written to a
  temporary file next to it and renamed over the old one, with mode 0600 on Unix; on Windows the
  file keeps the permissions the user's profile folder gives it. It is never in the repository or
  next to an Archive. A key is stored only after it passed the check above when an Archive with an
  encrypted Entry is at hand.
- **The key does not leave the process.** It is never logged and never part of an error message,
  JSON output, Debug output or a crash report; where a front-end shows a stored key, all but its
  last four digits are `*`.
- **No key in the repository.** `p4kx-testkit` encrypts synthetic Archives with a test key of its
  own, so CI needs no real key. Real-Archive tests read `P4KX_ARCHIVE_KEY` and fail with a message
  when it is missing. `cargo xtask keys` fails when a tracked file holds a list of 16 or more
  hexadecimal bytes, 32 hexadecimal digits or a base64 string of 16 bytes close behind the word
  `key`, and, while `P4KX_ARCHIVE_KEY` is set on a developer's machine, when any tracked file holds
  the real key as raw bytes, hexadecimal digits, a list of hexadecimal or decimal bytes or base64.
  The check itself contains neither the key nor a hash of it, and its messages name the file and
  the spelling only.
- **History stays.** The Git history and the released binaries 0.1.0 and 0.2.0 are not changed;
  from the next Release on the key is no longer shipped. The documentation says that the key has to
  be entered and does not say where to find it.

## Considered Options

- Keep the key in the code but obfuscated, or split over several constants: still ships the key,
  and only looks like it does not.
- Ship a hash or a derived value of the key to check a typed one: a hash of a 128-bit key stays a
  fingerprint of it that a brute-force search or a table can use; the CRC-32C of an Entry that the
  Archive carries already does the job.
- Download the key on first run: moves the distribution to a server, which does not change the
  legal position.
- Check the key on every Entry: costs a decryption per Entry; once per key and Archive is enough.
