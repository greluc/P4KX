# A character whose head names no body is dressed by its body Entity, and a worn world prop is left out

Decided for issue #968. It extends ADR-0219 (the clothing of a character takes the sub-geometry of
its body). Evidence: the LIVE Archive, the 1441 `NPC_Archetypes` Entities, and the real-Archive
test `crates/p4kx-assembly/tests/real_archive/real_generic_npc_clothing.rs`.

## What the data says

About 1000 worn `Char_` items of the NPC archetypes resolved to a cardboard box
(`box_clothing_1_*.cgf`) or an armour crate (`crate_armor_*.cgf`), drawn at the origin of the
character. The cause is neither a missing `SubGeometry` nor a gap in the resolver's lookup: the
Records of these items hold their real models as `SubGeometry` tagged `Male` and `Female`, and the
main geometry is a placeholder.

ADR-0219 takes the body from the sub-type of the `Char_Head` Entity. 1393 of the 1441 archetypes
wear the generic head `PU_Protos_Head`, whose sub-type is `UNDEFINED` and which has no model, so the
body was unknown, `dress_body` did nothing and the placeholder stayed. Only the 20 story characters
with a head of their own (`*_t1_head`, sub-type `Male` or `Female`) were dressed.

Every one of these characters has a `Char_Body` Entity (`m_body_01`, `f_body_01`) whose sub-type
names the body (`Male`, `Female`; all but the 14 below) and whose model lies in the folder of the body
(`male_v7`, `female_v2`). 14 prisoners wear `body_01_noMagicPocket` with the sub-type `Fluid`; their
model lies in `male_v7`.

A few items name no `Male` or `Female` sub-geometry at all and only a placeholder as main geometry
(`bnu_shirt_01_01_01`, `Clothing_Torso2_Unified`): the Records hold nothing to wear.

## Decision

- **The body of a character is the one its head names, else the one its `Char_Body` Entity
  names:** the sub-type of the body Entity when it is `Male` or `Female`, else the body whose
  folder holds the model of the body Entity. A tree with neither is left as it is, as in ADR-0219.
  The head keeps priority.
- **A worn item whose only geometry is a world prop has no geometry once the body is known.**
  Such an item is a `Char_` item without sub-geometry per body whose main geometry is neither in
  `Objects/Characters` nor a `.cdf`, `.chr` or `.skin` file. It is left out of the Assembly and
  `no_geometry` is listed for it, as for an item without a sub-geometry for the body (ADR-0219).
  An item whose main geometry is a character model keeps it.
- `EntityNode::character_body`, the body zones and `npc_head` keep the head as their source: they
  need the head Entity, and the body cut is not part of this change.

## Consequences

The generic NPCs of the Entity Assembly wear their `Male` or `Female` clothing instead of boxes; no
`Char_` item of the 1441 NPC archetypes has a world prop as its geometry. Items that have no
sub-geometry for the other body, such as a hat with a `Male` variant on a female character, are
still left out with `no_geometry`. The body zones that clothing covers are not cut from the body of
a generic NPC, since the cut still needs a head that names the body; this is a follow-up.
