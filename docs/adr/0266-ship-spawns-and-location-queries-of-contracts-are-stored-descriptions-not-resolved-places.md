# Ship spawns and location queries of contracts are stored descriptions, not resolved places

Decided for issue #902. Evidence: LIVE 4.10 (the DataForge Records of the contracts, their templates
and the 2,105 `MissionLocationTemplate` Records, 2,104 of them under `missiondata/pu_locations`). Builds on ADR-0054 (additive kinds) and ADR-0237.

## Decision

- Three kinds are added, additively (ADR-0054, no schema version bump): `missionLocationTemplates`,
  `contractShipSpawns` and `contractLocationQueries`. `contracts` gets three link fields,
  `shipSpawns`, `locationQueries` and `locationPrerequisites`.
- **Ships.** The Archive has no ship pool Record. A contract states its ships as
  `MissionPropertyValue_ShipSpawnDescriptions` properties: groups of slots of weighted options, each
  option a set of tags (ship model, role, faction, cargo, spawn flags) the game searches by. The
  catalog exports them as stored. It does not turn the tags into ships, factions, waves or roles:
  the Archive does not say which tag means which.
- **Locations.** A location is a tag query (`MissionPropertyValue_Location` and `_Locations` with
  `matchConditions` and `resourceTags`) that the game runs against the `MissionLocationTemplate`
  Records when the mission starts. The catalog exports the query and the templates with their tags
  and never resolves a query to templates or to starmap objects: the result depends on conditions of
  the run (the other properties of the mission, the distance to them, the organisations).
  A `ContractPrerequisite_LocationProperty` is exported as the name of the property the contract is
  offered at.
- **Identity.** The query and the spawn description are inline structures without a GUID in the
  Archive. Equal content shares one entry whose `guid` is the first 16 bytes of the SHA-256 of the
  canonical JSON of the content (the field order of the structs). It is the catalog's identifier,
  not a GUID of the Archive, and holds as long as the content does; a consumer must not store it
  across a change of the description. Without sharing, each of the 7,901 ship spawn links of the
  contracts would repeat its description; 564 are distinct.
- **Links.** A contract's properties are found by `missionVariableName` and resolved like the other
  fields of a contract: the first of sub contract, contract, handler and template wins. A weak
  pointer to another property of the same Record (the excluded or searched property of a
  condition, the `nameProperty` of a ship) is exported as that property's `missionVariableName`.
  Tags are written as their full tag paths, not as GUIDs.
- Not exported: NPC spawn descriptions (`MissionPropertyValue_NPCSpawnDescriptions`),
  the auto spawn settings, mission modules and limits of a location template, the embedded AI names
  of a ship group and the `ObjectiveToken` structure of a template. The data of third-party
  databases (SCMDB) is a hint only and is not part of the repository.

## Consequences

- A consumer joins `contracts[].shipSpawns[].poolGuid` to `contractShipSpawns` and
  `locationQueries[].queryGuid` to `contractLocationQueries`; every link resolves.
- To list candidate places of a query a consumer matches the tags of `contractLocationQueries`
  against `missionLocationTemplates[].tags`; the catalog gives no such list because the game picks
  at run time.
- `missionLocationTemplates` holds the Records under `missiondata/pu_locations`; one Record of the
  type outside that folder (a test platform) is not part of it.
