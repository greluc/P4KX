# A port's flags choose the tagged sub-geometry of the Entity on it

Decided for issue #731. It extends to every port what armour already does for worn pieces (the
flags of a port select the sub-geometry of the Entity on it). Evidence: the LIVE Archive,
`DRAK_Cutlass_Black`, `MISC_Prospector` and `CNOU_HoverQuad`.

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
flags, those of the port's definition in the Entity Record and those of the vehicle
implementation's item port, ignoring ASCII case and a leading `$`. Of several, the one with the
most tags wins, the first on a tie; a sub-geometry without tags never matches. It goes before the
main geometry. A port whose flags cover no sub-geometry keeps the main geometry, or, without one,
the part has no model and `no_geometry` is listed as before.

A chosen sub-geometry whose file is not in the Archive is listed as `geometry_missing`; the main
geometry does not stand in for it.

## Consequences

The Cutlass Black draws its two side doors, closed, and the Prospector its right aux and retro
thrusters on the right. Other Entities shared by mirrored ports draw the model of their side.

On LIVE 538 of 1144 ships and ground vehicles change at least one model, nearly all by swapping
the left model for the right one on the right side. Models are gained on the Cutlass family, the
Redeemer, the Gladiator, the HoverQuad and the X1. The four lower support engines of the Javelin
lose theirs: the sub-geometry tagged `lower` names a file the Archive does not hold.
