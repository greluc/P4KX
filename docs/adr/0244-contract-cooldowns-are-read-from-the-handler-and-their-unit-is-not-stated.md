# Contract cooldowns are read from the handler, and their unit is not stated

Decided for issue #832. Evidence: LIVE 4.10, the Archive's DataForge field statistics. Amended by
ADR-0271: contracts and sub contracts do override the cooldown through name-keyed parameter
overrides, so the fields resolve through the layer order of the contract facts.

## Decision

- Each contract entry of the catalog carries `hasPersonalCooldown`, `personalCooldownTime`,
  `personalCooldownTimeVariation`, `abandonedCooldownTime` and `abandonedCooldownTimeVariation`,
  as stored, and `cooldownLayer`, the level that states them. Schema version 2 keeps its number:
  the fields are additive (ADR-0054).
- The five fields exist only in the `defaultAvailability` of the handler, in every handler type
  (`Career`, `List`, `Legacy`, `LinearSeries`, `PVPBountyDef`, `ServiceBeacon`,
  `TutorialSeriesDef`). A contract and a sub contract have no cooldown field on LIVE 4.10, so the
  catalog reads the handler only and `cooldownLayer` is `handler` or `null`. A nearer layer would
  be added if the game ever states one.
- The unit is not stated. The fields are `Single` values; a search for the unit found no type, no
  attribute and no game text that shows the time (the localisation texts that mention a
  cooldown concern quantum drives, ping scans and EMPs). The catalog therefore keeps the numbers
  as stored and says so. The range (`personalCooldownTime` 0 to 2880 on `Career`, 0 to 45 on
  `List`; `abandonedCooldownTime` 0 to 60) fits minutes, with 2880 being 48 hours; a consumer
  that needs seconds must take that as an assumption, not as a fact of the Archive.
- A handler whose `defaultAvailability` states none of the five fields gives `null` for all and
  no layer. The catalog does not decide how the personal and the abandoned cooldown combine, or
  how the variation applies.

## Consequences

- The Basetool can show when a blueprint reward can be earned again from the contract that grants
  it, with the unit caveat.
- On LIVE 4.10 every one of the 5,175 contract entries states a cooldown through its handler.
