# Basetool catalog

The Basetool catalog is the JSON Export the basetool's P4K import consumes (ADR-0002). Its contract
is the JSON Schema (draft 2020-12) in
[`schemas/basetool-catalog.schema.json`](../schemas/basetool-catalog.schema.json), generated from
the Rust types in `crates/p4kx-export/src/contract.rs`. A test fails when the checked-in schema and
the types drift apart; `P4KX_BLESS=1 cargo test -p p4kx-export --test catalog_schema` regenerates
it.

## Versioning and consumers

The rule is ADR-0054 (amending ADR-0002):

- **Additive changes keep `schemaVersion`:** a new kind, a new field on an entity, on `source` or
  on an ingredient, a new kind in `completeness`. Example: a `playable` field on every ship.
- **Breaking changes raise `schemaVersion`:** renaming, removing or retyping a field, a changed
  meaning, restructuring a kind, or a changed selection or ordering of an existing kind. Example:
  renaming `crewSize`.
- **A consumer ignores kinds and fields it does not know,** and still rejects a `schemaVersion` it
  does not know. The JSON Schema therefore allows additional properties everywhere; every field it
  lists is required.
- **Guarantees the Basetool relies on;** breaking one is a breaking change: an existing field never
  changes its type, its meaning or whether it can be `null`; a new kind never changes the
  selection or the ordering of an existing kind; the meaning of `completeness` stays.
- **A new value in an existing closed value set is additive,** e.g. a new playability value or a
  new `itemClass`. A reader maps an unknown value to `null`. The schema documents such a set but
  keeps the type a string, so an older schema accepts the new value.
- **Loadouts (#393), paints, item stats and the starmap are kinds of the one catalog;** the size is
  handled by compression (`.json.zst`), not by a second Export. The Basetool accepts `.json`,
  `.json.gz` and `.json.zst`; the 64 MB limit applies to the compressed upload, the decompressed
  content is capped at 1 GB. The compressed form is described under "Compressed form" below.
- **A production import takes catalogs of the Channel `LIVE` only.** `PTU`, `EPTU`, `TECH-PREVIEW`
  and `HOTFIX` are confidential test Channels (ADR-0038).

### Game texts

Identifiers, short names and fact values may reach third parties; descriptions and lore may not.
The game texts of the catalog are every `description` and every `displayType`, each with its `key`,
`en` and `de`. `name`, `role`, `career` and the other fields are identifiers, short names or fact
values. The catalog keeps the `description` fields for the Basetool's logged-in members; the
Basetool keeps them, and `displayType`, away from third parties. Every such field carries
`"x-gameText": true` in the JSON Schema, and no other field does; a test checks this.

```powershell
p4kx catalog <archive> --out basetool-catalog.json [--filter <filter>] [--exported-at <seconds>]
p4kx catalog <archive> --out basetool-catalog.json.zst
p4kx catalog schema
```

### Compressed form

An output name ending in `.zst` (any case) writes the catalog as one zstd frame; any other name
writes plain JSON, which stays the default. The JSON is serialised straight into the encoder and
never held in memory; it is serialised twice, the first time only to count its length. The frame
uses zstd level 19, long-distance matching, a content checksum, and declares the content size
(`Frame_Content_Size`), so the window it needs is the content size, about 26.5 MB for LIVE, and not
the 2^27 maximum the encoder is allowed. A plain `zstd -d basetool-catalog.json.zst` is enough, no
`--long` option is needed, and a decoder's memory is about the size of the catalog. The decoded
bytes are exactly the plain catalog for the same Archive and export time (tests check this with the
Rust decoder and read the window and content size from the frame header). The file is written to a
temporary file next to the target and renamed on success, so a failure or Ctrl+C leaves an
existing catalog as it was.

The Basetool detects `.json`, `.json.gz` and `.json.zst` by magic bytes, not by name. Its limits
apply to two sizes, and `p4kx catalog` and the desktop window check both:

| Size | Limit | Constant |
|------|------:|----------|
| The file as uploaded, compressed if it is | 64,000,000 bytes (64 MB) | `UPLOAD_LIMIT` |
| The decompressed content | 1,000,000,000 bytes (1 GB) | `DECOMPRESSED_LIMIT` |

The Basetool rejects a file above either limit; `p4kx catalog` and the desktop window only warn,
from 90 % of a limit and again above it. The summary (`--json`: `size`, `compressed_size`,
`upload_limit`, `decompressed_limit`, `size_warning`, `decompressed_warning`) carries the
uncompressed size, the compressed size (`null` for plain JSON) and the warnings of both.

## Shape

The file is UTF-8 JSON in one line, ending in a newline. Field names are camelCase and mean the
same in every kind. Every field is always present; one that has no value is `null`.

| Field | Meaning |
|-------|---------|
| `schemaVersion` | The version of the shape, now `1`; an import rejects a version it does not know |
| `source.channel` | The Channel: the folder holding the Archive, e.g. `LIVE` |
| `source.build` | `branch`, `version`, `changeNumber` and `buildId` from the build manifest, or `null` |
| `source.build.publicPatch` | The public patch version, e.g. `4.7.2`, from the RSI Launcher log line `SC <CHANNEL> <version>-<tag>.<change>` (`<version>`: two or three dot-separated numbers, kept as logged) whose change equals `changeNumber`; `null` when the log has no such line (never guessed from a neighbouring one) |
| `source.archiveSize` | The size of the Archive file in bytes |
| `source.archiveModified` | The modification time of the Archive file, UTC in RFC 3339 form |
| `source.toolVersion` | The version of P4K-Extractor that wrote the catalog |
| `source.exportedAt` | When the catalog was written, UTC in RFC 3339 form |
| `completeness.<kind>` | `{"status": "complete"}`, or `{"status": "partial", "filter": "…"}` when a Filter selected the Records |
| `manufacturers` | The manufacturers, ordered by GUID |
| `items` | The items, ordered by GUID |
| `ships` | The ships and ground vehicles, ordered by GUID |
| `commodities` | The commodities, ordered by GUID |
| `blueprints` | The crafting blueprints with their ingredients, all tiers, default availability and dismantle link, ordered by GUID |
| `resourceGroups` | The groups of resource types, the targets of `typeGuid`, ordered by GUID |
| `blueprintCategories` | The blueprint categories, the targets of `categoryGuid`, ordered by GUID |
| `gameplayProperties` | The gameplay properties the quality modifiers refer to, ordered by GUID |
| `qualityDistributions` | The quality distributions and their location overrides, ordered by GUID |
| `blueprintPools` | The blueprint pools with the places that reward them, ordered by GUID |
| `dismantleProcesses` | The dismantle processes, ordered by GUID |
| `paints` | The paints, each the `itemGuid` of an entry of `items` with the ships it fits, ordered by `itemGuid` |
| `craftingGlobal` | Not a kind: the global crafting parameters that apply to every dismantle |

A complete kind lists every Record of its kind in the Build, so the import may treat a Record it
knows but no longer finds as removed. A partial kind lists only what the Filter selected.

### Ordering Builds

`source.build.changeNumber` is the Perforce change number the Build was made from
(`RequestedP4ChangeNum` of the build manifest). It is a decimal integer written as a string; a
consumer compares it as a number, never as text.

- **Within a Channel it is monotonic.** Every Build a Channel published has a higher change number
  than the one before, so a consumer may refuse a catalog whose `changeNumber` is lower than that of
  the last applied catalog of the same Channel. The same number means the same Build; re-applying it
  is safe.
- **HOTFIX** is a Channel of its own and monotonic too. A HOTFIX Build later becomes the LIVE Build
  unchanged, so HOTFIX and LIVE then share one change number, one `buildId` and one `version`. A
  HOTFIX Build lies between the LIVE Builds around it, and the LIVE Build that replaces it is the
  same or higher, so ordering a HOTFIX catalog against a LIVE one by `changeNumber` is also sound
  as long as equal numbers count as the same Build.
- **Across Channels it orders the code, not the time of publication.** PTU, EPTU and TECH-PREVIEW
  Builds come from other branches: a LIVE Build with a lower change number can be published after a
  PTU Build with a higher one. A consumer keeps one last-applied `changeNumber` per
  `source.channel` and does not compare numbers of different Channels. A Build promoted to another
  Channel keeps its number, so it is never rejected there.
- **`buildId` does not order.** It is a random version-4 UUID. It identifies a Build for equality
  only.
- **`version` does not order well.** The first three parts follow the Builds, but the fourth is a
  counter of the build farm: a PTU Build had `4.10.196.33342` and the later LIVE Build
  `4.10.197.2878`. TECH-PREVIEW starts at `1.0`. Use it for display, not for order.

Evidence for the rule is in the pull request that closed krt-profit/p4k-extractor#402: 65 launcher
log lines (`SC <CHANNEL> <version>-<tag>.<change>`, the same lines as for `source.build.publicPatch`)
over 4.5.0 to 4.8.2 with no inversion in any Channel (LIVE 15 distinct Builds, PTU 25, HOTFIX 6,
TECH-PREVIEW 3), and the five installed manifests. The logs hold no EPTU line and rotate, so the
claim covers the Builds the log kept; a rollback of a Channel to an older change would break it and
would show as a lower number than the last applied one.

### Entities

Every entity carries:

| Field | Meaning |
|-------|---------|
| `guid` | The Record GUID as canonical lower-case UUID text; entities are ordered by it |
| `className` | The Record name without its type, e.g. `ACAS` for `SCItemManufacturer.ACAS` |
| `recordPath` | The record path, without `.xml` |
| `name`, `description` | `key` (the localisation key), `en` and `de` (the resolved texts), each `null` while unknown |

A resource group and a blueprint category carry `name` and no `description`.

A manufacturer adds `code`, e.g. `ACAS`.

An item takes its fields from the components of its `EntityClassDefinition`:

| Field | Source |
|-------|--------|
| `name`, `description` | `SAttachableComponentParams.AttachDef.Localization.Name` and `.Description` |
| `type`, `subType` | `AttachDef.Type` and `AttachDef.SubType`, e.g. `PowerPlant` and `Power` |
| `size`, `grade` | `AttachDef.Size` and `AttachDef.Grade`, integers; see "Semantics of `grade`" below |
| `mass` | `SEntityPhysicsControllerParams.PhysType.Mass`, a number |
| `manufacturerGuid` | The GUID the Reference `AttachDef.Manufacturer` links to, `null` for a null Reference |
| `tags` | The words of `AttachDef.Tags`, split at white space |
| `displayType` | A text: `SCItemPurchasableParams.displayType`, e.g. `@item_displayType_Pants` |
| `itemClass` | `Civilian`, `Competition`, `Industrial`, `Military` or `Stealth`, or `null` |
| `playability` | `player`, `mission` or `special`, or `null`; see "Playability" below |
| `stats` | The typed stat blocks of a weapon, shield, quantum drive, power plant, cooler or missile, `null` for any other item; see "Item stats" below |

`itemClass` takes the options of the DataForge enum `EItemClass` (without `UNDEFINED`). No
DataForge field carries it: it is read from the first line of the English `description` that
starts with `Class:`, its value trimmed of white space and matched ignoring case. Any other value
(FPS weapons use the line for the damage type, e.g. `Ballistic`), an item without that line and
an item without an English description leave it `null`. `p4kx catalog` counts the items with a
class, with another value, and without the line or an English description. The schema types the
field as a string; the five options are documented, not enumerated.

#### Semantics of `grade`

`grade` is the integer of `AttachDef.Grade`, exported unchanged; the catalog carries no letter.
The letter the game shows is not in the DataForge. For ship components it is the `Grade:` line of
the English `description`, and it follows the integer: **1 = A, 2 = B, 3 = C, 4 = D**, so A is the
lowest integer. Checked on LIVE 4.10.2 (change 12913470) against that line of every item that has
one:

| Item `type` | Integer to letter |
|-------------|-------------------|
| `Cooler`, `PowerPlant`, `QuantumDrive`, `Radar`, `Shield` | 1 = A, 2 = B, 3 = C, 4 = D, uniform |
| `JumpDrive`, `LifeSupportGenerator` | only 3 = C is consistent; grade 1 items name no letter or contradict it |
| FPS gear (`Char_Armor_*`, `Char_Clothing_*`, `WeaponPersonal`) | no `Grade:` line; grade is 1, on a few helmets and shirts 2, with no letter evidence |
| `WeaponAttachment` (ship weapon parts) | no `Grade:` line; grade 3 to 7 follows the size of the part |
| every other `type` | no `Grade:` line; grade is 1 almost everywhere |

Of the 350 items with a `Grade:` line, 331 name A to D and agree with the integer. 19 do not, and
all 19 have grade 1:

- 10 name B or C: a `Cooler` (B, and another with C), a `Radar`, a `JumpDrive`, a
  `QuantumInterdictionGenerator`, two `Misc` computers and three `WeaponMining` lasers. The text and
  the integer disagree; the Archive does not say which one the game shows.
- 9 name `Bespoke`: the ship-specific components of the Reclaimer, 890 Jump and Polaris.

The Basetool can take the letter from `grade` by `1 = A` to `4 = D` for the five component types,
and must treat grade 1 as unknown elsewhere and for those 19 items. It reads the integer, never the
`Grade:` line, which is a game text and stays away from third parties.

Distribution of `grade` over the 23280 items of LIVE 4.10.2:

| `grade` | Items |
|---------|-------|
| 1 | 21325 |
| 2 | 153 |
| 3 | 170 |
| 4 | 76 |
| 5 | 8 |
| 6 | 8 |
| 7 | 4 |
| `null` | 1536 |

`null` is an item without `AttachDef`: all 1536 also have a `null` `type`. 5 to 7 occur only on
`WeaponAttachment` parts (`Barrel`, `FiringMechanism`, `PowerArray`, `Ventilation`), 3 and 4 on
those parts too. No value is below 1 or above 7.

#### Item stats

`stats` holds raw facts of the Archive and no derived value (ADR-0065): every number is the stored
field in the stored unit, `p4kx` computes no damage per second, no cycle time, no acceleration and
no fuel rate. The Basetool owns those formulas. `stats` is `null` unless the item has at least one
of the six blocks; every block is `null` when the item does not have it, and a field inside a
present block is `null` when the Archive does not state it, never 0. None of the fields is a game
text. The additions are additive in version 1 (ADR-0054); `kind` of a fire mode and
`trackingSignalType` are closed sets that the schema keeps as strings.

| Block | Present when | Fields and sources |
|-------|--------------|--------------------|
| `weapon` | the item has `Components[SCItemWeaponComponentParams]` (ship weapons and FPS weapons) | `fireActions`: per element of `fireActions`, in stored order, with a sequence (`sequenceEntries`) flattened to its entries: `kind` (the struct, e.g. `SWeaponActionFireRapidParams`), `name`, `fireRate` (rounds per minute), `pellets` and `ammoCost` of `launchParams`, `heatPerShot`, and for a charged mode `chargeTime`, `overchargeTime`, `cooldownTime`; a charged or wrapped mode reads `fireRate`, `launchParams` and `name` from its `weaponAction` when it has none of its own. `ammoCapacity`: `Components[SAmmoContainerComponentParams].maxAmmoCount`. `ammo`: the `AmmoParams` Record that `SAmmoContainerComponentParams.ammoParamsRecord` links to, `null` without a readable one: `speed`, `lifetime`, `damage` (the damage types of `projectileParams.damage`), `detonationDamage`, `detonationMinRadius` and `detonationMaxRadius` (from `explosionParams`) |
| `shield` | `Components[SCItemShieldGeneratorParams]` | `maxHealth` (`MaxShieldHealth`), `maxRegen` (`MaxShieldRegen`), `decayRatio` (`DecayRatio`), `downedRegenDelay` (`DownedRegenDelay`), `damagedRegenDelay` (`DamagedRegenDelay`) |
| `quantumDrive` | `Components[SCItemQuantumDriveParams]` | `fuelRequirement` (`quantumFuelRequirement`, as stored), `jumpRange`, `disconnectRange`, and of `params`: `driveSpeed`, `cooldownTime`, `spoolUpTime`, `stageOneAccelRate`, `stageTwoAccelRate`, `engageSpeed` |
| `powerPlant` | `AttachDef.Type` is `PowerPlant`, ignoring case | `powerOutput`: the amount per second of the first `ItemResourceDeltaGeneration` of `Components[ItemResourceComponentParams]` whose `resource` is `Power` |
| `cooler` | `AttachDef.Type` is `Cooler`, ignoring case | `coolingRate`: as `powerOutput`, for the resource `Coolant` |
| `missile` | `Components[SCItemMissileParams]` | `damage` (the damage types of `explosionParams.damage`), `explosionMinRadius`, `explosionMaxRadius`, `trackingSignalType`, `lockTime`, `lockingAngle`, `lockRangeMin`, `lockRangeMax` (of `targetingParams`) and `linearSpeed` (of `gcsParams`) |

A damage object has `physical`, `energy`, `distortion`, `thermal`, `biochemical` and `stun`
(`DamagePhysical` and so on); a type the struct does not hold is `null`. An item with at least one
of the six blocks also carries the inputs of its signatures and durability, each `null` without the
component: `power` (`EntityComponentPowerConnection`: `powerBase`, `powerDraw`, `powerToEm`,
`decayRateOfEm`), `heat` (`EntityComponentHeatConnection`: `thermalEnergyBase`, `thermalEnergyDraw`,
`temperatureToIr`, `overheatTemperature`) and `health` (`SHealthComponentParams.Health`). Items
without a block carry none of them.

The block of a power plant or a cooler follows from the type because both share the resource
component; every other block follows from its component, and no class name or record name enters
a rule. A damage object is read from the first struct below the named parent that states a damage
type, and a generation from the structs of its type below the resource component, so a field the
game moves turns into `null` rather than into a wrong number; the real-Archive test
(`real_archive_item_stats`) counts the filled fields of each block.

Size: an item without stats adds `"stats":null` (14 bytes, about 0.3 MB for the 23,280 items of
LIVE); an item with a block adds a few hundred bytes, two to three thousand items about 1 MB,
which compresses well because the keys repeat.

A ship takes its fields from `VehicleComponentParams`:

| Field | Source |
|-------|--------|
| `name`, `description` | `vehicleName` and `vehicleDescription` |
| `role`, `career` | Texts: `vehicleRole` and `vehicleCareer`, e.g. `@vehicle_class_lightfreight` |
| `crewSize` | `crewSize`, an integer |
| `manufacturerGuid` | The GUID the Reference `manufacturer` links to, `null` for a null Reference |
| `vehicleDefinition` | `vehicleDefinition`, the path of the vehicle XML |
| `playability` | `player`, `ai`, `mission` or `special`, or `null`; see "Playability" below |
| `vehicleKind` | `spaceship` or `groundVehicle`, or `null`; see "Ship base data" below |
| `size` | `Components/SAttachableComponentParams/AttachDef/Size`, an integer, `null` without an `AttachDef` |
| `mass` | The hull mass in kilograms; `null` when unknown, never 0; see "Ship base data" below |
| `massBasis` | What `mass` is the sum of: `parts` or `physics`; `null` exactly when `mass` is |
| `hullHealth` | The hull hit points, `null` when unknown; see "Ship base data" below |
| `cargoScu` | The cargo capacity in SCU: 0 without a cargo grid, `null` when unknown; see "Ship base data" below |
| `cargoGridsWithIssue` | How many cargo grids have no readable capacity; above 0, `cargoScu` is a lower bound |

#### Ship base data

`mass`, `massBasis`, `hullHealth`, `cargoScu`, `cargoGridsWithIssue`, `size` and `vehicleKind` are
facts only (no game text) and come from the ship base data of `p4kx ships`
([ship-base-data.md](ship-base-data.md), ADR-0061): the catalog build reads each ship once with
`p4kx_export::ShipReader` in the same pass that reads the other ship fields, so the ship base walk
runs once, not twice. Every value is `null` when the Archive does not state it, and a reader never
sees 0 for unknown. The exports agree value for value; the catalog leaves out the per-value
sources (file and field), the thresholds, the Loadout and the cells, which stay in `p4kx ships`.
The shapes are additive within version 1 (ADR-0054, ADR-0063); `vehicleKind` and `massBasis` are
closed sets that the schema keeps as strings.

| Field | Rule and source |
|-------|-----------------|
| `mass` | The sum of the `mass` of every part of the vehicle implementation (`vehicleDefinition`) after the ship's Modification is applied, rounded to four decimals; a sum too large to round to a finite number counts as unknown (so does `hullHealth`). Landing gear, Loadout items and docked vehicles are not part of it (#305). Without part masses, the mass of the rigid physics controller (`Components/SEntityPhysicsControllerParams/PhysType/Mass`, `massBasis` `physics`) |
| `hullHealth` | The sum of the positive `damageMax` of the parts, except parts holding an item port flagged `uneditable` or `invisible`; `null` without a vehicle implementation or when no part states one |
| `cargoScu` | The sum of the capacity of the `CargoGrid` Entities of the default Loadout (docked vehicles and templates are not entered): an open container is `x * y * z / 1.953125` of its `interiorDimensions`, a closed one its `capacity` in standard cargo units. 0 for a ship with no grid, `null` when it has grids and none is readable |
| `cargoGridsWithIssue` | The grids whose capacity cannot be read, e.g. a dangling Reference to the `InventoryContainer`. Above 0, `cargoScu` counts only the readable grids and is a lower bound |
| `vehicleKind` | `groundVehicle` for a Record under `entities/groundvehicles`, `spaceship` for one under `entities/spaceships` whose vehicle implementation has no wheel (`SubPartWheel`) or tread (`Tread`) part; a missing or unreadable vehicle implementation counts as no wheels, so such a Record under `entities/spaceships` is a `spaceship`. `null` when the evidence disagrees (a wheeled or tracked Record under `entities/spaceships`) or the path names neither folder |

LIVE anchors, read by the real-Archive test: Harbinger 230,404 kg, 48,490 hull, 0 SCU; Freelancer
MAX 341,408 kg, 35,650 hull, 120 SCU; Carrack 3,275,858 kg, 88,000 hull, 456 SCU; Constellation
Andromeda 386,857 kg, 180,844 hull, 96 SCU (without the docked snub craft). The Javelin and the
Hornet F7C have `cargoScu` `null`: their cargo grids refer to `InventoryContainer` Records that do
not exist.

A ship with `cargoGridsWithIssue` above 0 and a `cargoScu` number may carry more than the number
says.

On LIVE the 1,087 catalog ships split into 1,048 `spaceship` and 39 `groundVehicle`, none `null`:
no Record under `entities/spaceships` has a wheel or tread. The folder `entities/groundvehicles`
holds 40 Records; the 40th, `RSI_Ursa_Rover_Unmanned_Template`, is left out of the catalog as a
development leftover (the whole word `template`, see "Selection").

**Minimum crew.** The game has none. A survey of the field names of all 1,144 ship Records on LIVE
finds only `crewSize` (on `VehicleComponentParams` and on `StaticEntityClassData/displayParams`) and
`crewManifest` (the NPC crew of AI variants, used by `playability`). No field named minimum, seat
or crew minimum exists, so `crewSize` stays the only crew value.

**Outer dimensions.** No field states the outer size of a ship in metres. The survey finds the
item-inventory fields `AttachDef/inventoryOccupancyDimensions` and
`inventoryOccupancyLocalBoundsMin` / `Max` on every ship Record, plus `maxBoundingBoxSize` and
`hasLocalBounds`. They describe how a ship takes up inventory space, not its hull, and their values
for ships are not verified, so the catalog exports no dimensions.

#### Playability

The catalog lists every ship and item Record of the Build, among them AI, mission and event
variants that no player can own, buy or fit. `playability` tells them apart. It is a closed set of
strings; the schema keeps a string, so a new value is additive and a reader maps a value it does
not know to `null` (ADR-0054). It is `null` where no rule below applies: the value is never
guessed. The rules read Record data only (components, fields, References, Tags, record-path
folders); no class name or Record name enters them.

| Value | Meaning |
|-------|---------|
| `player` | Has the structural markers of a player entity |
| `ai` | Part of the AI; ships only |
| `mission` | Part of a mission: an unmanned ship or an item of a mission folder |
| `special` | Neither: a ship with `inclusionMode` `DoNotInclude`, or an item without a player-visible name |

For a ship or ground vehicle the first rule that holds decides. `inclusionMode` is
`StaticEntityClassData[EAEntityDataParams].inclusionMode` (`EAEntityDataParams`, believed to
govern Electronic Access / Arena Commander inclusion). It correlates on LIVE with hangar-listed
ships; it is used as evidence, not as a definition. Any value other than `ReadyToInclude` and
`DoNotInclude` leads to `null`, unless rule 1 or 2 holds.

1. `ai` when `Components[SEntityComponentCrewManifestParams].crewManifest` links to a
   `CrewManifest` Record whose `crew` array is not empty (the NPC crew; the empty manifest
   `crewmanifest_null` does not count). For a Record under `entities/spaceships/` also when
   `Components[VehicleComponentParams].criticalPartExplosionChance` is above zero, or when
   `Components[VehicleComponentParams].aiEngineeringOptions.powerPlantsCanExplodeOnDeath` is true
   while `inclusionMode` is not `ReadyToInclude`. Ground vehicles all carry these two hull values,
   so they are not read there.
2. `mission` when one of the Record's `tags` links to a `Tag` Record whose `tagName` is `Unmanned`
   (the salvage, derelict and wreck targets of missions).
3. `special` when `inclusionMode` is `DoNotInclude` (templates, wrecks, sentries, event props).
4. `player` when `inclusionMode` is `ReadyToInclude` and, for a spaceship,
   `powerPlantsCanExplodeOnDeath` is not true.
5. Also `null` when the crew manifest or a tag the Record links to could not be read (the
   catalog reports that Record as a failure), since its evidence is missing; checked before
   rule 1.
6. Otherwise `null`: no `EAEntityDataParams`, another `inclusionMode`, or a `ReadyToInclude` spaceship whose hull
   explodes on death and that no crew manifest marks (LIVE: Stingray, Skylark, Railen and the
   `ai_template` ships mix here; the data does not separate them).

For an item the first rule that holds decides:

1. `player` when `Components[SCItemPurchasableParams].displayName` is a localisation key that is
   not a placeholder key: the item has a shop display.
2. `mission` when the record path lies under `entities/scitem/mission_entities/` or
   `entities/scitem/missionstorage/`.
3. `special` when `AttachDef.Localization.Name` is no key or a placeholder key: the item has no
   player-visible name (doors, props, decals, internal parts).
4. Otherwise `null`: a named item without a shop display (loot, attachments, parts); the Records
   do not say whether a player can obtain it.

No item is `ai`: items carry no AI marker the rules can read.

The development-leftover token rule (see "Selection") runs first and is unchanged: a Record it
leaves out is no entity and has no playability. Known limit: a few leftover-like ships whose names
the token list does not hold (`temp`, `display`, `hijacked` variants; about 20 on LIVE) carry no
structural marker and read as `player`.

**The Basetool seeds only `player` ships and items, and treats them as candidates.** `player`
means the Records carry the structural markers of a player entity (ships: hangar-ready inclusion
mode and no AI crew or hull; items: a shop display name). It is not proof that a player can obtain
it: about 20 leftover-like ships on LIVE are `player`, and items carry no AI marker. The Basetool
matches every other value and `null` against what it knows and seeds none of them. A `null` ship
(LIVE: Stingray, Skylark, Railen) is added by hand.

`p4kx catalog` prints the count per value and the unclassified count for items and ships; `--json`
lists them under `playability`. LIVE, 2026-10-21:

| Kind | `player` | `ai` | `mission` | `special` | `null` |
|------|---------:|-----:|----------:|----------:|-------:|
| ships | 324 | 690 | 25 | 35 | 13 |
| items | 7,954 | 0 | 522 | 8,570 | 6,234 |

A commodity is an `EntityClassDefinition` under `entities/commodities` or a `ResourceType` Record that
an ingredient uses and no commodity Record carries (see below). A commodity Record takes its fields
from the entity:

| Field | Source |
|-------|--------|
| `name` | `SCItemPurchasableParams.displayName` |
| `description` | `EntityUIDisplayParams.displayDescription` in `StaticEntityClassData` |
| `typeGuid` | The GUID of the `ResourceTypeGroup` Record that directly lists the `ResourceType` in its `resources` (19 groups on LIVE: Metal, Mineral, Gas, …); `null` without a `ResourceType` or a group |
| `subtypeGuid` | The GUID of the `ResourceType` Record that the first entry of `ResourceContainer.defaultComposition` links to; `null` without one, e.g. `Special_HolidayLetter` |
| `displayType` | A text: `SCItemPurchasableParams.displayType`, e.g. `@items_commodities_type_metal` |

A name or description that is missing or a placeholder key is taken from the `ResourceType` Record
`subtypeGuid` links to (`displayName`, `description`) instead. The game holds no field named type
or subtype for a commodity; the groups of `ResourceType` Records are the types, the `ResourceType`
is the subtype.

A `ResourceType` Record that the ingredients of a blueprint use and that no commodity Record
carries is listed as a commodity as well, so that every resource ingredient points at a catalog
entry (LIVE: Aslarite, Ouratite, Iron, Stileron, Lindinium, Riccite, Tin, Torite, Savrilium and
Pressurized Ice). Its `guid` is the GUID of the `ResourceType`, `className`, `recordPath`, `name`
and `description` are those of the `ResourceType`, `typeGuid` is its group, `subtypeGuid` is its own
GUID and `displayType` has no key. With a `--filter`, such an entry is listed only when its record
path matches.

The join from an ingredient to its commodity is `resourceGuid` = `subtypeGuid`. A `ResourceType`
that a commodity Record carries yields no second entry, so every `resourceGuid` of the catalog
matches exactly one commodity (checked against LIVE). A `ResourceType` that several commodity
Records share (on LIVE the Slam variants and the med pens) matches several; no recipe uses one.

A blueprint (`CraftingBlueprintRecord`) takes its fields from `blueprint`:

| Field | Source |
|-------|--------|
| `className`, `key` | The Record name without its type, e.g. `BP_CRAFT_behr_lmg_ballistic_01_mag`; both hold it |
| `name` | `blueprintName` |
| `categoryGuid` | The GUID `category` links to |
| `producedItemGuid` | The GUID `processSpecificData.entityClass` links to; `null` for a Reference to a Record the DataForge lacks |
| `craftTimeSeconds` | `costs.craftTime` of the first tier (`days`, `hours`, `minutes`, `seconds`) in whole seconds, rounded |
| `ingredients` | The costs of the first tier, see below |
| `tiers` | Every tier in stored order, see "Tiers" below; the first one is the tier `craftTimeSeconds` and `ingredients` describe |
| `defaultAvailable` | `true` when the blueprint is in `CraftingGlobalParams.defaultBlueprintSelection`, the blueprints a player has by default |
| `dismantle` | What dismantling the produced item returns, see "Dismantle returns" below; `null` without one |

`craftTimeSeconds` and `ingredients` keep their meaning; they are derived from the first tier, so
the cost tree is read once.

An ingredient is one `CraftingCost_Resource` or `CraftingCost_Item` leaf of `mandatoryCost` and
`optionalCosts`. Nested `CraftingCost_Select` costs are flattened depth first in stored order:

| Field | Meaning |
|-------|---------|
| `resourceGuid` | The GUID of the `ResourceType` Record of a resource line, otherwise `null` |
| `itemGuid` | The GUID of the entity of an item line, otherwise `null` |
| `quantityScu` | `quantity.standardCargoUnits` of a resource line, otherwise `null` |
| `quantityUnits` | `quantity` of an item line, otherwise `null` |
| `minQuality` | `minQuality` |
| `slot`, `slotKey` | `debugName` and `displayName` of the nearest enclosing selection that has a name; a placeholder `slotKey` is `null` |

A resource group (`ResourceTypeGroup`, 19 on LIVE: Metal, Mineral, Gas, …) is the type of a
commodity; a commodity's `typeGuid` is the `guid` of the group that lists its `ResourceType`. It
takes its fields from the Record:

| Field | Source |
|-------|--------|
| `className` | The Record name without its type, e.g. `Metal` |
| `name` | `displayName`, e.g. `@items_commodities_type_metal`; a placeholder key (`@LOC_PLACEHOLDER`, 7 groups on LIVE) leaves `en` and `de` `null`, and the Basetool falls back to `className` |
| `parentGuid` | The GUID of the group that lists this group in its `groups`, `null` for a top-level group; the first group in Record order wins when several list it |

Groups nest one level on LIVE: Metal lists Alloy and UnrefinedOres, Mineral lists Raw_Minerals,
Nonmetal lists Halogen, Organic lists Food, ProcessedGoods lists SyntheticMaterials, Vice and
Bulk_Supplies. A commodity points at the group that lists its `ResourceType` directly, so a
commodity of Alloy has the child as `typeGuid`, not Metal. `description`, the thumbnail paths and
the lists `resources` and `groups` are not exported: the descriptions are game texts, the paths are
media, and the membership is `typeGuid` of the commodities. All groups share one `recordPath`, the
Record file `resourcetypedatabase`; `className` and `guid` tell them apart.

A blueprint category (`BlueprintCategoryRecord`, 21 on LIVE) is the target of the `categoryGuid` of
a blueprint. The Record holds no field at all, so the category has `guid`, `className` (e.g.
`FPSArmours`, `VehicleComponentS2`), `recordPath` and a `name` without a key, which stays `null`
until the game adds one; the game localises no category. Categories do not nest: the Record has no
parent, and the `BlueprintCategoryDatabaseRecord` beside them only lists them. Some categories
carry no blueprint on LIVE (`FuseBattery`, `Medical`, `Utility`, and the examples `Example1`,
`Example2`, `DismantleExample1` and `RefiningExample1`); they stay, as the kind is complete by
Record.

Every `typeGuid` of a commodity and every `categoryGuid` of a blueprint resolves to a `guid` of
`resourceGroups` or `blueprintCategories` (checked against LIVE; a commodity without a
`ResourceType` or group, 1 on LIVE, and a blueprint without a category have `null`). With a
`--filter`, a kind is partial and the references need not resolve.

### Tiers

`tiers` lists every `CraftingBlueprintTier` in stored order. Every tier has:

| Field | Meaning |
|-------|---------|
| `craftTimeSeconds` | `costs.craftTime` of the tier in whole seconds, rounded; `null` without one |
| `mandatory` | The mandatory cost as a cost line, `null` without one |
| `optional` | The optional costs as cost lines in stored order |

A cost line is a selection, a resource line or an item line. The tree keeps what `ingredients`
flattens, so a selection stays a selection:

| Field | Meaning |
|-------|---------|
| `kind` | `select`, `resource` or `item`. A closed set the game may extend; the schema keeps it a string and a reader maps an unknown value to nothing |
| `count` | `select`: how many options the player chooses ("choose n"); fewer than `options` offers means the player may leave options out (3 of the 1,618 LIVE blueprints) |
| `slot`, `slotKey` | `select`: `debugName` and `displayName` of `nameInfo`; a placeholder `slotKey` is `null` |
| `options` | `select`: the options as cost lines in stored order, otherwise `[]` |
| `resourceGuid`, `quantityScu` | `resource`: the `ResourceType` Record and the quantity in SCU, before the quantity multiplier |
| `itemGuid`, `quantityUnits` | `item`: the entity and the number of units, before the quantity multiplier |
| `minQuality` | `resource` and `item`: `minQuality` |
| `quantityMultiplier` | The factor of `CraftingCostContext_QuantityMultiplier` (1.15 on 3 lines of LIVE), `null` without one. The quantities stay the stored ones; the consumer applies the factor |
| `compositionInclusion` | The option of `CraftingCostContext_ResultCompositionInclusion`, e.g. `Include`; `null` without one |
| `modifiers` | `CraftingCostContext_ResultGameplayPropertyModifiers`: per modifier `propertyGuid` (a `gameplayProperties` entry) and `ranges`, each with `kind` (`multiplier` for factors, `additive` for summands; a closed set the game may extend), `startQuality`, `endQuality`, `atStart` and `atEnd`, linear in the quality between the two ends |

A selection carries the context of the selection itself in the same fields.

LIVE holds exactly one tier per blueprint on all five Channels, so the real-Archive test checks
that tiers and first-tier fields agree, and the multi-tier case is covered by a synthetic
Archive (`catalog_crafting_tiers`), which checks a blueprint with two tiers, selections,
multipliers and modifiers.

### Quality, pools and dismantle

Four kinds hold what a blueprint refers to; each entry has `guid`, `className` and `recordPath`,
and the ordering, completeness and leftover rules of every other kind apply (a `--filter` matches
the record path and marks the kind partial).

| Kind | Fields |
|------|--------|
| `gameplayProperties` | `name`: the localisation key `propertyName` and its texts; a placeholder key leaves `en` and `de` `null`. The target of a modifier's `propertyGuid` |
| `qualityDistributions` | `distribution`: a normal distribution `kind`, `min`, `max`, `mean`, `stddev` (`null` for an override Record or a value beyond a 32-bit float), and `overrides`: per location a `locationGuid` and its `distribution` (empty for a default Record). Both Record types of `crafting/qualitydistribution` are listed |
| `blueprintPools` | `entries`: per entry `blueprintGuid` and `weight`; `totalWeight`, the sum of the positive weights; `rewardSources`: every struct of any Record with a Reference to the pool (on LIVE the `BlueprintRewards` result of a contract) with `recordGuid`, `recordPath`, `contractId`, `contractName` (both from the nearest enclosing struct that has an `id`) and `chance`, ordered by `recordGuid` and then in stored order |
| `dismantleProcesses` | `efficiency` (0.5 on LIVE) and `timeSeconds` (15 on LIVE) of the `GenericCraftingProcess_Dismantle` |

A blueprint's chance from a source is `chance` x the entry's `weight` / `totalWeight`; a source
without a chance or with one outside 0 to 1, a pool without a positive weight and an entry with a
weight of zero or less give none. The quality distributions carry no link to a blueprint or source:
the Records do not hold one, so the catalog lists them as they are.

Not represented: the `qualityquantization` Records (quantised quality steps per `ResourceType`)
are not read; they are added when the Basetool needs them (#409). Everything else of the issue's
list is in the catalog.

#### Dismantle returns

`dismantle` of a blueprint is `{"processGuid", "tierIndex"}`: the dismantle process that applies to
the item the blueprint produces and the index into `tiers` of the tier whose costs the item
returns, `null` for a blueprint without a tier. `craftingGlobal` holds
`dismantleExcludedResourceGuids` (the `ResourceType` Records a dismantle never returns) and
`dismantleExcludedItemGuids` (the entities that cannot be dismantled; their blueprints have no
`dismantle`). The contract:

- The Build must hold exactly one dismantle process. With several, no blueprint gets a
  `dismantle`, the processes are still listed, and `p4kx catalog` warns and exits with code 1.
- The item returns the share `efficiency` of the costs of the first tier (`tierIndex` 0), except
  the resources in `dismantleExcludedResourceGuids`.
- A `select` is not resolved: the Records do not say which options a dismantle returns, so the
  consumer applies the `count` itself. The `quantityMultiplier` is not applied.

These are assumptions of the library, not verified against the game.

### Paints

A paint (`paints`) is an item of the type `Paints`. The kind refers to the item and adds no
description, no name and no media: name, record and description stay on the entry in `items`, so
no GUID occurs twice among the `guid` fields of the catalog:

| Field | Source |
|-------|--------|
| `itemGuid` | The `guid` of the paint's entry in `items`; paints are ordered by it |
| `shipGuids` | The `guid` of every ship of `ships` the paint fits, ordered by GUID; empty when it fits none |

A paint is selected like an item (the same record-path prefixes, the same development leftovers
left out), whose `AttachDef.Type` is `Paints` ignoring case; LIVE holds 1,132 such Records, 1,064
after the leftovers (`paint_*_template` and the like). Two of them are weapon skins
(`Skin_Gold`, `Skin_Space_Kitty`), which no ship port takes.

The rule is structural and never reads a name. A paint fits a ship when one paint port of the ship
accepts it, i.e. when every tag in the port's required tags is a tag of the paint's `AttachDef.Tags`
or has a tag of the paint's `Tags` below it in the tag tree (`TagTree`, ADR-0052). Tag names compare
ignoring case; the paint's marker tag `@Name` is a tag of its own. A port without required tags
accepts nothing. The paint ports of a ship are the union of:

- the `SItemPortDef` entries of `SItemPortContainerComponentParams.Ports` of its Record, which hold
  an `SItemPortDefTypes` with `Type` `Paints`; the tags are `RequiredPortTags`; this is where the
  newer ships keep the port (e.g. the Paladin: `Paint_Paladin`);
- the `ItemPort` elements of its vehicle implementation (`VehicleComponentParams.vehicleDefinition`)
  that take `Paints` (a child `Types/Type` of `type` `Paints`), with the Modification named by
  `VehicleComponentParams.modification` applied. The implementation is read by`n  `p4kx_assembly::VehicleImplementation` after the rules of ADR-0050 (patch file, `Elem` entries
  by id, fallback file names); the tags are the `requiredTags` the patched port has (the Omega
  variant of the Mustang requires `Paint_Omega` instead of `Paint_Mustang`).

The `RequiredTags` of a paint and the `portTags` of a port are not compared: the Omega paint
requires `Paint_Omega` while the port of the Omega still lists `Paint_Mustang`. A ship whose
`modification` is empty keeps the base port, so the Idris-M fits the paints of the Idris-P (the
game selects its Military paints elsewhere).

`p4kx catalog` counts the paints, those that fit a ship and those that fit none, the ships that
have at least one paint and the ships whose vehicle implementation cannot be read, or whose patch file or Modification is
not applied as it names, except that a patch file the Archive lacks is a fact of the game data; `CatalogSummary::paints()` returns them, and an unreadable vehicle file is also a warning of
`CatalogSummary::warnings()`. Measured on LIVE (4.7): 1,064 paints, 1,039 assigned, 25 not
assigned, 1,046 ships with paints (PTU 1,010 of 1,035, EPTU 992 of 1,017, TECH-PREVIEW 1,039 of 1,063). Of the 25, 16 are paints of the ARGO ATLS, an actor rather than
a ship, 2 are weapon skins and 7 are the default livery of a ship (`Paint_Ballista_Base`,
`Paint_Gladius_Default`, `Paint_Carrack_Default`, `Paint_Hurricane_Default`, the Vanguard
Harbinger, Sentinel and Hoplite) whose tags no port requires. With a `--filter` the paints and the ships are
both filtered, so a paint lists only ships the filter selected; both kinds are partial.

### Crafting data

`p4kx_export::read_crafting(&forge, filter, cancel)` returns a `Crafting` with everything the
`blueprints` kind and the crafting kinds carry; the catalog is built from it, so library and
catalog never drift. It reads the DataForge in parallel (0.6 s on LIVE, plus the reference index for
the reward sources) and leaves out a Record it cannot read, listing it under `failures`. Every
list is ordered by Record GUID. The values are facts and identifiers only: no description, no
lore, no media.

The blueprints are selected by the one rule behind the `blueprints` kind (`selects_blueprint`:
prefix, type, Filter, leftover tokens). A `Filter` keeps only the blueprints and pools it matches;
the other lists are always read, and the catalog then keeps the properties, quality distributions
and dismantle processes whose record path matches.

A reference index that holds more than 2^23 References, or cannot read a Record, loses reward
sources, and says so: `Crafting::warnings` holds `RewardSourcesTruncated` or
`UnreadableReferenceRecords` with the count, and `p4kx catalog` prints the warning and exits with
code 1 (`CatalogSummary::warnings()`; `is_complete()` is then `false`).

The Rust names of the library:

| Field | Source |
|-------|--------|
| `blueprints` | The same set as the `blueprints` kind. Each has `produced_item` and `tiers`: per tier `craft_time_seconds`, `mandatory` and `optional` as a tree of `Cost` |
| `Cost::Select` | `count` ("choose n") with its `options`, `slot` and `slot_key`; `chooses_fewer()` tells whether the player may choose fewer than offered |
| `Cost::Resource`, `Cost::Item` | As an ingredient (`resource` / `item`, `quantity_scu` / `quantity`, `min_quality`), before the context |
| `CostContext` | `quantity_multiplier` (`CraftingCostContext_QuantityMultiplier`), `composition_inclusion` (`Include` or `Exclude`) and `modifiers` (`CraftingCostContext_ResultGameplayPropertyModifiers`): per gameplay property the linear ranges `start_quality`, `end_quality`, `at_start`, `at_end` as a factor (`Multiplier`) or a summand (`Additive`) |
| `properties` | `CraftingGameplayPropertyDef` Records under `crafting/craftedproperties`: GUID, `class_name`, `record_path` and the localisation key of the name |
| `quality_distributions` | The Records under `crafting/qualitydistribution`: a default distribution (`min`, `max`, `mean`, `stddev` of a normal distribution) or the list of location overrides, each a location GUID with its distribution |
| `pools` | `BlueprintPoolRecord` Records under `crafting/blueprintrewards`: entries with a blueprint GUID and a `weight`; `total_weight()` is the sum of the positive weights |
| `reward_sources` | Every struct of any Record that holds a Reference to a pool (on LIVE the `BlueprintRewards` result of a contract): the Record, the `id` and `debugName` of the nearest enclosing struct that has an `id` (both from the same struct), the pool and the `chance` |
| `reward_chances()` | Per blueprint the chance of each source: `chance` x `weight` / `total_weight()`; a source whose chance is outside 0 to 1 and an entry with a weight of zero or less give none |
| `dismantle` | The `GenericCraftingProcess_Dismantle` Records under `crafting/blueprints/dismantle`: `efficiency` and `dismantleTime` in seconds (LIVE: one process, 0.5 and 15 s) |
| `global` | `CraftingGlobalParams`: the default blueprints (`defaultBlueprintSelection`), the resources dismantling never returns and the entities that cannot be dismantled |
| `dismantle_return()` | Links a blueprint to the only dismantle process and to the item it produces, unless the item is on the item blacklist; `None` for none or several processes. It carries the `tier` (the first) and the `excluded_resources` |

LIVE holds exactly one tier per blueprint (also PTU, EPTU, TECH-PREVIEW and HOTFIX); the library
reads as many as a Record holds. The dismantle Record is generic, not one per item: the link from
an item to its returns runs through the blueprint that produces it.

The assumptions of `dismantle_return()` are those under "Dismantle returns" above: the first tier,
no resolved `Select`, no applied quantity multiplier, the resource blacklist exposed as
`excluded_resources` and never applied to an amount.

A blueprint's `name` is almost always a placeholder key. The Basetool names a blueprint through
`producedItemGuid`, the item it produces, not through `name`.

Manufacturer GUIDs are always taken from References, never from embedded Records (ADR-0004).

### Texts

`key` is the localisation key as the Record holds it, e.g. `@manufacturer_NameACAS`; an empty key
is `null`. `en` and `de` are its texts from the Archive's `global.ini` of that language:

| Language | Entry |
|----------|-------|
| `en` | `Data\Localization\english\global.ini` |
| `de` | `Data\Localization\german_(germany)\global.ini` |

Each line `key=value` of a `global.ini` holds one text. Keys match without the leading `@` and
ignoring case; a suffix after a comma (`ASD_01,P`) is not part of the key. When a `global.ini`
holds both `key=` and `key,P=`, the lookup keeps both lines and uses the text of the plain key,
unless that is empty and the suffixed one is not (the Archive's LIVE `global.ini` files hold no
such pair; English has 13,504 `,P` keys, all without a plain twin). The text is the rest of
the line as the game stores it, except that the escape `\n` becomes a line feed; markup, a lone
carriage return and every other sequence stay as they are.

A `global.ini` is read as UTF-8 with or without a byte order mark, or as UTF-16. Bytes that are no
valid text become U+FFFD and lines without `=` are skipped, so a damaged line costs only itself;
`p4kx catalog` names every such Entry in a `note:` line on stderr and the catalog stays as it is.
Only a `global.ini` over 64 MiB or with more than 1,048,576 keys fails the catalog, with exit
code 3.

A key the language's `global.ini` lacks or holds with an empty text (`key=`), or a language the
Archive holds no `global.ini` for, leaves that text `null` and keeps the key, so the basetool can
resolve it later; it counts as unresolved.

The placeholder keys `@LOC_EMPTY`, `@LOC_PLACEHOLDER`, `@LOC_UNINITIALIZED` and `@blank_space`
(the game's filler for an empty line, which Records use 372 times; ignoring case,
`PLACEHOLDER_KEYS` in `crates/p4kx-localization/src/lookup.rs`) mean "no text": the key stays,
`en` and `de` are `null` even where the `global.ini` holds a text for them such as
`<= PLACEHOLDER =>`, and they count as placeholders, neither resolved nor unresolved.

`p4kx catalog` counts the resolved, unresolved and placeholder texts per language in its summary.

## Selection

| Kind | Record-path prefix | Type |
|------|--------------------|------|
| `manufacturers` | `libs/foundry/records/scitemmanufacturer/` | `SCItemManufacturer` |
| `items` | `libs/foundry/records/entities/scitem/`, `libs/foundry/records/entities/legacyitems/` | `EntityClassDefinition` |
| `ships` | `libs/foundry/records/entities/spaceships/`, `libs/foundry/records/entities/groundvehicles/` | `EntityClassDefinition` |
| `commodities` | `libs/foundry/records/entities/commodities/` | `EntityClassDefinition` |
| `blueprints` | `libs/foundry/records/crafting/blueprints/crafting/` | `CraftingBlueprintRecord` |
| `resourceGroups` | `libs/foundry/records/resourcetypedatabase/` | `ResourceTypeGroup` |
| `blueprintCategories` | `libs/foundry/records/crafting/blueprintcategories/` | `BlueprintCategoryRecord` |
| `gameplayProperties` | `libs/foundry/records/crafting/craftedproperties/` | `CraftingGameplayPropertyDef` |
| `qualityDistributions` | `libs/foundry/records/crafting/qualitydistribution/` | `CraftingQualityDistributionRecord` and `CraftingQualityLocationOverrideRecord` |
| `blueprintPools` | `libs/foundry/records/crafting/blueprintrewards/` | `BlueprintPoolRecord` |
| `dismantleProcesses` | `libs/foundry/records/crafting/blueprints/dismantle/` | `CraftingBlueprintRecord` whose process is a `GenericCraftingProcess_Dismantle` |
| `paints` | Those of `items`; only the items whose `AttachDef.Type` is `Paints` | `EntityClassDefinition` |

The blueprints and the four crafting kinds below them are read by `read_crafting`, with the one
selection rule behind it (`selects_blueprint` for the blueprints); `Kind::rule()` of
`qualityDistributions` names the first of its two types.

The resource-group prefix also holds the 207 `ResourceType` Records and the `ResourceTypeDatabase`
Record of LIVE, and the category prefix a `BlueprintCategoryDatabaseRecord`; the type selects only
the groups and the categories.

The blueprint prefix leaves out `crafting/blueprints/dismantle/`, whose Record is no recipe. The
`$templates` folders below it hold 27 real blueprints, each the only recipe for its item, and
stay: the token `templates` is not `template`.

`mining/mineableelements` is not a kind. Its 46 Records only link a mineable to its `ResourceType`
and hold no commodity of their own; the `ResourceType` Records enter through the ingredients that
use them.

An item without an `SAttachableComponentParams` component, such as a door or a shop counter, is
still an item; its definition fields are then `null` and `tags` is empty.

The prefix ignores case and the kind of separator; the type is the Record's struct, matched
exactly. `--filter` further selects by record path with the rules of `p4kx search` and marks
every kind partial.

Development leftovers are left out and counted as skipped: a selected Record whose record path or
name holds one of the tokens `test`, `template`, `dummy`, `debug`, `deprecated` or `dev` as a
whole word, ignoring case. Words are the runs of ASCII letters and digits, so `paint_test_01` is
left out while `contestant` stays.

A selected Record that cannot be read, such as one whose struct layout is corrupt, does not stop
the catalog: it is left out, counted as a failure and reported with its record path, GUID and
reason. `p4kx catalog` prints each failure on stderr, lists them under `failed` in `--json` and
exits with code 1, with the catalog written; the desktop app lists them in the summary. The
catalog file itself, its schema and its completeness are not affected.

## Determinism and size

Equal Archives and an equal export time give byte-identical catalogs. The export time is the
current time unless `--exported-at` or the `SOURCE_DATE_EPOCH` environment variable gives it in
seconds since 1970-01-01 UTC.

### Upload limit

`UPLOAD_LIMIT` in `crates/p4kx-export/src/summary.rs` is the largest catalog file the basetool
accepts, 64,000,000 bytes (64 MB), compressed if it is; `DECOMPRESSED_LIMIT` is the cap of the
decompressed content, 1,000,000,000 bytes (1 GB). They must match the basetool's limits: when one
changes, the other changes with it. The owner kept the upload limit at 64 MB after the first
measurement (21.6 MB). The crafting detail (all tiers, quality modifiers, pools and their reward
sources) grew the largest plain catalog to 26.6 MB, 41 % of the upload limit; compressed it is a
small fraction of it. The real-Archive tests write the catalog as `.json.zst`, assert that the
compressed size is within 64 MB and the decompressed size within 1 GB, decode the frame and compare
it with the plain catalog, and print both sizes and the time per zstd level.

Measured on 2026-10-21, with `playability` and `paints`, on every installed Channel (complete catalog of every kind, English and
German texts):

| Channel | Bytes | Manufacturers | Items | Ships | Commodities | Blueprints | Resource groups | Blueprint categories | Properties | Distributions | Pools | Dismantle | Paints |
|---------|------:|--------------:|------:|------:|------------:|-----------:|----------------:|---------------------:|-----------:|--------------:|------:|----------:|-------:|
| LIVE | 26,648,375 | 1,201 | 23,280 | 1,087 | 145 | 1,618 | 19 | 21 | 29 | 21 | 160 | 1 | 1,064 |
| PTU | 26,271,532 | 1,172 | 22,945 | 1,054 | 145 | 1,618 | 19 | 21 | 29 | 21 | 160 | 1 | 1,035 |
| EPTU | 25,840,617 | 1,154 | 22,788 | 1,049 | 145 | 1,607 | 19 | 21 | 29 | 21 | 154 | 1 | 1,017 |
| HOTFIX | 26,648,377 | 1,201 | 23,280 | 1,087 | 145 | 1,618 | 19 | 21 | 29 | 21 | 160 | 1 | 1,064 |
| TECH-PREVIEW | 26,695,043 | 1,203 | 23,635 | 1,035 | 145 | 1,618 | 19 | 21 | 29 | 21 | 160 | 1 | 1,063 |

The ship base fields (#395) added 143,268 bytes on LIVE (0.5 %). Written as `.json.zst` (level 19,
window log 27, `--exported-at 1`), measured on 2026-10-21 with the ship base fields:

| Channel | Uncompressed | Compressed | Ratio |
|---------|-------------:|-----------:|------:|
| LIVE | 26,648,375 | 1,702,931 | 15.6 |
| PTU | 26,271,532 | 1,681,150 | 15.6 |
| EPTU | 25,840,617 | 1,656,576 | 15.6 |
| HOTFIX | 26,648,377 | 1,702,646 | 15.6 |
| TECH-PREVIEW | 26,695,043 | 1,721,630 | 15.5 |

The compressed catalog is 2.7 % of the 64 MB upload limit; the decompressed one is 2.7 % of the
1 GB cap. The level trades time for size (LIVE, window log 27; the plain build takes 2.1 s):

| Level | Compressed | Ratio | Compression time |
|------:|-----------:|------:|-----------------:|
| 15 | 2,005,158 | 13.2 | about 2.6 s |
| 17 | 1,807,476 | 14.7 | about 7.0 s |
| 19 | 1,700,181 | 15.6 | about 12.7 s |

Level 19 stays: 12.7 s is acceptable and it saves 15 % over level 15.

Of the 26.5 MB of LIVE, the paints take 0.5 MB (the ship lists), the blueprints take 5.5 MB (4.4 MB more than before: the `tiers` repeat
the cost tree, every line with its null fields), the pools 0.43 MB (824 entries, 1,039 reward
sources), the quality distributions 29 kB and the properties 8 kB. The kinds written before are
unchanged: items 16.9 MB, manufacturers 1.1 MB; the ships grew from 1.4 MB to 1.5 MB.

A catalog builds in 4 to 7 seconds (LIVE 5.5 s) before the ship base fields. With them LIVE builds
in 3.2 s in a release test run (the `p4kx ships` export alone: 2.9 s). A first version that gave
every split of the parallel scan its own ship reader took 10.5 s, because each reader began with
empty Entity caches; the scan now keeps the readers in a pool, so a reader keeps its caches across
splits. Left out as development leftovers on
LIVE: 2 manufacturers, 1,145 items, 57 ships and 1 quality distribution (1,205 Records).

### Verification against the live data

Checked on all five Channels; `crates/p4kx-export/tests/real_archive/real_archive_verification.rs` repeats it
on `P4KX_DATA_P4K` and `P4KX_DATA_P4K_B`.

- The record-path prefixes and types match the live DataForge: of 29,693 `EntityClassDefinition`
  Records on LIVE, 25,704 lie under the item, ship and commodity prefixes (1,202 of them are left
  out as leftovers); the rest are actors, mineables, job boards, interiors, audio, decorations,
  lights and the like and are no catalog kind. All 1,203 `SCItemManufacturer` and 1,619
  `CraftingBlueprintRecord` Records are selected or left out as leftovers.
- The token list gained `debug`, `deprecated` and `dev`, which mark 20 further items on LIVE (the
  Vlk spewgun test weapons under `fps_weapons/dev`, `*_deprecated` bar counters, a debug hacking
  chip). `placeholder` stays out of the list: the manufacturer `PLACEHOLDER` is referenced by an
  item, and the manufacturers are complete by decision. The rest of the vocabulary
  (`fake`, `old`, `legacy`, `temp`, `prototype`) names real Records and stays in the catalog by
  decision.
- Every `manufacturerGuid` of an item or ship, every `producedItemGuid`, every ingredient
  `resourceGuid` and `itemGuid` resolves to a catalog entry; no GUID occurs twice.
- Every commodity `typeGuid` resolves to a `resourceGroups` entry, every `parentGuid` of a group to
  another one, and every blueprint `categoryGuid` to a `blueprintCategories` entry. On LIVE the
  catalog holds 19 groups (8 of them nested) and 21 categories, 14 of which carry blueprints.
- Texts that carry a key (placeholder keys excluded), LIVE: English 27,751 of 27,889 resolve
  (99.5 %), German 15,819 (56.7 %). The German file lacks a text for about 11,250 of 23,180 item
  texts, so German `null` is common for items and the Basetool falls back to `en`; per kind:

  | Kind | Keyed texts | Missing in en | Missing in de |
  |------|------------:|--------------:|--------------:|
  | manufacturers | 2,248 | 8 | 116 |
  | items | 23,179 | 118 | 11,252 |
  | ships | 2,172 | 12 | 677 |
  | commodities | 289 | 0 | 24 |
  | blueprints | 1 | 0 | 1 |

  The `p4kx catalog` summary counts them over all texts: English 38,004 resolved, 160 unresolved,
  German 25,510 resolved, 12,654 unresolved, 23,665 placeholders in each.
- Crafting, LIVE (the same on every Channel but for the counts): every blueprint has exactly one
  tier, equal to its `craftTimeSeconds`; 3 blueprints let the player choose fewer than offered
  (`BP_CRAFT_thp_light_legs_01_01_01`, `_03` and `_12`), 3 carry the factor 1.15 (the slaver arms
  `_01`, `_13` and `_15`), 1,555 carry quality modifiers of both range kinds; all modifier
  `propertyGuid` resolve, all 824 pool entries point at a catalog blueprint, 49 of the 160 pools
  have no reward source; 8 blueprints are available by default, 1,617 link to the one dismantle
  process (the exception, `BP_CRAFT_COOL_S04_CNOU_Pioneer`, has no produced item).

`p4kx catalog` warns on stderr when a catalog reaches 90 % of the limit and when it exceeds it;
the file is written either way.
