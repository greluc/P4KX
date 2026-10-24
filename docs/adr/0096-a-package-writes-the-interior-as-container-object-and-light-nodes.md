# A package writes the Interior as container, object and light nodes

Decided for issue #161 (M13 Blender package, #13). Builds on ADR-0083 (the package), ADR-0086
(an Entity's Interior), ADR-0087 (props and lights) and ADR-0093 (the add-on).

## Decision

**Format 1.2.** The Interior is additive: three new `NodeKind`s (`container`, `object`, `light`)
and one new optional node field `light`. A reader of 1.1 already skips unknown kinds with their
children, so it reads the hull of a 1.1 package unchanged and warns about the minor version.
`entity package --interior` writes a node `Interior` (kind `container`) below the root, below
it the containers at the placement ADR-0086 computes, below those the `.soc` files, models,
props and lights of ADR-0081 and ADR-0087 with the names the Interior gives them.

**One node list, one mesh plan.** `p4kx-blender` flattens the Assembly and its Interior into one
list of items (`items.rs`); meshes are numbered and deduplicated over that list, so a model an
Interior shares with the hull, or two rooms share, is one GLB. An Interior model has no tint
palette. A node of the kind `object` whose model cannot be read keeps its place and has no mesh.
Container nodes and the nodes of `.soc` streams are the same kind, `container`; the
difference is in `name` and `path`, not in behaviour.

**A light is a record, not a glTF extension.** `light` holds `kind` (`point`, `spot`,
`directional`), linear `colour`, the raw `intensity` of ADR-0087, an optional `range` and, for
spots, `inner_cone` and `outer_cone` in radians. The values are those the GLB writer writes
(`SceneLight::clamped`, shared by both), so a package and `entity glb --interior` agree. The
node transform carries the light frame of the GLB writer (`light_node_transform`): the light
shines along the node's -Z axis in glTF axes, so no importer needs to know the engine's +X.
Engine types glTF cannot express stay out of the nodes and in the issues (`light_unsupported`).

**Blender mapping, in the add-on only.** The package stays unit-free; `lights.py` maps it:
point, spot and sun lights, the colour unchanged, 10 W per unit of intensity for point and
spot lights and 1 per unit for suns, times an import option *Light strength*, a spot size of
`2 * outer_cone` within 1 to 180 degrees, a blend of `1 - inner_cone / outer_cone` (as Blender's
glTF importer), the range as custom distance, and a quarter turn about X on top of the axis
conversion so that Blender's -Z lines up with the node's. The factors are assumptions, not
measurements: the engine's unit is unknown (ADR-0087), and the option exists so that a user can
correct the exposure.

## Not done

- `p4kx interior package <container>` (a container on its own, without an Entity): the node
  kinds support it; the command is a follow-up.
- Light states beyond the one ADR-0087 chooses, animation, shadows and the gobo texture.
- Materials of Interior models beyond what meshes already carry (#158).
