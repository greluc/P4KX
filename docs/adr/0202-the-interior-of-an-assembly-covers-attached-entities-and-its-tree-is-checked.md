# The Interior of an Assembly covers attached Entities and its tree is checked

Decided for issue #526, the follow-ups of the review of #138. It amends ADR-0086, which used only
the containers of the root Entity.

## Decision

**Attached Entities.** An Entity attached to a port names its object containers in its Record as
the root does (`EntityNode::object_containers`). `EntityInterior::build` now opens the containers
of every part of the Assembly, the root first, then the attached parts in the order of
`Assembly::parts`. A container sits relative to the Entity that names it: its bone is looked up in
that Entity's model, the offset rule of ADR-0086 is unchanged, and `EntityContainer::part` names
the part. The `Assembly` keeps the containers of each part (shared, not copied) when it is built.

**Where they are written.** The GLB and the Blender package get one `Interior` node below each
part that has containers, at the part's own node, so the container follows the part when it is
moved. The viewer composes the part's world transform with the container's placement. The limits
of `InteriorOptions` (containers, parts, vertices) hold for the Assembly as a whole.

**Issues.** The `path` of an issue about a container is `Interior` for the root and
`<port path>/Interior` for an attached Entity; the paths of issues inside a container are prefixed
with the port path in the same way.

**No parts.** An Assembly without parts has no node to hang the `Interior` below, so the group is
written at the top of the scene instead of being dropped while the report counts it.

**Wording.** `helper_not_found` says the helper "is no node of the Entity's model", which holds for
a port's parent and for a container's Entity alike; `offset_differs` names the Entity, not the
hull.

**API safety.** `build`, `build_watched` and `build_with` still take the Loadout tree, but they
compare it with the one the Assembly was built from (the root's GUID and its containers) and
return `AssemblyError::TreeMismatch` for another. The parameter can be dropped once nothing passes
a tree of its own; the containers are read from the Assembly.

## Consequences

An Assembly with a carrier module that ships rooms now shows them in the viewer, the GLB and the
package. The count of containers in the report includes the attached Entities' containers.
