# The flags of a head choose a face part's sub-geometry before the body tag

Decided for issue #769. It refines ADR-0208 (a port's flags choose the sub-geometry) and ADR-0219
(clothing takes the sub-geometry of its body), and follows ADR-0109 (the head's `PortFlags` apply to
all its ports). Evidence: the LIVE Archive, the NPCs `NPC_Archetypes-Male-Human-MissionGivers-MilesEckhart`
and `NPC_Archetypes-Female-Human-MissionGivers-TeciaPacheco`
(`crates/p4kx-assembly/tests/real_archive/real_face_tags.rs`).

## What the data says

The `Char_Head` Entity of a story character gives its face ports flags in two places: `Flags` on a
port and `PortFlags` on the port container, which apply to every port (`eckhart_t2_head`). The
Entities on those ports (eyelashes, teeth, eyes, brows) have one `SubGeometry` per story character,
tagged with that character's head flag, and generic ones tagged `Male` or `Female`. A head flag
names exactly one character; the body tag names every character of the body. The data holds no
rank between the two sources; the in-game order was not observed, so this rule is the one the
armour path has used since ADR-0109: the specific source, the head's flags, comes first, and the
body tag is the default.

The entity path read only the flags of the port itself, so a port without its own flags, whose head
flag sits in the container's `PortFlags`, chose nothing and fell to the body tag: Eckhart's
eyelashes were the generic male ones, while `armour glb` drew his own.

## Decision

The Assembly of an Entity of the item type `Char_Head` passes the flags of its port container to
the choice of each port's sub-geometry, beside the flags of the port (`choose_sub_geometry`). The
order for an Entity on a face port is:

1. the sub-geometry whose tags all are among the port's flags and the head's shared flags
   (ADR-0208's choice: of several, the one with most tags, the first on a tie);
2. else, for an item of type `Char_*`, the sub-geometry of the body (ADR-0219);
3. else the main geometry.

Each face port therefore has exactly one sub-geometry. The shared flags of other Entities (ships,
vehicles) are not used, so their trees are unchanged.

## Consequences

`entity glb` and `armour glb` choose the same eyelashes and brows for Eckhart
(`eckhart_t2_eyelashes.skin`, `eckhart_t2_eyebrow.skin`) and for Pacheco: her `Eyebrow_ItemPort`
holds `pacheco_t3_eyelashes.skin` and her `Eyelashes_ItemPort` the generic `protos_human_female_face_t1_pu_eyedetails.skin`,
as her Records name them. The head's own folder naming rule of ADR-0109 (bespoke teeth and eyes)
exists in the armour path only and is not a tag choice.
