### Changed

- Basetool catalog (breaking for the Basetool, part of `schemaVersion` 2): ships whose record path
  or name holds the word `temp`, `display` or `hijacked` are left out and counted as skipped, 21
  ships on LIVE, 15 of them `player` (ADR-0167). The other kinds are not affected. The Basetool
  must accept version 2 before it reads the new catalogs.
