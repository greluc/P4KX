# The Archive has no reliable released marker for starmap objects

Decided for issue #417. Builds on ADR-0054.

The Basetool imports every `starmapObjects` entry and shows none until it knows which ones a player
can reach. The question: does the Archive carry a structural marker, not guessed from names, that
separates a released starmap object from an unreleased or hidden one?

## Decision

- **No field is added.** No candidate separates released from unreleased without naming a system or
  a record. `starmapObjects` keeps its shape; the Basetool keeps the release decision.
- **What the Archive does state, all already exported or derivable from exported fields.**
  `hideInStarmap` is a display flag of the object itself. `system` and `position` tell whether a
  system container places the object. `type` tells the kind, and the type Record carries
  `validQuantumTravelDestination` and `showInMapSelectList`, which are facts about the type, not
  about release.

## Evidence

LIVE, 2026-10-22 (2,093 exported objects, 2,123 Records under `starmap/pu`, 21 `StarMapObjectType`
Records):

- **The three `SolarSystem` Records are identical in every flag.** `NyxSolarSystem`,
  `PyroSolarSystem` and `StantonSolarSystem` all have `hideInStarmap`, `hideInWorld` and
  `blockTravel` true, `excludeFromLevelLoad` false and `exposeForPlayerCreatedMissions` true. Only
  Stanton carries a `jurisdiction`. Nothing on a system Record says it is released, and Nyx cannot
  be told from Pyro.
- **Placed objects by system and `hideInStarmap`.** Stanton 489 visible and 360 hidden, Pyro 230
  and 57, Nyx 19 and 84. The flag marks helper content inside a released system (Stanton hides 360
  of 849) as it does in an unreleased one, so "visible" is not "released", and Nyx has 19 visible
  objects (its planets, the Keeger and Glaciem structures, the Levski station).
- **Unplaced objects (854).** 666 are hidden, 188 are not. Their roots are the Nyx, Pyro and
  Stanton stars (the same trees as the placed ones) and some 50 single-object trees: mission and
  player quantum beacons, early-access leftovers (`LOC_PLACEHOLDER`, `LOC_UNINITIALIZED` names),
  sandbox locations. They sit in no system container, so no system exists to be released. The
  `parent` chain does not help: a placed and an unplaced object share the same star root.
- **Jump points.** Records of the type `Anomaly` and `JumpPoint` reach Nyx with `hideInStarmap`
  false (`JumpPoint_Nyx_*` to the Nyx destinations) and Pyro or Stanton with it true, so the flag
  does not follow reachability. None of them has `blockTravel`.
- **Type suffixes.** `Asteroid_ValidQT` (134 objects) and `Outpost_InvalidQT` (7) describe whether
  a player can quantum travel to the object, not whether the content is live: the 7 invalid
  outposts are 6 hidden Pyro outposts and 1 unplaced.
- **Other fields.** `jurisdiction` and `affiliation` are null on 95 % and 98 % of the objects, placed
  or not. `blockTravel` is true for 6 of 2,093. `excludeFromLevelLoad` is false for every placed
  object and true for only 3 unplaced ones. `locationHierarchyTag` is set on 515 placed and 83
  unplaced objects. None splits the set along a release line.

## Consequences

- The Basetool combines the exported facts under its own rule, e.g. "placed in a system the
  Basetool lists, not `hideInStarmap`". That is a policy and stays outside the catalog; the catalog
  carries no derived release value (ADR-0065).
- A marker may appear in a later patch, or a Record type outside `starmap/pu` (missions, locations,
  jurisdictions) may state availability. Nothing of the kind was found in the starmap Records; a
  reference scan of the objects (`p4kx dcb refs`) was not needed to reach this conclusion and
  can be done when the question returns.
