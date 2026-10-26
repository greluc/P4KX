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
is written once however often it is reached, and it remembers the smallest depth its layers were
followed from. A reach at a shallower depth follows its layers again from there, and so the
layers below them, because they now lie less deep; a reach at the same or a greater depth does
nothing. At the depth limit of 8 layers are not followed. Each description is followed again at
most 8 times, so the work is bounded.

A description whose layers were not followed from any depth is an issue (`layer_material`,
"layers are followed 8 files deep"), and is dropped when a later reach follows them. These issues
are listed after the other issues of the package. A missing or unreadable layer file is reported
once however often it is followed.

## Consequences

The result does not depend on the order in which meshes and materials are visited: every
description has the layers a walk from its shallowest reach gives. A layer file reached through a
long chain and also directly has its full chain in the package.
