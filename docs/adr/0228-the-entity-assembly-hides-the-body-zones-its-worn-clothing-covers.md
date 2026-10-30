# The Entity Assembly hides the body zones its worn clothing covers

Decided for issue #768. It applies ADR-0140 (worn clothing hides the body zones its Record
covers) to the Entity Assembly, and follows ADR-0219 (the clothing of a character takes the
sub-geometry of its body). Evidence: the LIVE Archive, the NPC
`NPC_Archetypes-Male-Human-MissionGivers-MilesEckhart`; a Blender check showed the body through the
pants and the shirt, and with the body hidden the clothing was complete.

## Decision

`Assembly::build` cuts the body the same way `Armour::build` does, with the same functions and so
the same rule (ADR-0140: covered zones, the zone table, the fit check, the cut).

- The body is the one the sub-type of the `Char_Head` Entity in the tree names. A tree without a
  head is left as it is, so ships and every other Entity are unchanged.
- A zone is covered when an Entity of the tree whose model was read, clothing or not, has an
  unconditional chunk for it on a layer above 0 that lets no body through. Hidden parts and parts
  without a model cover nothing.
- The part whose attach type is `Char_Body` gets the cut copy of its model, whatever skin file it
  names; every other part keeps its model. The fit check against the zone table guards a skin
  that is not a human body.
- The zones are reported as `body_zones_hidden`. A body that does not fit the zone table is kept
  whole and reported as `body_zones_unknown`, as for the armour; so is a tree whose Entities cover
  zones but that has no `Char_Body` part with a model.

`EntityNode` keeps the chunks of its Record so that the Assembly needs no second read of the
Records.

## Consequences

`entity glb`, `entity package`, the hologram export and the GUI's Assembly views show no skin
through the clothing of an NPC. Ships and vehicles are unchanged. The limits of ADR-0140 stay:
`HiddenParts` and conditions are not applied, and a Record with more than 256 chunks is not
reported as truncated in the Assembly.
