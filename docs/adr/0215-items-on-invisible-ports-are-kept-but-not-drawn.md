# Items on invisible ports are kept but not drawn

Decided for issue #748 (and #747, the power plant under the Prospector's hull). Evidence: the LIVE
Archive, `MISC_Prospector` imported into Blender.

## What the data says

- In `MISC_Prospector.xml` the ports of the cooler, quantum drive, radar, shield generator, power
  plant, quantum fuel tank and ship computer carry the flag `invisible`. The game draws the art
  pieces of the interior (`Comp_*` bays, component doors) there and not the items.
- The package drew the items on top of those pieces: through the walls, in the wrong bays and, for
  the power plant, below the hull.
- The relay, life support and shield B/C ports come from the Entity Record, not from the vehicle
  implementation, so the vehicle implementation says nothing about them.

## Decision

- A port is invisible when the part of that name in the vehicle implementation (the Modification
  applied) carries the flag `invisible`; when the vehicle implementation has no part of that name,
  when its own port definition carries the flag. A Modification that clears the flag makes the port
  visible again.
- `PortNode::visibility` states that result. The Loadout tree, `p4kx entity` and the catalog keep
  the port and its item.
- An Assembly keeps the part of an item on an invisible port, and the parts below it, in its list
  and marks them hidden (`Part::is_hidden`). A hidden part has no model, so the GLB, the package
  and the viewer do not draw it, and its geometry file is not read. It raises no Issue.
- `entity glb --json` reports `hidden_parts` and `hidden` per placement.

## Consequences

- The `visibility` of the catalog's loadout ports is `invisible` for a port whose definition is
  flagged and that the vehicle implementation does not name; it was `unspecified`.
