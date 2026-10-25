### Changed

- CLI: `p4kx wwise events` now exits with 1 when the Entity lookup fails (DataForge, key or
  Archive read error); it exited with 0 before. The triggers are still listed, and
  `--no-entities` still exits with 0.
