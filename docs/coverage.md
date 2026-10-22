# Coverage of companion-app game data

A companion app reads a fixed selection of game data from DataForge and the Archive. This check
holds that selection as a list of names and tells, for each name, whether the Archive has it, how
much of it, and whether it reads, exports and converts. Gaps show up early and become issues.

The list holds facts only: the names of Record types, of component structs and of a folder, as the
game spells them. It is data, not code, in
[`crates/p4kx-extraction/data/coverage-reference.txt`](../crates/p4kx-extraction/data/coverage-reference.txt),
one `<area> <kind> <name>` per line. The kinds are `record`, `candidate` (a Record type named as a
likely future need), `component` and `entries`.

```text
p4kx coverage <archive> [--format table|json]
cargo test --release -p p4kx-extraction --test real_archive -- --ignored --nocapture the_companion_app
```

The JSON is defined in [cli-json.md](cli-json.md). The exit code is 1 when a listed type, component
or folder is missing, unreadable or does not convert; candidates do not count. The ignored test
prints the whole table before it asserts, and needs `P4KX_DATA_P4K`.

## What is checked

- **Record type**: the DataForge defines it; how many Records it has, counting the types that
  derive from it as a Query `from <type>` does; how many Records cannot be built; how many values
  in them are a `Value::Error`; and that the first readable Record writes as Record XML and as
  Record JSON.
- **Component**: the struct, or a struct deriving from it, occurs under `Components` of at least
  one `EntityClassDefinition`, counted as Records (`entities`) and as components (`instances`);
  Records that cannot be built are counted apart; and the Component selector `Components[<name>]`
  resolves in a Query over `EntityClassDefinition`. A component is a struct, so a Query has to
  select a field of it, as the examples below do.
- **Entries**: every Entry under the folder is read and, when it is CryXml, converted to XML
  without error. Text XML counts separately.

## The list and a Query per area

The Query is an example to start from; run it with `p4kx dcb query <archive> "<Query>"`.
`p4kx dcb fields <archive> <type> --path "<glob>"` lists the fields a type has.

| Area | Record types | Components | Example Query |
| --- | --- | --- | --- |
| `missions` | `MissionOrganization`, `MissionItem`, `AIWaveCollection`, `ContractGenerator`, `ContractDifficultyProfile`, `MissionBrokerEntry`, `ContractTemplate`; candidate `MissionLocationTemplate` | | `from ContractGenerator` |
| `reputation` | `Faction`, `FactionReputation`, `SReputationRewardAmount`, `SReputationScopeParams`, `SReputationStandingParams` | | `from FactionReputation` |
| `crafting` | `BlueprintPoolRecord`, `CraftingBlueprintRecord`, `BlueprintCategoryRecord`, `CraftingGameplayPropertyDef` | | `from CraftingBlueprintRecord` |
| `mining` | `MineableComposition`, `MineableElement`, `MiningGlobalParams`, `ResourceType`, `ResourceTypeGroup`; candidate `SubHarvestableMultiConfigRecord` | `MineableParams`, `SEntityComponentMiningLaserParams` | `from MineableComposition` |
| `consumables` | `ConsumableSubtype`, `ConsumableType` | `SCItemConsumableParams` | `from ConsumableType` |
| `law` | `InfractionDefinition`, `Jurisdiction` | | `from Jurisdiction` |
| `cargo` | `CargoManifest`, `InventoryContainer` | `ResourceContainer`, `SCItemInventoryContainerComponentParams`, `SItemPortContainerComponentParams` | `from CargoManifest` |
| `starmap` | `StarMapObject`, `StarMapObjectType`, `StarMapAmenityTypeEntry`, `SolarSystem` | | `from StarMapObject` |
| `vehicles` | `VehicleCareer`, `VehicleRole`, `SLoadoutAssortment` | `VehicleComponentParams`, `SEntityComponentDefaultLoadoutParams`, `SEntityPhysicsControllerParams`, `SCItemVehicleArmorParams`, `SCItemQuantumDriveParams`, `SCItemShieldGeneratorParams` | `from EntityClassDefinition at "libs/foundry/records/entities/spaceships/*" select Components[VehicleComponentParams].vehicleName` |
| `items` | `AmmoParams`, `SCItemManufacturer`, `Tag` | `ItemResourceComponentParams`, `SAmmoContainerComponentParams`, `SAttachableComponentParams`, `SCItemBombParams`, `SCItemMissileParams`, `SCItemWeaponComponentParams`, `SSCSignatureSystemParams`, `EntityComponentAttachableModifierParams` | `from EntityClassDefinition at "*/ships/weapons/*" select Components[SAttachableComponentParams].AttachDef.Size` |

The Entry folder is `Data/Scripts/Entities/Vehicles/Implementations/Xml/`, the vehicle
definitions (`vehicles` area).

## Counts on LIVE

LIVE 4.10. Counts change with every patch; `p4kx coverage <Data.p4k>` gives those of a Build.

**Record types** (Records, including derived types; error values in brackets)

| Area | Type | Records |
| --- | --- | --- |
| `missions` | `MissionOrganization` | 69 |
| | `MissionItem` | 228 (2) |
| | `AIWaveCollection` | 184 (35) |
| | `ContractGenerator` | 110 (2) |
| | `ContractDifficultyProfile` | 7 |
| | `MissionBrokerEntry` | 2584 (13) |
| | `ContractTemplate` | 496 |
| | `MissionLocationTemplate` (candidate) | 2105 (10) |
| `reputation` | `Faction` | 60 (1) |
| | `FactionReputation` | 39 |
| | `SReputationRewardAmount` | 58 |
| | `SReputationScopeParams` | 46 |
| | `SReputationStandingParams` | 380 |
| `crafting` | `BlueprintPoolRecord` | 160 |
| | `CraftingBlueprintRecord` | 1619 (1) |
| | `BlueprintCategoryRecord` | 21 |
| | `CraftingGameplayPropertyDef` | 29 |
| `mining` | `MineableComposition` | 249 |
| | `MineableElement` | 46 |
| | `MiningGlobalParams` | 7 |
| | `ResourceType` | 207 (7) |
| | `ResourceTypeGroup` | 19 |
| | `SubHarvestableMultiConfigRecord` (candidate) | 103 |
| `consumables` | `ConsumableSubtype` | 83 |
| | `ConsumableType` | 6 |
| `law` | `InfractionDefinition` | 43 |
| | `Jurisdiction` | 13 |
| `cargo` | `CargoManifest` | 82 |
| | `InventoryContainer` | 585 |
| `starmap` | `StarMapObject` | 2132 (3) |
| | `StarMapObjectType` | 21 |
| | `StarMapAmenityTypeEntry` | 25 |
| | `SolarSystem` | not a struct (0) |
| `vehicles` | `VehicleCareer` | 11 |
| | `VehicleRole` | 60 |
| | `SLoadoutAssortment` | 321 (107) |
| `items` | `AmmoParams` | 243 |
| | `SCItemManufacturer` | 1203 (1) |
| | `Tag` | 18891 |

`SolarSystem` is not a DataForge struct on LIVE 4.10; Nyx, Pyro and Stanton are `StarMapObject`
Records whose `StarMapObjectType` is named `SolarSystem` (#465). The error values are tracked in
#464. No Record failed to build and every sample exported as Record XML and Record JSON.

**Components** (`EntityClassDefinition` Records with the component; these counts are from the run
that counted the exact struct, so a rerun can show more where structs derive from a component)

| Area | Component | Records |
| --- | --- | --- |
| `mining` | `MineableParams` | 274 |
| | `SEntityComponentMiningLaserParams` | 51 |
| `consumables` | `SCItemConsumableParams` | 308 |
| `cargo` | `ResourceContainer` | 2336 |
| | `SCItemInventoryContainerComponentParams` | 2499 |
| | `SItemPortContainerComponentParams` | 16919 |
| `vehicles` | `VehicleComponentParams` | 1151 |
| | `SEntityComponentDefaultLoadoutParams` | 8463 |
| | `SEntityPhysicsControllerParams` | 21274 |
| | `SCItemVehicleArmorParams` | 220 |
| | `SCItemQuantumDriveParams` | 63 |
| | `SCItemShieldGeneratorParams` | 73 |
| `items` | `ItemResourceComponentParams` | 4360 |
| | `SAmmoContainerComponentParams` | 487 |
| | `SAttachableComponentParams` | 26531 |
| | `SCItemBombParams` | 3 |
| | `SCItemMissileParams` | 69 |
| | `SCItemWeaponComponentParams` | 866 |
| | `SSCSignatureSystemParams` | 7008 |
| | `EntityComponentAttachableModifierParams` | 94 |

All components are reachable by a Component selector Query.

**Entries**: 177 under `Data/Scripts/Entities/Vehicles/Implementations/Xml/` (the issue counted 172
at the time of writing); all 177 convert from CryXml.
