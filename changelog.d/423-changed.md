### Changed

- Basetool catalog (breaking for the Basetool, part of `schemaVersion` 2): ships whose record path
  or name holds the word `temp`, `display` or `hijacked` are left out and counted as skipped, 21
  ships on LIVE, 15 of them `player` (ADR-0167). The other kinds are not affected. The Basetool
  must accept version 2 before it reads the new catalogs.
- `p4kx ships` and `build_ship_base` apply the same rule, which is a breaking change of the ship
  base data: its `schemaVersion` is now 2 and the 21 ships are no longer listed
  (`docs/ship-base-data.md`).
