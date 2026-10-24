# Scene meshes are shared by model, level and material library

Decided for issue #126, which builds on the Assembly scene of ADR-0040.

## Decision

`write_scene_glb` writes the meshes of nodes that hold the same model once. Two scene nodes share a
mesh when their `ScenePart`s have the same model (the same reference, not equal contents), are
drawn at the same level of detail and with the same `MaterialStatus` (again the same reference).
The first node plans the binary blocks and the glTF meshes; every later node only references those
meshes, keeping its own glTF node, name and transform. A shared mesh costs no bytes, accessors or
buffer views, and the later node plans no materials of its own.

The key is the identity of the three references, so nothing is hashed or compared byte by byte. The
Assembly already reads each model file and each material library once and hands out the same
`Arc` to every part that uses it (ADR-0040), so parts with the same Entity path and the same
library are exactly the parts that share. Models with equal bytes under different paths are not
detected: that needs a content hash with a byte comparison on a hit and is not worth the cost
until a real Assembly shows such duplicates.

The same model with another library keeps its own mesh, because a primitive names its material.
Sharing only the vertex and index blocks between those meshes is possible later without changing
the API.

## Materials and textures

Equal glTF materials of different parts were already written once (ADR-0040), and an image used by
several materials is embedded once. This decision only counts them. `GlbSummary` and the
`p4kx entity glb` report gain:

- `shared_meshes`: the meshes not written again because an earlier node holds them;
- `shared_materials`: the material requests answered by an equal material already written;
- `shared_textures`: the texture references of materials that reuse an image another reference
  already uses (0 until #127 writes textures for Assemblies).

The textured materials of #127 plug into the same key: a part's `MaterialStatus` reference decides
the texture images, and `Instance::twin` already skips a twin's material and texture planning.

## Consequences

`summary.meshes` is the number of glTF meshes, so it is lower than the number of mesh-bearing
nodes; `primitives`, `vertices` and `triangles` count what is stored, once. `hidden_primitives`
still counts every model, shared or not.
