# A CrCh layout the reader does not implement is unsupported, not corrupt

Decided for issue #376. Amends ADR-0041, which refused such layouts with `CorruptChunk` variants.

## Context

ADR-0041 reads only the `CrCh` layouts the Archives use and refuses the rest with a typed error.
That error was `ModelError::Corrupt`, so a well-formed big-endian chunk, a streamed mesh or a
physics primitive other than a polyhedron exited with code 3, the code of a damaged file. A
script could not tell a file this tool cannot read yet from a broken one.

The `Mesh` `0x801` header also has four physics slots; only the first was read, and a filled
second to fourth slot was dropped without a trace. The survey of 2026-10-25 in LIVE found one
`CrCh` model, whose `Mesh` fills the first slot only.

## Decision

- `ModelError::Unsupported { file, index, chunk, source: UnsupportedLayout }` with
  `UnsupportedLayout::{BigEndian, StreamedMesh, MeshVersion, PhysicsVersion, PhysicsPrimitive}`.
  The variants leave `CorruptChunk`, which keeps truncation, bad counts and indices, cycles and
  non-finite values.
- The CLI maps `Unsupported` to exit code 1 (not 3): the input is valid, the tool cannot export
  it.
- The first physics slot that names a chunk is read. Each further slot that names one adds the
  `ModelIssue::PhysicsSlotsIgnored`, so `p4kx model info` shows that data was left out. The
  other slots are not read until a file with a filled one allows a test against real data.
- `CrCh` render models and a shared big-endian reader stay unimplemented for lack of a sample.

## Consequences

- `ModelError` gains a variant and `CorruptChunk` loses `BigEndian`, `StreamedMesh` and
  `PhysicsPrimitive`; callers that match them exhaustively must change.
- A `Mesh` chunk of another version that a node refers to is now an error instead of a node
  without a mesh.
- The line between the two is not sharp: junk in a header field, such as a nonzero stream count
  or an unknown primitive type, reads as unsupported (exit 1), not corrupt, because one sample
  cannot tell a new layout from damage.
- `Node` and `MtlName` chunks of other versions are still skipped silently, as before.
