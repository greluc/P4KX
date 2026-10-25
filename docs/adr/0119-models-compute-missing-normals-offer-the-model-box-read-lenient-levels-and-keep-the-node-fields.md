# Models compute missing normals, offer the model box, read levels leniently and keep the node fields

Decided for issue #283. Amends ADR-0028, which rejected the fallback from a missing LOD companion
to the base companion.

## Context

Comparison with StarBreaker (MIT) showed model details that p4kx dropped or refused:

- a mesh without normals was left without, and viewers flat-shaded it;
- quantised positions were always dequantised with the second, scaling box, while a CGF placed by
  a socpak's `IncludedObjects` is authored in the first, model box;
- a missing LOD companion, or a missing geometry companion of a model that announces a mesh, was
  an error, although a caller that places a model at a distance wants a model it can draw;
- the `NodeMeshCombo` records hold a scale (+128) and a geometry type (+150); the chunk holds a
  list of material indices and, after the table of names, a table of properties with one
  NUL-terminated entry of `key=value` lines per node, e.g. `class=AnimatedJoint` (its length is
  header word 6). All were dropped, and a transform counted as the identity only when exactly
  zero.

## Decision

- **Computed normals.** A mesh that stores no normal stream gets smooth unit normals: the
  area-weighted mean of the face normals of the triangles around each vertex.
  `Mesh::normals_computed` flags them, and `model info` shows `computed` (text) and
  `normals_computed` (JSON). The `CrCh` physics meshes are covered, so their GLB carries normals.
- **Model box.** `PositionBox::{Scaling, Model}` and `Model::read_with_box`. Every other read
  method keeps `Scaling`, and `Model::read_without_materials_with_box` and
  `ModelSource::from_index_without_materials_with_box` give the placement code the same choice.
  The reader does not decide which box a file needs. The interior assembly (`ArchiveModels`) does
  not use the model box: by the face-normal fit of docs/mesh-bounding-boxes.md, of 400 interior
  CGFs of LIVE the scaling box fits clearly better in 12 and the model box in none, so the
  model box would distort them. StarBreaker's claim that placed CGFs are authored in the model
  box is therefore not adopted until a file is found that proves it; a follow-up can wire the
  choice into `ModelProvider` then.
- **Lenient levels.** `LodSelection::OnlyOrBase(n)` reads level `n` and, when it has no
  companion, the meshes of level 0 labelled with level `n`; when the geometry companion is
  missing too, or announced and absent, the model has no meshes. Either case is a
  `ModelIssue` in `Model::issues` (`LodFallback`, `MissingGeometry`), not an error; a level whose
  companion holds visibility areas only is reported as `LodWithoutMesh`. The other
  selections stay strict: `--lod n` of the CLI and the GLB export still fail with `NoSuchLod`
  (exit code 2) for a level the model lacks, so `--lod 3` is never LOD0 under another name. The
  nodes of every level come from the base model file, as before.
- **Node fields.** `Node::{scale, geometry_type, properties}` and `Model::material_indices` are
  read and shown by `model info`. A record that stores no scale reads as 1; a transform or scale
  whose values are all below 1e-5 is the identity or 1 (was: exactly zero).

## Consequences

- A GLB of a mesh without stored normals now has `NORMAL`; viewers shade it smooth.
- A caller that wants StarBreaker's fallback chooses `OnlyOrBase` and reads `Model::issues`.
- Properties are untrusted text; front-ends print them through `printable`.
