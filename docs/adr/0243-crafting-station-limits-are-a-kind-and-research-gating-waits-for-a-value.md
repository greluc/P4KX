# Crafting station limits are a kind and research gating waits for a value

Decided for issue #831. Evidence: LIVE 4.10. Amends nothing; extends the catalog under ADR-0054.

## Context

VerseKit asked for the per-player blueprint cap. The Archive has none. The only crafting limits
are on the crafting stations, in `Components[CrafterComponentParams]`: `maxJobsInHistory` and, per
queue, `maxJobsInProgress` and `maxJobsWaiting`. LIVE 4.10 has three such entities, all under
`entities/basebuilding/interactables`. Separately, 926 of the 1,618 blueprint tiers carry a
`CraftingResearch` whose `researchCosts` and `unlockRequirements` are empty strong pointers on
every tier.

## Decision

- The catalog gets the kind `crafters`, one entry per entity with a `CrafterComponentParams`:
  `guid`, `className`, `recordPath`, `maxJobsInHistory` and `queues`, each queue with `name`
  (`debugName`), `maxJobsInProgress` and `maxJobsWaiting`. It is additive (ADR-0054), so
  `schemaVersion` stays 2. `Kind::Crafters` comes last in `Kind::ALL`.
- Crafters are found by the component, not by a folder. The kind's rule takes the
  `EntityClassDefinition` Records under `entities` that no other entity kind takes. An item, ship
  or commodity that is a crafter is read from its own Record and stays an item, ship or
  commodity as well (`Picked::Crafted`). Development leftovers follow the project rule: the
  `_template` bench of LIVE is skipped and counted, and only a leftover that holds the component
  is counted. Items and ships whose name marks them as leftovers are not checked for the component.
- Crafters are not limited to stations: ships, vehicles and buildings may carry them. The
  selection stays structural (the component, never a name or a station folder). For a crafter
  mounted as an item, the host is findable from the crafter: `hostGuids` lists the `ships` whose
  Default Loadout mounts it at any depth of the port tree, derived from `loadouts`. The data
  supports this link, whereas a link on the ship entry would add a field to a kind that other
  work extends. A crafter that is a building part has no loadout host and an empty list, and a
  ship whose loadout is empty or failed yields no hosts. A ship that carries the component itself is
  not its own host: its hostGuids stays empty and a reader matches crafters[].guid with
  ships[].guid.
- The categories and process types a queue accepts are not exported; they are not limits.
- The tier field `research` is left out. Neither pointer holds a value on any tier, so there is
  no value to read the pointed-to type from, and a field typed without one would be a guess that ADR-0054's guarantees (an existing field never changes its type) forbid
  correcting. When a Build fills them, the fields join the tier as an additive change.

## Consequences

- Every entity Record under `entities` that no other kind takes is read into a Record tree to
  look for the component, a cost paid once per catalog build.
- The Basetool reads a missing `crafters` as an older catalog.
