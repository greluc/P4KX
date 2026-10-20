# A missing material library is a status of the model, not an error

Decided for issue #104. Evidence: docs/model-materials.md.

`Model::read` fetches the `.mtl` file the `MtlName` chunk names, through the same resolver as the
geometry companions (`CompanionPath::Material`), and stores the outcome in `Model::materials` as
a `MaterialStatus`: `Unnamed`, `Missing`, `Unreadable` or `Loaded`.

- A model is useful without its colours, and 46 of 150 sampled skins and a few static models name
  a library the Archive does not hold. Failing the read for it would make those models
  unexportable, so a missing, corrupt, oversized or unsafe library leaves the geometry readable;
  `p4kx model info` and `model glb` report it, in text and in JSON, and the export uses the default
  material.
- Geometry companions stay fatal when missing: without them there is nothing to export.
- The name is validated by the model, not by each resolver, so that no front-end can be led out of
  the `Data` folder by a hostile name. The resolvers add only what they know: the extension and
  the root.
- A bare name is first looked for beside the model, as the engine does; the resolver for a model
  file on disk reads only `<last component>.mtl` of the path beside it.
- Sub-materials are the first level of `SubMaterials` only. No file in LIVE nests them or refers
  to another `.mtl`, so there is no cycle to detect; limits on the number of sub-materials (1,024)
  and texture slots (64) bound what a corrupt file can allocate.
- Submeshes whose material draws nothing are left out of the GLB, with a count in the summary:
  collision, shield and helper geometry would otherwise cover the model.
- Two decisions can change without touching the model crate: the colour heuristics live in
  `p4kx-gltf` (`materials.rs`), and `MaterialMode` is open for the textured mode of #105.
