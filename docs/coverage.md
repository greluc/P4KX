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
- **Component**: the struct occurs under `Components` of at least one `EntityClassDefinition`,
  counted as Records (`entities`) and as components (`instances`) with
  [`FieldStatistics`](query.md#field-statistics); and the Component selector `Components[<name>]`
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

Run `p4kx coverage <LIVE Data.p4k>` for the counts of a Build; they change with every patch and are
not kept in this file. Types, components or folders that are missing or fail are filed as their own
issues.
