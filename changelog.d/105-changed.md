### Changed

- Models: the `TANGENT` attribute of a GLB has the handedness `w` negated, so that the bitangent
  points up the image as glTF's does; the engine's points down it, and the normal maps of
  textured materials have their green channel flipped to match (ADR-0042, which settles the open
  question of ADR-0028).
