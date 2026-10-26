# Skeletons and bone weights: what the model keeps and how glTF gets it

Decided for issue #106. Evidence: docs/model-skeletons-and-skinning.md.

## The model

- `Model::bones` is the skeleton of the model file: `CompiledBones2` (a skin, a `.chr`) or
  `CompiledBones` (a static model with animated parts). A `Bone` has its name, parent, controller
  ID, the node it drives (the second form only) and two rigid poses, `relative` and `world`, as the
  file stores them in `CryEngine` axes. #107 matches animation tracks by controller ID and applies
  them to `relative`.
- `Mesh::influences` holds up to eight (bone, weight) pairs per vertex, strongest first, a bone once,
  weights as stored. The reader fails a mesh that weights a vertex to a bone its skeleton lacks
  (`JointRange`); a pair with weight 0 is not checked.
- A level of detail is weighted to the skeleton of its own `<stem>_lod<n>.<extension>` file when that
  differs from the model's (`Lod::bones`, `Model::bones_of`); 354 levels of LIVE do. The file is read
  for skinned levels only, and its other chunks are not listed.
- A non-finite texture coordinate reads as 0 instead of failing the model (14 `.skinm` of LIVE).

## The GLB

- The skeleton is written when a mesh of the chosen level is skinned, and when the level has no mesh
  at all (a `.chr`): one node per bone below its parent, named after the bone, with the relative pose
  as translation and rotation. The helper bones of a static model are not written; its nodes carry
  the hierarchy, and #107 decides what animating them needs.
- Y up is applied once, as for positions: a vector is `(x, z, −y)`, a quaternion `(x, z, −y, w)`, and
  an inverse bind matrix is `C · inverse(world) · Cᵀ` with `C` the turn of the axes. `world` is
  composed from the relative poses along the parents, not read from the stored world pose, so the
  bind pose always agrees with the node transforms, even for a file whose stored world poses are
  inconsistent (`chained_world_poses`). The checker of the testkit and the tests verify global matrix
  times inverse bind matrix equals identity.
- The skinned meshes share one skin whose joints are all bones in file order, with the inverse bind
  matrices as `MAT4` (floats, no buffer view target). A skin has `skeleton` when it has one root
  bone.
- The node of a skinned mesh is a root of the scene without transform: the vertices of a skin are in
  the space of the skeleton, and glTF ignores the transform of such a node. An unskinned node below
  a skinned node hangs below the nearest ancestor that is not skinned and takes over the transforms
  of the skinned nodes in between (`local(S_k) · … · local(S_1) · local(C)`), so its world pose is
  the one the model file gives; a node with no such ancestor becomes a root.
- `JOINTS_0` is `VEC4` of `u16`, `WEIGHTS_0` `VEC4` of normalised `u8`, both four-byte aligned. A
  vertex with more than four bones is written with its strongest four in set 0 and the rest in set
  1; a mesh with no such vertex has set 0 only. The weights of a vertex always add up to 255: the
  stored ones are scaled when they do not, a vertex without any weight is bound to bone 0, and a
  weight of 0 names bone 0. (Cryengine-Converter keeps eight weights with only four bone indices;
  every weight here has its bone.)

## Considered Options

- Cut the heads to four bones and renormalise by default: loses detail for viewers that read set 1
  anyway; rejected as the default, the second set costs nothing for the meshes that need only one.
  Offered as an opt-in instead (`--max-influences N`, `GlbOptions::max_influences`): the strongest
  `N` bones of a vertex are kept and their weights scaled to add up to 255 again, so `N` of 4
  writes set 0 only. The trade-off is lost detail on the 640,854 vertices of the survey with more
  than four bones (heads) against viewers such as Three.js that read `JOINTS_0` and `WEIGHTS_0`
  only and would otherwise draw those vertices with weights that do not add up to 1.
- Remap the bone indices through the nodes of the model file, as Cryengine-Converter describes for
  `#ivo`: the index is the position in the bone list in every vertex of the sample (the strongest bone
  is the near one), and no file has both a bone map and the second skeleton form.
- Split a submesh that is bound to one bone per bone, as StarBreaker does for rigid parts: a
  rendering optimisation, not a property of the data.
- Put the skinned mesh below its node and compensate in the inverse bind matrices: skins have no
  node chain, and a skinned node that is not a root draws a warning from the Khronos validator.
- Hold one skeleton per model and fail a level that disagrees: 354 levels of LIVE disagree.
