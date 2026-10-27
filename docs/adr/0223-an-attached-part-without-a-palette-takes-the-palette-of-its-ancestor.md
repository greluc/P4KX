# An attached part without a palette of its own takes the palette of its ancestor

Decided for issue #752. It amends ADR-0195, whose rule that attached parts inherit nothing from the
ship is replaced by the one below. Evidence: the LIVE Archive, `DRAK_Cutlass_Black` (side doors,
ramp, landing gear) and `ANVL_Arrow` (landing gear), which rendered light grey or white while the
hull is black; the Prospector's gear already had the ship's palette.

## Decision

- **Its own palette** is one the part's Entity names through its geometry or sub-geometry. The
  default palette of the part's material library is not its own.
- A part takes, in this order: its own palette; else the palette of its parent part, which is
  itself its own or inherited, so the palette travels up to the root along `Part::parent()`; else
  the default palette of its material library.
- A part with a palette of its own keeps it (weapons, thrusters and so on), and its attachments
  inherit that one.
- The rule runs through the parent, not through the part's geometry: a landing gear (ADR-0210) and a
  part without a model inherit as well, and a part without a model hands the palette of its parent
  on to its own attachments.
- A named palette that cannot be resolved is listed as before; the part then takes the palette of
  its ancestor or the library default instead.
- Armour (ADR-0214) and interiors choose their palettes per piece and are unchanged.

## Consequences

`Assembly::build_with_palettes` gives more parts a palette, so `palettes` of `entity glb` and
`entity package` count them. The synthetic ship `TEST_Livery` now draws all its parts without a
palette of their own with `p_ship`; `TEST_Bare` and `TEST_Carrier` cover the library default and
the inheritance through a part without a palette.
