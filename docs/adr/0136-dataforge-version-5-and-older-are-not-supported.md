# DataForge version 5 and older are not supported

Decided for issue #312. Builds on ADR-0004 (the canonical Record tree) and the version 6 and 8
reader of `p4kx-dataforge`.

## Decision

**P4KX reads DataForge versions 6 and 8 and nothing else.** No reader for version 5 or older is
written. The reference tools differ from version 6 in the width of the data mappings (16 bits below
version 5, 32 bits from it), in the Record row and in the name table (one shared string table
before version 6); a reader would need a layout of its own for each of them.

**Every other version is a typed error, never a parse.** The header's version is checked before
any other field is read:

- Library: `DataForge::open` returns `DataForgeError::UnsupportedVersion { version }` for 5, 4, 0,
  7 and every other version outside `SUPPORTED_VERSIONS`. It is not `Truncated` and not a corrupt
  table, and nothing panics, even when the input ends right after the version word.
- CLI: a command that opens the DataForge exits with code 5 (the DataForge cannot be read) and
  prints `DataForge version <n> is not supported; this version of P4KX reads versions 6 and 8`.
- GUI: the Records tab names the version with the text `dataforge-unsupported` (de-DE and en-GB);
  the rest of the window keeps working.

## Reasons

- **No Archive uses it.** Every Channel of every shipped Star Citizen build the project has seen
  carries version 8 (`docs/dataforge-format.md`); version 6 is kept for the synthetic fixtures and
  older tools' output.
- **There are no samples.** No version 5 or older file is available, so the layouts could only be
  written from the source of other tools, which the issue itself calls buggy for this path.
- **An untestable reader costs more than it is worth.** Without real files a reader could only be
  tested against fixtures written from the same reading of the same source, which proves nothing.
  It would still have to be kept in step with every change of the Record tree, the queries and
  the exports, and it would be a second place where untrusted input is parsed.
- **The value is small.** It would serve comparisons across builds from before about 3.x, which the
  owner does not need.

## Consequences

- The tests keep a synthetic header for version 5, for version 4 and for version 6, the oldest
  supported one, in `p4kx-dataforge` and for `p4kx dcb info`.
- A later wish for older versions needs a real sample and a new decision that supersedes this one.
- The criteria of issue #312 are met in this sense: the ADR decides that older versions are not
  supported; the second criterion (read them into the canonical tree) falls away.
