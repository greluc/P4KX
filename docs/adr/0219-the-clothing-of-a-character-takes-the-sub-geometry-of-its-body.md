# The clothing of a character takes the sub-geometry of its body

Decided for issue #753. It extends ADR-0208 (a port's flags choose the sub-geometry) and follows
the body rule of the armour path (`Wearable::geometry_for`, ADR-0214).
Evidence: the LIVE Archive, the NPC `NPC_Archetypes-Male-Human-MissionGivers-MilesEckhart`.

## What the data says

The jacket of Miles Eckhart (`alb_jacket_01_01_01`) names the placeholder box
`box_clothing_1_jacket_a.cgf` as its main geometry and its real model only as `SubGeometry`
variants tagged `Male` and `Female`. His boots, pants and shirt name no main geometry at all, only
those variants. No port flag names the body, so ADR-0208 chose nothing: `entity glb` drew the jacket
as a box and listed pants, shirt and boots as `no_geometry`.

The body of a character is known from its head: the Entity of the item type `Char_Head` has the
sub-type `Male` or `Female`, which is what `npc_head` uses.

## Decision

The Entity Assembly of a character chooses the body's sub-geometry, as armour does. After a Loadout
tree is resolved (`LoadoutResolver::resolve` and the resolutions with changes), the resolver looks
for the `Char_Head` Entity anywhere in the tree and takes the body its sub-type names. A tree
without such a head is left as it is, so ships and every other Entity are unchanged.

For every Entity of the tree whose item type starts with `Char_` (armour, clothing, head parts),
whose Record names geometry per body, that is, has a sub-geometry tagged `Male` or `Female`, and
that no port flag gave a sub-geometry (ADR-0208 keeps its priority):

- it takes the sub-geometry tagged for the body, with the material library and tint palette of the
  main geometry where the sub-geometry names none;
- with no sub-geometry for the body it has no geometry and `no_geometry` is listed; the main
  geometry, a placeholder, is never drawn instead.

Items of other types keep their main geometry: the mobiGlas has a prop model with the helpers its
screens attach to and a `Male` variant without them. Armour, `character package` and `armour glb`
read the Records and are not touched.

## Consequences

`entity glb` and `entity package` of Miles Eckhart draw the jacket, shirt, pants and boots as the
`Male` skins instead of the box (15 of 19 parts with a model instead of 10), and also his hair and
teeth, which only had variants. They sit at the origin of the body's bind pose, like the other
parts of the character; the helpers they name (`Spine3`, `Hips`) are not nodes of the models, as
before. Ships and vehicles are byte for byte unchanged (`DRAK_Cutlass_Black`, `MISC_Prospector`).
