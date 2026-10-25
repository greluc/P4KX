# The four-byte second normal stream is the second UV set and the sixteen-byte tangent layout is not read

Decided for issue #343. Amends ADR-0028 (its tangent rule) and the stream reader of #299 (PR #338), which accepted both streams as normal sources.

## Context

The stream reader of #299 (PR #338) took the normal of a mesh from the best source it found: floats, a packed vector, a
tangent frame, and as a last resort the four-byte `NORMALS2` stream and the unverified 16-byte
`TANGENTS` layout. The tangent itself (with handedness, in `Mesh::tangents` and the GLB `TANGENT`
attribute) was added for #103.

The survey of the mesh chunks (docs/model-lods-and-vertex-attributes.md) shows:

- the 16-byte `TANGENTS` layout occurs in no chunk of LIVE;
- the four-byte `NORMALS2` stream occurs in 17,902 chunks of LIVE and PTU, never as the only
  normal source, always beside a tangent frame, and its packed vectors disagree with the frame's
  normal. Read as two half floats per vertex it gives finite values for all but 13 of 316 million
  vertices, 95 % of them in 0…1.

## Decision

- `Mesh::second_uvs` holds the second texture coordinates, two half floats per vertex from the
  four-byte `NORMALS2` stream, a non-finite value read as 0. It is `None` when the mesh has no such
  stream. The stream is no longer a normal source.
- The 16-byte `TANGENTS` layout is removed. A chunk that holds one has the stream skipped like any
  unknown tag; its normals come from another stream or are absent.
- The GLB writer does not write `second_uvs` as `TEXCOORD_1`. No material it writes samples a
  second set, so the attribute would only add 8 bytes per vertex. The GLB of every model stays
  byte-identical to the one before, except for the meshes whose only normal source was one of the
  two streams, and the survey finds none.

## Consequences

- A front-end that needs lightmap or detail coordinates reads them from the model.
- If a later material mapping samples a second set, the writer adds `TEXCOORD_1` for exactly
  those materials.
- Should an Archive ever hold the 16-byte layout, the survey test of the stream counts shows it
  and a new decision is needed.
