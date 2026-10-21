# `CrCh` models are read from the layouts the Archives use

Decided for issue #108. Evidence: docs/crch-models.md.

`Model::read` reads a `CrCh` file the way it reads an `#ivo` file, with its own builder
(`crates/p4kx-model/src/crch.rs`).

- The Archives hold one `CrCh` model, a Carrack collision proxy, and no `CryTek` or big-endian
  file. Only the layouts of that file are implemented: `Node` `0x824`, `Mesh` `0x801`,
  `MeshPhysicsData` `0x800` and `MtlName` `0x802`. Others are skipped and reported, or refused
  with a typed error when the file declares data the reader cannot place (`StreamedMesh`,
  `BigEndian`). A layout nothing uses would be code without a test against real data.
- The geometry is in the physics data, not in `DataStream` chunks. The reader takes the vertices
  and triangles from it at the offsets measured on the one file and checks every count and index
  against the data. It reads no material IDs of triangles; the mesh has one submesh of ID 0.
  The primitive type at `0x44` (Cryengine-Converter `PhysicsData.cs`: 0 cube, 1 polyhedron,
  5 cylinder, 6 other) refuses everything but the polyhedron, which the Carrack proxy is.
- The node matrix is the local transform, relative to the parent, as in Cryengine-Converter
  (`ChunkNode.LocalTransform`) and the engine's CGF loader (`localTM`). The two nodes of the file
  are the identity, so no measurement decides this; the reader keeps the matrix as it is.
- A `CrCh` model has one level of detail and no companion. `--lod 0` works, any other level is
  `NoSuchLod`.
- `p4kx model` prints the names of a model with control characters replaced, since the name of a
  node is read from an untrusted file.
- `ModelError::UnsupportedContainer` is removed: both containers are read.
