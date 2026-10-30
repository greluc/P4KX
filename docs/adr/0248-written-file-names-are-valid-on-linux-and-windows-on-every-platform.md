# Written file names are valid on Linux and Windows, on every platform

Decided for issue #861. Refines ADR-0008 (Record files by record path).

## Decision

- One check, `check_part`, decides whether a name part may be written by an Extraction or a Record
  export. It accepts a part only when it is valid on both Linux and Windows:
  - no `<`, `>`, `:`, `"`, `/`, `\`, `|`, `?`, `*` and no character from U+0000 to U+001F;
  - no empty part, no `.` or `..`, no trailing `.` or space;
  - no Windows device name (`CON`, `PRN`, `AUX`, `NUL`, `CONIN$`, `CONOUT$`, `COM0`–`COM9`,
    `LPT0`–`LPT9` with the superscript digits), with or without an extension;
  - at most 255 bytes in UTF-8. NTFS counts 255 UTF-16 units; a character takes at least as many
    UTF-8 bytes as UTF-16 units, so the byte limit already keeps a part within the unit limit and
    no second check exists.
- The rule is the same on every platform, so an output folder written on one platform stays valid
  on the other. It runs in time linear in the length of the part and allocates nothing.
- Entry paths and Record names and paths use this function, so the two cannot drift apart.
- An Entry whose path fails is not written. The report names the Entry and the rule
  (`ExtractionError::InvalidName`, or `UnsafePath` for an empty or dot part); the run goes on.
- Two Entries of a run whose paths differ only in case are a collision on every platform: the
  first one in Archive order is written, a later one fails with `ExtractionError::CaseCollision`
  naming the first. Windows file systems compare names case-insensitively, so one would otherwise
  replace the other after the folder is copied.
- The full path of a file (target and relative path) is limited to 260 UTF-16 units with its end,
  the Windows limit without long-path support. A longer one is reported per Entry
  (`ExtractionError::PathTooLong`) instead of being written with the `\\?\` prefix: output below
  that limit opens in every tool. The 4096 bytes of Linux (`PATH_MAX`) are never reached by a path
  of at most 260 units, so there is no separate Linux check.
- A Record whose name fails the check is written under its GUID, as an unnamed Record is, and the
  Record export report says so (`RecordRenamed`, `RenameCause::Name`).
- A part of a record path that fails is escaped, and the report says so (`RenameCause::Path`):
  each character that breaks a rule, the trailing dot or space and the first letter of a device
  name become `%` and two upper-case hex digits; an empty part becomes `%`. `%` itself is always
  written `%25`, so every `%` in a file name starts an escape and two different record paths never
  give the same file. A part that is still longer than 255 bytes after escaping, and a file name
  that is too long once the GUID and the extension are added, put the Record into
  `%long/<GUID>.xml` (or the extension of the format).

## Consequences

- The record path `a|b` is written as `a%7Cb`; a record path that contains `%` is written with
  `%25`. On LIVE 4.10 no record path contains `%` or breaks a rule, so no file name changes there.
- One Record name of LIVE 4.10 ends in a space; it is used for a file name only when its record
  path is shared, and then the GUID names the file.
- An Entry whose full path would exceed 260 units on Windows now fails on every platform, where
  the standard library would have written it on Windows through the verbatim prefix.
