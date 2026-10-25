### Removed

- Models: `CorruptChunk::BigEndian`, `CorruptChunk::StreamedMesh` and
  `CorruptChunk::PhysicsPrimitive` in `p4kx-model`; the layouts are now
  `ModelError::Unsupported`. This is breaking for an exhaustive `match` on `CorruptChunk`.
