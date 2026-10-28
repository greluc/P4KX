# Harvestable locations come from the preset in the .soc of a system container

Decided for issue #320. Evidence: LIVE 4.10, ScDataDumper `SocpakMappingGenerator`.

## Decision

- The catalog exports the mining Records as five kinds of schema version 2 (`mineableCompositions`,
  `miningGlobalParams`, `harvestableProviderPresets`, `subHarvestableMultiConfigs`,
  `harvestableLocations`). Version 2 is not released, so no bump. Facts only: numbers, GUIDs and
  Record names; the one text is the `depositName` of a composition, a game text.
- A provider preset follows each harvestable to the entity and to the composition and mining
  parameters of its `MineableParams`, so a consumer reads which rocks a place can spawn without
  the `HarvestablePreset` Records, which are not a kind.
- A location is a container below `ObjectContainers/PU/system` whose first `.soc` holds the GUID of
  a kept provider preset after the string `preset`. The `.soc` is scanned as a stream, because a
  container holds a `.soc` of over 100 MB; the scan stops at 256 MiB and a container that hits
  the limit is warned about, never taken for one without a preset. The starmap object is the
  nearest one at or above the Child element that reaches the container in the walk of the system
  containers, a mining claim marker giving way to the object above it. A marker is a starmap
  object by its `description` key `@MiningClaims_Desc_001` (all 188 `PrivateMiningPoint_*`
  Records of LIVE 4.10 and no other), a field and not the class name. A container no Child
  reaches is listed with no object.
- A `HarvestablePreset` or entity Record that is missing or cannot be built is a warning with
  count and details; the link fields of its harvestables stay `null`.
- The catalog does not pin a container to an object by hand as ScDataDumper does for four
  containers, and it does not guess a place from a name: a preset no container names (the
  surface presets of the planets) has no location; its `objectPresetPaths` name the body.

## Consequences

- 19 of 49 presets of LIVE 4.10 have a location. A later Build that names more containers in a
  `.soc` raises the number without a change of the catalog.
