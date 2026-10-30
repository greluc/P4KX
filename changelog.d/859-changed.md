### Changed

- Entity export: `entity glb` and `entity package` fail when the main geometry of the Entity
  itself is not in the Archive (exit code 2) or cannot be read (exit code 3), naming the file, and
  write nothing; before they wrote a GLB or package without meshes. A part below the Entity that
  cannot be drawn is still listed and skipped.
