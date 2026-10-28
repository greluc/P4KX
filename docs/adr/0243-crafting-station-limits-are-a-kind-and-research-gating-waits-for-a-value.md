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
  is counted.
- The categories and process types a queue accepts are not exported; they are not limits.
- The tier field `research` is left out. Neither pointer holds a value on any tier, so the
  Archive does not show the type they point to, and a field typed without a value to read would
  be a guess that ADR-0054's guarantees (an existing field never changes its type) forbid
  correcting. When a Build fills them, the fields join the tier as an additive change.

## Consequences

- Every entity Record under `entities` that no other kind takes is read into a Record tree to
  look for the component, a cost paid once per catalog build.
- The Basetool reads a missing `crafters` as an older catalog.
