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
  (`ExtractionError::InvalidName`, whose part and rule include an empty part, a dot part and a
  device name); the run goes on.
- An Entry collides with an Entry the run wrote earlier when its path, folded to lower case,
  equals that file, needs a folder where that file is, or is a file where that Entry made a
  folder. The collision is reported as `ExtractionError::CaseCollision` naming the written Entry.
  Only files put in place (or kept) count, so the named Entry is always one that exists. The
  Entries that can collide are the ones the run writes one after the other, in the order of the
  run and after all others, so the first of them is the one written, whatever the threads do.
  The fold is the Unicode simple lower case per character, one function for Entries and Records.
  NTFS compares names through its own upper-case table, which differs for a few characters (such
  as dotless `ı`), so two names this folds apart can still collide there.
- A file path is limited to 260 UTF-16 units with its end and a folder path to 248, the Windows
  limits without long-path support, measured on the absolute path of the target and applied to
  every file the Extraction and the Record export write. A longer path is reported per Entry
  (`ExtractionError::PathTooLong`) instead of being written with the `\\?\` prefix: output below
  that limit opens in every tool. The 4096 bytes of Linux (`PATH_MAX`) are never reached by a
  path of at most 260 units, so there is no separate Linux check.
- A Record whose name fails the check is written under its GUID, as an unnamed Record is, and the
  Record export report says so (`RecordRenamed`, `RenameCause::Name`).
- A part of a record path that fails is escaped, and the report says so (`RenameCause::Path`):
  each character that breaks a rule, the trailing dot or space and the first letter of a device
  name become `%` and two upper-case hex digits; an empty part becomes `%`. `%` itself is always
  written `%25`, so every `%` in an escaped path starts an escape and two different record paths
  never give the same file. A backslash in a record path is a character like any other and is
  escaped as `%5C`; only `/` separates the folders of a record path. A part that is still longer
  than 255 bytes after escaping, and a file name that is too long once the GUID and the extension
  are added, put the Record into `%long/<GUID>.xml` (or the extension of the format).
- A Record name is used as it is when it passes the check, even if it looks like an escape such
  as `%3C`. Where its file would equal the file of an escaped path, the existing rule for
  Records that collide ignoring case applies: the Record alone at its record path keeps
  `<record path><extension>` and the shared Record takes `<name>.<GUID><extension>`.

## Consequences

- The record path `a|b` is written as `a%7Cb`; a record path that contains `%` is written with
  `%25`; a record path such as `../evil` or `C:/evil` is written inside the target as
  `.%2E/evil` and `C%3A/evil`, where it was refused before.
- On LIVE 4.10 no Entry path or record path breaks a rule, no two Entry paths collide, and no
  record path contains `%`, so no file name changes there. One Record name ends in a space; it is
  used for a file name only when its record path is shared, and then the GUID names the file.
- An Entry whose full path exceeds 260 units fails on every platform, where the standard library
  would write it on Windows through the verbatim prefix.
