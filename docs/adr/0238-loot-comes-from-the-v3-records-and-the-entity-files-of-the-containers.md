# Loot comes from the V3 loot Records and from the entity files of the containers

Decided for issue #322. Evidence: LIVE 4.10, ScDataDumper and NovaSCDataExtractor (facts only).

## Decision

- The catalog exports the V3 loot Records as three kinds of schema version 2 (`lootTables`,
  `lootArchetypes`, `lootPoolFilters`) and the placed containers as a fourth, derived one
  (`lootContainers`). Version 2 is not released, so no bump. Facts only: GUIDs, Record names,
  weights, counts and tag links. The `name` of an entry is a designer's label, so no field is a
  game text.
- A table or archetype keeps every weight, the `optionalData` the Archive has for it (priority,
  choice limit, dupe limit, chance to exist, stack size, spawn-with) and the selector of each
  entry. A selector is read by its fields (`tags`, `entityClasses`) and an optional data block by
  the field it holds, not by the name of its struct.
- The older `LootTable` and `LootArchetype` Records (161 and 233) are not exported; 635 of the
  1,958 slots with a loot config on LIVE still name an older table. The slot keeps that GUID in
  `lootTableGuid`, unresolved. Secondary choices (manufacturer weights) are not exported either; no
  slot names one.
- The loot of a slot is the `lootConfig` of a sub harvestable of a multi config. A slot without a
  `lootConfig` field has `null`.
- A placed container is found as the entity file (`entdata/*.entxml`) of a container that names a
  `multiConfigRef`; the container it places (`objectContainer`) holds the tag points. The walk
  starts at every container below `ObjectContainers/PU/system` and follows the containers the
  entity files place, each read once, so a loot area in `PU/loc` is located by the system
  container that reaches it. The catalog counts the tags of the entities of the placed container
  and does not state which tag point gets which slot.
- The starmap object is that of the container the walk started in, as for the harvestable
  locations (ADR-0236): the nearest object at or above the Child that reaches it.

## Consequences

- A consumer joins `lootContainers[].multiConfigGuid` to `subHarvestableMultiConfigs`, a slot's
  `lootConfig.lootTableV3Guid` to `lootTables`, an entry's `archetypeGuid` to `lootArchetypes` and
  `lootConfig.poolFilterGuid` to `lootPoolFilters`; on LIVE every such link resolves.
- The build reads the entity files of every container the walk reaches, in parallel.
- Reading the older loot Records is a follow-up; it adds two kinds and changes nothing here.
