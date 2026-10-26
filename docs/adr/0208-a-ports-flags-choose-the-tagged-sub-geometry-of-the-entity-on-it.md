# A port's flags choose the tagged sub-geometry of the Entity on it

Decided for issue #731. It extends to every port the idea armour already uses for worn pieces (the
flags of a port select the sub-geometry of the Entity on it). Armour keeps its own rule: it takes
the first sub-geometry that has any tag among the flags of the whole container
(`Wearable::geometry_selected`), and the body variant tags it matches are single tags. The rule
below needs all tags covered and prefers the most tags. Armour is left as it is, because changing
its rule would need a check of every outfit; its output for the NPC checked does not change with
this decision.
Evidence: the LIVE Archive, `DRAK_Cutlass_Black`, `MISC_Prospector` and `CNOU_HoverQuad`.

## What the data says

The side doors of the Cutlass Black are one Entity on two ports,
`Door_RN_RoomConnector_Breachable_OpenReverse_Cutlass` on `hardpoint_door_left` and
`hardpoint_door_right`. Its main geometry names no model; its `SubGeometry` has one node tagged
`right` (`drak_cutlass_door_side_right.cga`) and one tagged `left`. The vehicle implementation
gives the ports `flags="uneditable right"` and `flags="uneditable left"`. An Assembly took only the
main geometry, so both doors were missing, listed as `no_geometry`, and the hull showed two holes.

The Prospector's aux thrusters (`MISC_Prospector_Thruster_Aux`) have a main geometry, the left
model, and sub-geometries tagged `left` and `right`; the right port flags `right`. Taking the main
geometry put a second left thruster inside the hull on the right and left the right side without
its thruster.

The hover skids of the HoverQuad are one Entity whose sub-geometries carry several tags
(`front left`, `center rear left`); the ports flag `center rear left`. A single shared word would
choose `front left` for it.

## Decision

The part on a port takes the sub-geometry of its Entity whose tags are all words of the port's
flags, ignoring ASCII case and a leading `$`. The flags are those of the port's definition in the
Entity Record; only when it has none, those of the vehicle implementation's item port, as the
port's constraints are chosen. Of several sub-geometries, the one with the most distinct tags wins,
the first on a tie; a sub-geometry without tags never matches. It goes before the main geometry.
The choice is made for every port that holds an Entity, also one a change of the plan replaced.
A port whose flags cover no sub-geometry keeps the main geometry, or, without one, the part has no
model and `no_geometry` is listed as before.

A sub-geometry that names no material library or tint palette takes those of the main geometry.

A chosen sub-geometry whose file is not in the Archive does not cost the part its model: the part
falls back to the main geometry of its Entity and a note issue `sub_geometry_missing` names the
missing file (`skipped` is false). Without a main geometry the part has no model and
`geometry_missing` names the sub-geometry. The Assembly finds the missing file where it reads the
models, and reads the main models of the parts that fall back once each, after the others, within
the same vertex budget. A file that exists but cannot be read does not fall back: it is
`geometry_unreadable`, as for any geometry. A sub-geometry that names the main geometry's file
needs no fallback.

## Consequences

The Cutlass Black draws its two side doors, closed, and the Prospector its right aux and retro
thrusters on the right. Other Entities shared by mirrored ports draw the model of their side.

On LIVE 538 of 1144 ships and ground vehicles change at least one model, nearly all by swapping
the left model for the right one on the right side. Models are gained on the Cutlass family, the
Redeemer, the Gladiator, the HoverQuad and the X1. No ship loses a model. The four lower support
engines of the Javelin name a sub-geometry tagged `lower` that the Archive does not hold; they keep
the main geometry and carry the note `sub_geometry_missing`.

`entity glb`, the packages, the viewer and the holograms draw what the Assembly holds, so they all
show the right-hand models. The Hammerhead pairs its main thrusters diagonally (`bottom_left` and
`top_right` carry the flag `right`) because its data says so. The NPC `MilesEckhart` gains one head
part with a model in `entity glb` (10 of 19 parts with a model instead of 9); `LucaBrunt`, a prison guard
and the armour GLB of Eckhart are byte for byte unchanged.
