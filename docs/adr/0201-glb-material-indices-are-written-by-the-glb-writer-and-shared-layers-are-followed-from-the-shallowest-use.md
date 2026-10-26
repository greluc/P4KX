# GLB material indices are written by the GLB writer and shared layers are followed from the shallowest use

Decided for issue #545; it refines ADR-0092 and changes no package format.

## Decision

**The GLB writer writes the `extras`.** `GlbOptions::material_indices` (set with
`with_material_indices`) makes the writer give every glTF material of a scene GLB the `extras`
`{"p4kx_index": <index>}` while it still holds the typed document. The package writer sets it
and no longer parses and re-encodes the finished file, so the key order of a package mesh is that
of `entity glb` and no number is written twice. The option is off by default; `entity glb` output
does not change. `p4kx-gltf` enables the `extras` feature of `gltf-json` for this.

**A shared layer is followed from the shallowest depth it is reached at.** A material description
is written once however often it is reached. When a layer file is first reached at the depth
limit of 8 its own layers are not followed; when the same file is reached again at a shallower
depth, they are followed then, and the issue that said they were cut is dropped. Only a
description whose layers were never followed at any depth is an issue
(`layer_material`, "layers are followed 8 files deep").

## Consequences

The result no longer depends on the order in which meshes and materials are visited. A layer file
reached through a long chain and also directly has its full chain in the package.
