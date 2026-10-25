### Added

- Models: `p4kx_model::ModelError::Unsupported` with the new `UnsupportedLayout` (`BigEndian`,
  `StreamedMesh`, `MeshVersion`, `PhysicsVersion`, `PhysicsPrimitive`) and
  `ModelIssue::PhysicsSlotsIgnored`. This is breaking for a `match` on `ModelError` that has no
  wildcard arm.
