# Landing gear are parts of the Assembly, placed at their bone

Decided for issue #736. Evidence: the LIVE Archive, `MISC_Prospector`, `DRAK_Cutlass_Black` and
`ANVL_Arrow`.

## What the data says

The `VehicleLandingGearSystem` Record that an Entity's vehicle component names holds one gear per
leg: a `bone` and a `geometry` path. The Assembly drew only the Entities at ports, so ships were
written without legs; the only gear node was the port `landingpad_helper`, which holds an Entity
without geometry. The bone names a node of the ship's model (`hardpoint_landinggear_front` on the
Prospector, `hardpoint_Landing_Gear_Front` on the Cutlass Black), as a port helper does.

## Decision

- Every gear of an Entity is one part of the Assembly, named `landing_gear/<bone>`, with the path
  `<path of the Entity>/landing_gear/<bone>` (just `landing_gear/<bone>` for the root). It hangs
  below the Entity that owns the system and comes right after it, before the Entity's ports.
  `class_name` is the bone, and `is_landing_gear()` is true. The part has no Entity of its own:
  its `guid` is that of the owner, and a package writes it as a node of the kind `object` without
  an `entity` field, not as an `entity` node.
- The gear's model is read like any other file: shared by path, within the vertex budget, and
  counted against the limit of parts (`too_many_parts` with the bone as its name).
- The part is placed at the node of the owner's model named like the bone, as a port helper is
  (`placement: helper`). When the model lacks the node, the part sits at the owner's origin and
  `helper_not_found` is listed. When the owner has no model, the part sits at its origin and,
  as for a port, nothing more is listed, the owner's own Issue saying why. A gear without a path
  lists `no_geometry`.
- A docked vehicle that is not expanded gets no gear, as it gets no ports.
- The gear is written in the bind pose of its model; nothing is animated. Whether that pose is
  retracted or deployed is not settled by this decision. The deploy clip of the ship's animation
  database could give a later option to write the gear in the other state.

## Known gap

A gear draws with the plain colours of its material library. It names no Geometry of its own, so
it takes neither the tint palette of the ship nor a material library chosen by the Entity. Issue
#752 closes this by letting a part without its own Geometry inherit the palette of its parent
through `draft.parent`; this decision does not.

## Consequences

`parts`, the placements and the Issues of `entity glb` and `entity package` include the gears
(`docs/cli-json.md`), and the viewer draws them from the parts. A package reader of version 1.1
skips the nodes of kind `object` with their children (`docs/blender-package.md`).
