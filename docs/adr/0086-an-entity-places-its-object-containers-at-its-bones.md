# An Entity places its object containers at its bones

Decided for issue #138, which builds on ADR-0081 (the Interior). Evidence: LIVE Constellation
Andromeda (5 containers, 86 parts, 69 models) and Cutlass Black (3 containers), whose models
all lie inside the hull's bounds with this rule and 9 of 69 and 7 of 10 outside without it.

## Decision

**Where the containers come from.** An Entity Record names its socpak containers in two places:
`VehicleComponentParams.objectContainers[]` (`fileName`, `boneName`, `Offset` as a position and an
`Ang3` rotation in degrees) and `SObjectContainerComponentParams.objectContainer` (a file only).
`EntityNode::object_containers` lists both, the vehicle's first, as `ObjectContainerRef`s. Only
the root Entity's containers are used; containers of attached Entities are not yet.

**How they are placed.** A container sits at the node of the hull's model named by `boneName`
(the root part's model), moved by the offset: `bone * offset`. A container without a bone sits at
the offset from the Entity's origin. The offset rotation uses the convention of port offsets (X,
then Y, then Z). A bone the hull lacks puts the container at the offset from the origin and is a
`helper_not_found` issue.

**Bone and offset are never applied twice.** In the ship Records the bone's transform in the hull
model already equals the Record's offset: composing both doubles every position (the Constellation's
front section at 39.5 m instead of 19.75 m). When bone and offset agree within 0.05 m in the
translation and 0.01 in each rotation entry, the bone alone places the container. Otherwise both
compose.

**API.** `EntityInterior::build(archive, assembly, tree, options)` opens each container from the
Archive (below `Data` or as spelled), builds its `Interior` with `Interior::build`, and returns the
placed containers and the issues. `Assembly::with_interior` adds it: the Assembly's GLB then has a
node `Interior` below the root Entity's node, a node per container below it (ADR-0081's nodes
inside), and the Interior's issues join `Assembly::issues`. Without `with_interior` the Assembly is
the hull alone. The CLI switch is `p4kx entity glb --interior`; a front-end needs only the two
calls. The Interior is built after the Assembly and does not change its parts, so a viewer can
add or drop it without rebuilding the hull.

**Unresolved parts.** A container the Archive lacks or cannot open is `container_missing` or
`container_unreadable` with path `Interior`; what an Interior cannot resolve keeps its kind, with
the container's name put in front of its path. At most `MAX_INTERIOR_CONTAINERS` containers are
opened per Entity.

## Open points

The tolerance rule is a heuristic of the Records seen on LIVE: a container whose bone is genuinely
not at the offset, but within the tolerance, is placed at the bone. Containers in the Records of
attached Entities (modules) are not placed. Models of the Interior are drawn with plain colours or
the textures of the materials, without the hull's tint palette.
