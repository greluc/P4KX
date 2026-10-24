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
(the root part's model). The Record's offset is in the Entity's space, so it normally equals the
bone: composing both doubled every position (the Constellation's front section at 39.5 m instead
of 19.75 m). A census of every Entity with containers on LIVE (1326 Entities, 5048 containers)
found 191 containers without a bone name, 45 whose bone the hull lacks, 996 with an offset of
zero, 3555 whose offset equals the bone within 0.05 m and 261 that disagree. The disagreements
are of two kinds: a bone at the hull's origin with the real position in the offset (MISC Hull C,
Command Module), and hulls longer or shorter than the base ship the offset was written for (the
Constellation Phoenix and Taurus, XIAN Scout, Retaliator animated parts, differences of 0.05 m to
several metres). So the rule is, comparing translations only (0.05 m per axis):

- a bone that resolves and sits away from the origin places the container alone; an offset that
  disagrees with it is an `offset_differs` issue (an offset of zero is no disagreement);
- a bone at the origin places the container with the offset: `bone * offset`;
- no bone name, or a bone the hull lacks (`helper_not_found`), places it at the offset from the
  origin, whatever its size;
- an offset that is not finite is the identity with an `invalid_offset` issue.

The offset rotation uses the convention of port offsets (X, then Y, then Z). The real-Archive
tests check that every placed container of the Constellation and the Cutlass equals this rule and
that all its models lie inside the hull's bounds.

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

**Limits per Entity.** `InteriorOptions` gains `max_containers`. The limits of the options hold for
the Entity as a whole: each container is built with what is left of the containers, parts and
vertices after the containers before it, and the hull's own vertices are counted. When one is
used up the remaining containers are left out with `too_many_containers`, `too_many_parts` or
`too_many_vertices`.

## Open points

The 0.05 m tolerance and the origin test are heuristics of the Records seen on LIVE. For the 261
disagreeing containers the bone is trusted over the offset, which is right for the longer Phoenix
hull and unverified for the others.
Containers in the Records of
attached Entities (modules) are not placed. Models of the Interior are drawn with plain colours or
the textures of the materials, without the hull's tint palette.
