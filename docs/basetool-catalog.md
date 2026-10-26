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
- **Version 2 (ADR-0161)** is breaking against version 1 in one respect: a paint also needs its own
  `RequiredTags` offered by the port or the ship (see "Paints"), so `paints[].shipGuids` lists
  fewer ships than version 1 did (40 fewer on LIVE 4.7). No field changed its name or type. The
  Basetool import must accept version 2.
- **A production import takes catalogs of the Channel `LIVE` only.** `PTU`, `EPTU`, `TECH-PREVIEW`
  and `HOTFIX` are confidential test Channels (ADR-0038).

### Game texts

Identifiers, short names and fact values may reach third parties; descriptions and lore may not.
The game texts of the catalog are every `description` and every `displayType`, the `name` of a
starmap object (a place name that comes from localisation, by the owner's decision for #321), and
the `title`, `missionType`, `contractor`, the `displayName` of a contract's faction and the `place`
of a pickup place of a contract, each with
its `key`, `en` and `de`. `name` of the other kinds, `role`, `career` and the other fields are
identifiers, short names or fact values. The catalog keeps the `description` fields for the
Basetool's logged-in members; the Basetool keeps them, `displayType` and the contract texts away
from third parties. Every such field carries
`"x-gameText": true` in the JSON Schema, and no other field does; a test checks this.

```powershell
p4kx catalog <archive> --out basetool-catalog.json [--filter <filter>] [--exported-at <seconds>]
p4kx catalog <archive> --out basetool-catalog.json.zst
p4kx catalog schema
```

### Compressed form

An output name ending in `.zst` (any case) writes the catalog as one zstd frame; a name ending in
`.gz` is refused with exit code 2, because gzip is not written; any other name writes plain JSON,
which stays the default. The zstd encoder is libzstd (ADR-0149). The JSON is serialised straight into the encoder and
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
| `schemaVersion` | The version of the shape, now `2`; an import rejects a version it does not know |
| `source.channel` | The Channel: the folder holding the Archive, e.g. `LIVE` |
| `source.build` | `branch`, `version`, `changeNumber` and `buildId` from the build manifest, or `null` |
| `source.build.publicPatch` | The public patch version, e.g. `4.7.2`, from the RSI Launcher log line `SC <CHANNEL> <version>-<tag>.<change>` (`<version>`: two or three dot-separated numbers, kept as logged) whose change equals `changeNumber`; `null` when the log has no such line (never guessed from a neighbouring one) |
| `source.archiveSize` | The size of the Archive file in bytes |
| `source.archiveModified` | The modification time of the Archive file, UTC in RFC 3339 form |
| `source.toolVersion` | The version of P4KX that wrote the catalog |
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
| `loadouts` | The ports of every ship and of every item that has ports, with the Entity the Default Loadout attaches to each, ordered by `entityGuid` |
| `starmapObjects` | The starmap objects (star systems, planets, moons, stations, jump points) with type, parent, name and world position, ordered by GUID |
| `contracts` | The contracts and sub contracts with resolved title, mission type, pickup places and the reputation granted on success, ordered by `guid` and then `subContractGuid` |
| `mineableElements` | The mineable elements with their instability and other plain numbers, each linked to a `ResourceType`, ordered by GUID |
| `refiningProcesses` | The refining processes (speed and quality pairs of a refinery), ordered by GUID |
| `shops` | The shops of the shop registry with kind, brand, location and the link to a starmap object, ordered by `key` |
| `shipStoreSlugs` | The RSI store slugs that map to a ship of `ships`, with the retired ones flagged, ordered by `slug` (curated data, ADR-0117) |
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
  counter of the build farm, so a later Build can carry a lower fourth part than an earlier one.
  Use it for display, not for order.

Evidence for the rule is in the pull request that closed p4kx-org/p4kx#402: launcher log lines
(`SC <CHANNEL> <version>-<tag>.<change>`, the same lines as for `source.build.publicPatch`) with no
inversion in any Channel, and the installed manifests. The logs rotate, so the claim covers the
Builds the log kept; a rollback of a Channel to an older change would break it and would show as a
lower number than the last applied one.

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
| `variantOf` | The `guid` of the base of the item's variant group, `null` for a base and an ungrouped item; see "Variant groups" below |
| `stats` | The typed stat blocks of a weapon, shield, quantum drive, power plant, cooler, missile, suit armour, clothing, radar, tractor beam or attachment modifier, `null` for any other item; see "Item stats" below |

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
of the eleven blocks; every block is `null` when the item does not have it, and a field inside a
present block is `null` when the Archive does not state it, never 0. None of the fields is a game
text. The additions are additive in version 1 (ADR-0054); `kind` of a fire mode and
`trackingSignalType` are closed sets that the schema keeps as strings. The blocks `armor`,
`clothing`, `radar`, `tractorBeam` and `weaponModifier` and the fire-mode field `damagePerSecond`
follow ADR-0072.

| Block | Present when | Fields and sources |
|-------|--------------|--------------------|
| `weapon` | the item has `Components[SCItemWeaponComponentParams]` (ship weapons and FPS weapons) | `fireActions`: per element of `fireActions`, in stored order, with a sequence (`sequenceEntries`) flattened to its entries: `kind` (the struct, e.g. `SWeaponActionFireRapidParams`), `name`, `fireRate` (rounds per minute), `pelletCount` (as `pellets`; `pellets` where a Record states that instead) and `ammoCost` of `launchParams`, `heatPerShot`, and for a charged mode `chargeTime`, `overchargeTime`, `cooldownTime`, and `damagePerSecond` (the damage types of the beam's `damagePerSecond`, the power of a mining laser; `null` for a mode without one), and `spread` (see below); a parallel mode (`weaponActions`) is listed as its entries; a charged or wrapped mode reads `fireRate`, `launchParams` and `name` from its `weaponAction` when it has none of its own. `ammoCapacity`: `maxAmmoCount` of the ammo container, the magazine's capacity (see below). `ammo`: the `AmmoParams` Record that the container's `ammoParamsRecord` links to, `null` without a readable one: `speed`, `lifetime`, `damage` (the damage types of `projectileParams.damage`), `detonationDamage`, `detonationMinRadius` and `detonationMaxRadius` (from `explosionParams`) |
| `shield` | `Components[SCItemShieldGeneratorParams]` | `maxHealth` (`MaxShieldHealth`), `maxRegen` (`MaxShieldRegen`), `decayRatio` (`DecayRatio`), `downedRegenDelay` (`DownedRegenDelay`), `damagedRegenDelay` (`DamagedRegenDelay`) |
| `quantumDrive` | `Components[SCItemQuantumDriveParams]` | `fuelRequirement` (`quantumFuelRequirement`, as stored), `jumpRange`, `disconnectRange`, and of `params`: `driveSpeed`, `cooldownTime`, `spoolUpTime`, `stageOneAccelRate`, `stageTwoAccelRate`, `engageSpeed` |
| `powerPlant` | `AttachDef.Type` is `PowerPlant`, ignoring case | `powerOutput`: the `amount` of the first `generation` flow of `resourceStates` whose resource is `Power`, and `unit`: the `unit` of that flow |
| `cooler` | `AttachDef.Type` is `Cooler`, ignoring case | `coolingRate` and `unit`: as for the power plant, for the resource `Coolant` (a cooler's flow is an `ItemResourceDeltaConversion`) |
| `missile` | `Components[SCItemMissileParams]` | `damage` (the damage types of `explosionParams.damage`), `explosionMinRadius`, `explosionMaxRadius`, `trackingSignalType`, `lockTime`, `lockingAngle`, `lockRangeMin`, `lockRangeMax` (of `targetingParams`) and `linearSpeed` (of `GCSParams`) |
| `armor` | `Components[SCItemSuitArmorParams]` (armour pieces, 2,388 items on LIVE 4.10.2 out of 2,391 Records; the rest are development leftovers) | `integrityMilestoneToBreak`; `damageResistance`: the `damageResistance` struct of the `DamageResistanceMacro` Record that `damageResistance` links to, `null` without a readable one (a Record that is missing, has another struct or cannot be built): `ignoreMeleeDamage` (`IgnoreMeleeDamage`) and per damage type `physical`, `energy`, `distortion`, `thermal`, `biochemical`, `stun` (`PhysicalResistance` and so on) the entry `multiplier`, `threshold`, `damageCap`, `damageCapLimit` (`Multiplier`, `Threshold`, `DamageCap`, `DamageCapLimit`) |
| `clothing` | `Components[SCItemClothingParams]` (4,423 items on LIVE 4.10.2 out of 4,426 Records; armour carries this component too) | `temperatureResistanceMin` and `temperatureResistanceMax` (`TemperatureResistance.MinResistance` and `.MaxResistance`), `radiationCapacity` (`RadiationResistance.MaximumRadiationCapacity`), `radiationDissipationRate` (`RadiationResistance.RadiationDissipationRate`), `gForceResistance` (`Flight.gForceResistance`) |
| `radar` | `Components[SCItemRadarComponentParams]` | `aimAssistDistanceMin`, `aimAssistDistanceMax`, `aimAssistOutsideRangeBuffer` (`aimAssist.distanceMinAssignment`, `.distanceMaxAssignment`, `.outsideRangeBufferDistance`), `pingCooldownTime` (`pingProperties.cooldownTime`) |
| `tractorBeam` | the weapon component holds an `SWeaponActionFireTractorBeamParams` (ship tractor beams and salvage heads) | of the first such struct below `Components[SCItemWeaponComponentParams]`: `minForce`, `maxForce`, `minDistance`, `maxDistance`, `fullStrengthDistance`, `maxVolume`, `maxAngle` |
| `weaponModifier` | `Components[EntityComponentAttachableModifierParams]` holds an `SSalvageModifier` or an `STractorModifier` | `salvage`: `radiusMultiplier`, `salvageSpeedMultiplier`, `extractionEfficiency` of the first `SSalvageModifier`; `tractor`: `forceMultiplier`, `fullStrengthDistMultiplier`, `maxDistMultiplier`, `maxVolumeMultiplier` of the first `STractorModifier`; each `null` without that struct. Salvage modifiers and tractor attachments carry them in `modifiers[].weaponModifier.weaponStats` |

##### From a gameplay property to a stat field

The Archive holds no link from a `CraftingGameplayPropertyDef` to the item field it changes: the
Record has a name (`propertyName`), a unit format and a display transformation, nothing that names a
field (ADR-0072). The catalog therefore states no link per entry of `gameplayProperties`; the
consumer joins a modifier's `propertyGuid` to the `className` of that entry and takes the stat
field from this table. The table covers the 29 properties of LIVE 4.10.2; `real_archive_gear_stats`
checks that every `className` of `gameplayProperties` has a row. A multiplier-kind modifier scales
the base value of the field, an additive one adds to it.

| `className` | `stats` field the modifier changes | Items |
|-------------|------------------------------------|-------|
| `GPP_Armor_DamageMitigation` | `armor.damageResistance.<type>.multiplier` for all six types; the SC Wiki's physical value is `physical.multiplier` | armour |
| `GPP_Armor_TemperatureMin` | `clothing.temperatureResistanceMin` | armour, clothing |
| `GPP_Armor_TemperatureMax` | `clothing.temperatureResistanceMax` | armour, clothing |
| `GPP_Armor_RadiationCapacity` | `clothing.radiationCapacity` | armour, clothing |
| `GPP_Armor_RadiationDissipation` | `clothing.radiationDissipationRate` | armour, clothing |
| `GPP_Health_MaxHealth` | `health` | ship components, weapons |
| `GPP_Weapon_Damage` | `weapon.ammo.damage` (guns), `weapon.fireActions[].damagePerSecond` (mining lasers) | weapons |
| `GPP_Weapon_FireRate` | `weapon.fireActions[].fireRate` | personal weapons |
| `GPP_Weapon_Recoil_Kick` | none: see below | personal weapons |
| `GPP_Weapon_Recoil_Handling` | none: see below | personal weapons |
| `GPP_Weapon_Recoil_Smoothness` | none: see below | personal weapons |
| `GPP_Weapon_ReloadSpeed` | none: see below | personal weapons |
| `GPP_Weapon_Spread` | `weapon.fireActions[].spread` | personal weapons |
| `GPP_ItemResource_PowerGeneration` | `powerPlant.powerOutput` | power plants |
| `GPP_ItemResource_CoolantGeneration` | `cooler.coolingRate` | coolers |
| `GPP_Shield_MaxHealth` | `shield.maxHealth` | shields |
| `GPP_Radar_MinAimAssistDistance` | `radar.aimAssistDistanceMin` | radars |
| `GPP_Radar_MaxAimAssistDistance` | `radar.aimAssistDistanceMax` | radars |
| `GPP_Quantum_Speed` | `quantumDrive.driveSpeed` | quantum drives |
| `GPP_Quantum_FuelRequirement` | `quantumDrive.fuelRequirement` | quantum drives |
| `GPP_Weapon_Tractor_Force` | `tractorBeam.maxForce` | tractor beams, salvage heads |
| `GPP_Weapon_Tractor_MaxDist` | `tractorBeam.maxDistance` | tractor beams, salvage heads |
| `GPP_Weapon_Tractor_FullStrengthDist` | `tractorBeam.fullStrengthDistance` | tractor beams, salvage heads |
| `GPP_Weapon_Tractor_MaxVolume` | `tractorBeam.maxVolume` | tractor beams, salvage heads |
| `GPP_Weapon_HullScraping_Radius` | `weaponModifier.salvage.radiusMultiplier` | salvage modifiers |
| `GPP_Weapon_HullScraping_Speed` | `weaponModifier.salvage.salvageSpeedMultiplier` | salvage modifiers |
| `GPP_Weapon_HullScraping_Efficiency` | `weaponModifier.salvage.extractionEfficiency` | salvage modifiers |
| `GPP_Crafter_CraftSpeed` | none: a property of a crafting station, not of an item | crafting stations |
| `GPP_Crafter_DismantleEfficiency` | none: a property of a crafting station, not of an item | crafting stations |

Properties with none: the three recoil properties and the reload speed have no single stat field.
A personal weapon's recoil is the weapon's `actorProceduralRecoilConfig` and the `recoil` of its
fire modes, Records with curves per stance and aim state, and its reload times are in further
Records; reducing them to one number is a model of the game, which ADR-0065 leaves to the
consumer. The SC Wiki has no field for them either. The two crafter properties modify the station
that crafts, which is not an item with `stats`. The spread has a field, `spread` of a fire mode
(`launchParams.spreadParams`, or `spreadParams` of a beam): `min`, `max`, `firstAttack`, `attack`
and `decay` as stored. No LIVE blueprint modifies the spread or the reload speed.

`GPP_Armor_DamageMitigation` is one factor; the Archive does not say which damage types it
scales, so the catalog gives the `multiplier` of all six (`physical`, `energy`, `distortion`,
`thermal`, `biochemical`, `stun`) and the SC Wiki's `physical` is `physical.multiplier`.

Items without the source component: some produced items carry no field for a property their
blueprint modifies, and the catalog states `null` for the block rather than a default. On LIVE
4.10.2 these are the backpacks, which have no `SCItemSuitArmorParams`, `SCItemClothingParams` or
health (16 items for the damage mitigation, 7 for the temperatures, 1 for the radiation
dissipation), `hdtc_shoes_01_01_01`, a piece of clothing with `clothing` but no `armor`, for the
damage mitigation, the 8 fuel nozzles for `GPP_Health_MaxHealth` (no health component) and the
personal weapons that only fire a beam (`none_smg_energy_01`) for `GPP_Weapon_FireRate` (no
`fireRate`). The clothing pieces of the issue's 1,346 mitigation targets are these items: where
`armor` is `null` the Archive holds no damage resistance for the piece, and a consumer treats the
base value as unstated. `real_archive_gear_trace` checks every modifier of every blueprint
against the produced item and prints these lists.

`resourceStates` lists the states of `Components[ItemResourceComponentParams].states` in stored
order: `name`, `emSignature` and `irSignature` (`signatureParams`: `nominal` is `nominalSignature`,
`decayRate`), and `flows`. A flow is one `generation` or `consumption` of a delta of the state:
`kind` (the struct of the delta, e.g. `ItemResourceDeltaGeneration`, `ItemResourceDeltaConversion`,
`ItemResourceDeltaConsumption`, `ItemResourceDeltaStorage`), `direction`, `resource`, `amount` (the
first number of `resourceAmountPerSecond`) and `unit` (the struct holding it:
`SPowerSegmentResourceUnit`, `SStandardResourceUnit`, `SMicroResourceUnit`, `SCentiResourceUnit`).
Units differ between structs and are not converted. A delta without a `generation` or
`consumption` (a network reflection) gives no flow.

Raw values that need care:

- `quantumDrive.jumpRange` is `f32::MAX` (3.4028235e38) on a drive the Archive leaves unbounded (a
  development drive of the Idris). It is exported as stored; a reader treats a value at that
  magnitude as unlimited.
- A number that is not finite (NaN, infinity) is `null`, never written.
- `weapon.ammoCapacity` is the capacity of the ammo container, which is the magazine and not the
  weapon body: a ship weapon holds `Components[SAmmoContainerComponentParams]` itself; a weapon
  without one (the personal weapons, ballistic and energy alike) links its magazine entity by
  `Components[SCItemWeaponComponentParams].ammoContainerRecord`, and the container of that entity,
  one hop, is read instead. `weapon.ammo` is read from the `AmmoParams` Record of that container's
  `ammoParamsRecord`. A Reference to a missing Record, to a Record of another struct, or to a
  Record that cannot be built leaves `ammo` `null`; the item stays in the catalog.
- The fire-mode fields `name`, `fireRate`, `heatPerShot`, `chargeTime`, `overchargeTime`,
  `cooldownTime` and `launchParams` are read from the fire mode and, where it has none of its own,
  from its `weaponAction`.

A damage object has `physical`, `energy`, `distortion`, `thermal`, `biochemical` and `stun`
(`DamagePhysical` and so on); a type the struct does not hold is `null`. An item with at least one
of the six blocks also carries `health` (`SHealthComponentParams.Health`) and `resourceStates`;
the signature and power draw inputs are in `resourceStates`, not in separate blocks. Items without a
block carry neither.

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
| `variantOf` | The `guid` of the base of the ship's variant group, `null` for a base and an ungrouped ship; see "Variant groups" below |
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

The development-leftover token rule (see "Selection") runs first: a Record it leaves out is no
entity and has no playability. No structural marker sets the leftover-like ships apart from the
ships a player owns, so a ship is also left out by name when its record path or name holds the
whole word `temp`, `display` or `hijacked` (21 ships on LIVE, 15 of them `player`; ADR-0167).

**The Basetool seeds only `player` ships and items, and treats them as candidates.** `player`
means the Records carry the structural markers of a player entity (ships: hangar-ready inclusion
mode and no AI crew or hull; items: a shop display name). It is not proof that a player can obtain
it: items carry no AI marker. The Basetool
matches every other value and `null` against what it knows and seeds none of them. A `null` ship
(LIVE: Stingray, Skylark, Railen) is added by hand.

`p4kx catalog` prints the count per value and the unclassified count for items and ships; `--json`
lists them under `playability`. LIVE, 2026-10-21:

| Kind | `player` | `ai` | `mission` | `special` | `null` |
|------|---------:|-----:|----------:|----------:|-------:|
| ships | 324 | 690 | 25 | 35 | 13 |
| items | 7,954 | 0 | 522 | 8,570 | 6,234 |

A commodity is an `EntityClassDefinition` under `entities/commodities` or a `ResourceType` Record that
a cost of any tier uses and no commodity Record carries (see below). A commodity Record takes its fields
from the entity:

| Field | Source |
|-------|--------|
| `name` | `SCItemPurchasableParams.displayName` |
| `description` | `EntityUIDisplayParams.displayDescription` in `StaticEntityClassData` |
| `typeGuid` | The GUID of the `ResourceTypeGroup` Record that directly lists the `ResourceType` in its `resources` (19 groups on LIVE: Metal, Mineral, Gas, …); `null` without a `ResourceType` or a group |
| `subtypeGuid` | The GUID of the `ResourceType` Record that the first entry of `ResourceContainer.defaultComposition` links to; `null` without one, e.g. `Special_HolidayLetter` |
| `displayType` | A text: `SCItemPurchasableParams.displayType`, e.g. `@items_commodities_type_metal` |
| `densityGramsPerCubicCentimetre` | The number `densityType.densityUnit.gramsPerCubicCentimeter` of the `ResourceType` holds (a fact, no game text); `null` without a `ResourceType`, a density or a density in another unit |
| `refinedSubtypeGuid` | The GUID `refinedVersion` of the `ResourceType` links to; `null` without a `ResourceType` or a `refinedVersion`. It equals the `subtypeGuid` of the refined commodity only when the catalog has that commodity |

The refining fields are described in [commodity-refining.md](commodity-refining.md), with what the
DataForge does not hold (instability per commodity, yields, counting in SCU or units). The refined
`ResourceType` may have no commodity of its own; then `refinedSubtypeGuid` matches no `subtypeGuid`.

A name or description that is missing or a placeholder key is taken from the `ResourceType` Record
`subtypeGuid` links to (`displayName`, `description`) instead. The game holds no field named type
or subtype for a commodity; the groups of `ResourceType` Records are the types, the `ResourceType`
is the subtype.

A `ResourceType` Record that the costs of a blueprint use in any tier, selections opened, and that
no commodity Record carries is listed as a commodity as well, so that every resource of a cost
points at a catalog entry (LIVE has one tier per blueprint: Aslarite, Ouratite, Iron, Stileron, Lindinium, Riccite, Tin, Torite, Savrilium and
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
| `gameplayProperties` | `name`: the localisation key `propertyName` and its texts; a placeholder key stays as `key` and leaves `en` and `de` `null`, as for `items` and `blueprints`. The target of a modifier's `propertyGuid` |
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

#### Variant groups

Many ships and items exist in cosmetic variants with the same function. `variantOf` is the `guid`
of the entry of the same kind that is the base of the entity's group, and `null` for the base
itself and for an entity that is alone in its group. Every `variantOf` resolves to an entry of the
same kind. The grouping is a derived fact (ADR-0066): it reads Record data and the vehicle
implementation, never a name, and it ignores `playability`, so an AI variant that is structurally a
player ship is grouped with it.

**Ships** share a group when all of these are equal:

- `vehicleDefinition` (`VehicleComponentParams.vehicleDefinition`), compared as a path without
  regard to case, to `/` or `\`, and to a leading separator;
- the vehicle implementation after the Modification the ship selects (ADR-0050), compared as a
  whole: the digest covers every element of the patched `Vehicle` tree (parts, item ports and
  their children such as `Pitch` or `Yaw`, the other children of `Vehicle` such as
  `MovementParams`) with all its attributes, sorted by name, and the children in file order. The
  `Modifications` element, the path of the file, the names of the Modification and its patch file,
  the `name` of the root and the attributes `material`, `materials`, `filename` and `geometry`
  anywhere are not compared. Because the patch file and the `Elem` overrides are applied before
  the comparison, whatever they change is part of it. The item ports that take paints are part of
  it too, so a Modification that changes only the paints a ship accepts starts a group of its own.

The ship Record has no armour component on LIVE (no component struct name contains `armor` or
`armour`; build 4.10), so no armour term enters the rule. The default loadout and the names do not
count: editions that differ only in their loadout, or in a material, are variants. On LIVE `AEGS_Idris_P` therefore groups under
`AEGS_Idris_M` and `AEGS_Sabre_Comet` under `AEGS_Sabre`, because they differ only in their
default loadout. A ship without a `vehicleDefinition`, or whose vehicle
implementation cannot be read in full (a missing file or patch file, a broken document), is
ungrouped, because equality cannot be shown.

**Items** share a group when all of these are equal:

- `type`, `subType`, `size`, `grade` and `manufacturerGuid` of the `AttachDef`;
- the stat blocks (`stats`, ADR-0065), including `resourceStates`;
- the components: every component of the Record is compared with all its fields, including
  `SAttachableComponentParams` (without its `Tags`), the physics, the default loadout of an item
  that has ports (`SEntityComponentDefaultLoadoutParams`, unlike ships) and the geometry, whose
  model (the geometry file of every node) must be equal: a different model is not cosmetic.
  Left out are exactly these components: `SEntityInteractableParams`,
  `SInteractionStateMachineParams`, `SEntityComponentCarryableParams` and `SActorUsableParams`
  (per-item Records of equal content), and these fields: `Tags` of `SAttachableComponentParams`,
  `Material`, `Materials` and `Palette` of `SGeometryResourceParams`, and `openInventoryInteraction`
  and `openLootingInteraction` of `SCItemInventoryContainerComponentParams`. Localisation keys
  (names, descriptions, display names) are never compared. On LIVE the clothing and armour
  colour variants differ in exactly these: name, tags, material path, palette and the per-item
  interaction Records. A Reference counts by the GUID of its target and, for a weak pointer, by
  the location of the instance inside that Record.

An item with no compared component besides `SAttachableComponentParams`, without an `AttachDef`
type, or whose data holds a value the reader could not read, is ungrouped. Names, descriptions,
tags and the display type may therefore differ inside a group.

`variantOf` means "structurally identical", not only "cosmetic". Per-ship functional duplicates
that differ in their name alone are grouped too (e.g. the `Controller_Shield_<ship>` and
`Engineering_Buff_Modifier_<ship>` items), and so can fitment such as ATC managers, docking tubes,
intakes or quantum tanks, depending on whether the Records of the ships differ in a compared field
(LIVE, build 4.10: groups of 117 ATC managers, 75 docking tubes, 54 intakes and 38 quantum tanks
were measured under an earlier version of the rule; the counts shift with the rule and the
build). Fitment differences may or may not split a group. A consumer that wants only what a player
sees or fits combines `variantOf` with `playability`.

**Base.** The member with `playability` `player`, then the member with the smallest `className`
(compared byte-wise), then the smallest `guid`. A group holds at least two members with different
`guid`s, so a repeated `guid` in a malformed Archive makes no group; `p4kx catalog` prints `variants` and `groups` per kind and `--json` lists them.

Size: one `variantOf` per ship and item, 4 bytes (`null`) or 38 bytes (a GUID) uncompressed; the
GUIDs repeat and compress well.

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
  `VehicleComponentParams.modification` applied. The implementation is read by
  `p4kx_assembly::VehicleImplementation` after the rules of ADR-0050 (patch file, `Elem` entries
  by id, fallback file names); the tags are the `requiredTags` the patched port has (the Omega
  variant of the Mustang requires `Paint_Omega` instead of `Paint_Mustang`).

The paint's own `AttachDef.RequiredTags` must be offered as well (ADR-0161): every tag in them is
one of the `portTags` or `requiredTags` of the same port, or a tag of the ship Record's
`AttachDef.Tags`, or has one of those below it in the tag tree. A paint without `RequiredTags`
passes. The Omega paint requires `Paint_Omega`, which the patched port of the Omega requires while
it still lists `Paint_Mustang` as its `portTags`. Measured on LIVE (4.7), the check removes 40
assignments of four paints and no paint loses its last ship: the Hornet F7A Mk II desert paint
(requires the `ANVL_Hornet_F7A_Mk2` ship tag) no longer fits ten F7C variants, the C8X Pisces
Expedition paint no longer fits the eight C8 and C8R variants, and the two Cutlass Black paints
that require `Paint_Cutlass_Black` no longer fit the eleven Red, Blue and Steel variants. A ship
whose `modification` is empty keeps the base port, so the Idris-M still fits the paints of the
Idris-P; the check does not change that, as the Idris paints require no tag of their own. The
game selects the Military paints elsewhere.

`p4kx catalog` counts the paints, those that fit a ship and those that fit none, the ships that
have at least one paint and the ships whose vehicle implementation cannot be read, or whose patch file or Modification is
not applied as it names, except that a patch file the Archive lacks is a fact of the game data; `CatalogSummary::paints()` returns them, and an unreadable vehicle file is also a warning of
`CatalogSummary::warnings()`. Measured on LIVE (4.7): 1,064 paints, 1,039 assigned, 25 not
assigned, 1,046 ships with paints. Of the 25, 16 are paints of the ARGO ATLS, an actor rather than
a ship, 2 are weapon skins and 7 are the default livery of a ship (`Paint_Ballista_Base`,
`Paint_Gladius_Default`, `Paint_Carrack_Default`, `Paint_Hurricane_Default`, the Vanguard
Harbinger, Sentinel and Hoplite) whose tags no port requires. With a `--filter` the paints and the ships are
both filtered, so a paint lists only ships the filter selected; both kinds are partial.

The development leftovers among the paints (68 on LIVE) are counted as skipped in `paints` and in
`items`, whose rule they follow; `p4kx catalog` shows the number in both lines, and the total of
skipped Records adds up the kinds that select Records only, so it counts them once. The catalog build
takes about 18 s on LIVE; deriving the paints takes well under 0.1 s of it, the ships' vehicle
files being read once by the record scan for the ship base data and shared through one path
index (`ArchiveFiles`). A cancelled build stops the derivation between two paints.

### Loadouts

A loadout (`loadouts`) is the Default Loadout of one ship or of one item that has ports, resolved
into a tree (ADR-0064). The kind refers to the entity and adds no name, no description and no
media: name and record stay on the entry in `ships` or `items`, so no GUID occurs twice among the
`guid` fields of the catalog. Loadouts are ordered by `entityGuid`.

| Field | Source |
|-------|--------|
| `entityGuid` | The `guid` of the entry in `ships` or `items` the ports belong to |
| `entityKind` | `ship` or `item`; a closed set the game may extend, so the schema keeps a string |
| `issues` | The codes of what kept part of the Loadout from being read, sorted, without duplicates: `dangling_reference`, `invalid_reference`, `unknown_class`, `ambiguous_class`, `not_an_entity`, `unreadable_record`, `cycle`, `too_deep`, `too_many_ports`, `no_loadout_files`, `xml_loadout`, `vehicle_implementation`; empty when all was read. `no_loadout_files` (an XML Loadout is named but no Archive was given to read it from) cannot occur in a catalog, which always has its Archive. A vehicle implementation or a patch file the Archive lacks is a `vehicle_implementation` issue too, a fact of the game data |
| `ports` | The ports of the entity, see below |

**Selection.** Every entry of `ships` has a loadout, also one without ports, with one exception: a
ship whose own Record cannot be resolved stays in `ships` without a loadout and is listed in
the failures (printed on stderr by `p4kx catalog`, under `failed` in `--json` and in the summary
of the window) with a reason that starts with `loadout: `; `completeness.loadouts` is not changed by it.
An entry of `items`
has one when its Record defines item ports (`SItemPortContainerComponentParams`) or names a default
Loadout (`SEntityComponentDefaultLoadoutParams`) and its tree holds at least one port: turrets,
racks, weapons with attachment ports. With a `--filter` the ships and items are filtered first,
so a loadout exists only for the entries the filter selected and the kind is partial.

**Ports.** Each port has:

| Field | Source |
|-------|--------|
| `name` | The port name as the Loadout or the port definition spells it; port names match ignoring ASCII case |
| `source` | Where the attachment comes from: `loadout` (the entity's own Default Loadout), `override` (the Loadout of the entry that attached the entity, which replaces the entity's own per port), `port_default` (the default item of the port definition), `unset` (defined, holds nothing) or `vehicle` (known only from the vehicle implementation, holds nothing). Closed set, string |
| `minSize`, `maxSize` | `MinSize` and `MaxSize` of the `SItemPortDef`, or `minSize` and `maxSize` of the `ItemPort`; `null` when the data gives none |
| `types` | `{type, subTypes}` per entry of `Types` of the `SItemPortDef`; for an `ItemPort` the `type` of each `Types/Type` and no subtypes |
| `flags` | The words of `Flags` as stored, without a leading `$` (`invisible`, `uneditable`) |
| `portTags`, `requiredTags` | The words of `PortTags` and `RequiredPortTags` of the `SItemPortDef`, or of `portTags` and `requiredTags` of the `ItemPort`: split at white space and otherwise as stored, like `tags` of an item (a leading `$` stays). An item fits a port when it carries the required tags (see "Paints" for the tag tree rule) |
| `displayNameKey` | `DisplayName` of the `SItemPortDef`, a localisation key; `null` for none and for a placeholder key. The key is exported, not its text |
| `visibility` | `visible`, `invisible` or `unspecified`: whether the vehicle implementation's part of that name carries the flag `invisible`; `unspecified` without such a part. Closed set, string |
| `itemGuid` | The `guid` of the entry in `items` (a docked vehicle: in `ships`) that the port holds by default; `null` for an empty port. For an attachment that cannot be resolved the GUID the Loadout names, or `null` when it names a class name only |
| `itemClassName` | The class name of the attached entity, or the class name the Loadout names; `null` for an empty port and an unresolved Reference |
| `dockedVehicle` | `true` when the attached entity is a docked vehicle (`AttachDef.Type` starts with `NOITEM_Vehicle`, e.g. a snub craft in a hangar port). It is a reference to a ship and is not expanded: `ports` stays empty (#305) |
| `unresolved` | The code of the error that kept the attachment from being resolved (`issues` lists the codes), otherwise `null` |
| `ports` | The ports of the attached entity under the Loadout that attached it, or the ports of the nested Loadout of an empty port; empty otherwise |

The ports of an entity are, in this order: those a Loadout names in Loadout order, the other
ports the Record defines in definition order, and the `ItemPort` elements of the vehicle
implementation (the Modification applied, ADR-0050) that no port above names, in file order, with
the source `vehicle`. The definition of a port is the Record's `SItemPortDef`; a port the Record
does not define takes it from the `ItemPort` of that name. The visibility always comes from the
vehicle implementation (#289). A ship's Loadout overrides the default Loadout of the turrets and
racks it carries per port, so the nested ports of a ship are the ones that apply to that ship;
the entry of the turret itself shows the turret's own defaults. The walk is the one of
`EntityNode::loadout` (#316).

**References.** Every `entityGuid` is a `guid` of `ships` or `items`. An `itemGuid` is one of
them too, except for an Entity outside both kinds: a development leftover, or an Entity of another
folder (`p4kx catalog` counts them as the Entities without an entry in items or ships,
`CatalogSummary::loadouts().outside`; the key `outside` of `--json`). Such a GUID stays, so the
Basetool can tell an unknown Entity from an empty port. An `itemGuid` of an unresolved attachment
may name no Record at all. With a `--filter` the references need not resolve.

`p4kx catalog` counts the loadouts of ships and items, the ports (the nested ports of a ship are
counted for every ship that carries them), the ports that hold an Entity, the docked vehicles, the
unresolved attachments, the Entities outside the kinds and the loadouts with an issue; `--json`
lists them under `loadouts`.

#### Matching tags

`tags` of an item and `portTags` and `requiredTags` of a port are free strings that the game
matches by name; they are not names of tags in the tag tree, so the catalog carries no tag GUIDs
for them. Checked on LIVE: of 70,629 port tag words 65,133 name no tag of the tree or several
(18,891 tags, 1,745 names used more than once), and of 28,575 item tag words 21,146 do not
resolve; the few unique hits are coincidental. An item fits a port when it carries every word of
the port's `requiredTags`, compared as stored (case-sensitive, a leading `$` is part of the word).
The tag tree rule of the paints (a tag below a required tag also counts) applies only to the
`Tag` Records that paints and Tags fields refer to (see "Paints").

### Contracts

`contracts` lists every contract of the `ContractGenerator` Records and every sub contract of a
contract, one entry each (issue #416). A contract with sub contracts has its own entry and one
more per sub contract; the entries of a contract share its `guid`, a sub contract's entry names its
`id` in `subContractGuid`. Entries are ordered by `guid`, then `subContractGuid` (`null` first),
then `recordGuid`; of two entries with the same `guid` and `subContractGuid` the first is kept.

A contract is an element of the arrays `contracts`, `introContracts`, `legacyContracts` and
`PVPBountyContract` of a handler in `generators`. One without an `id` (or with a nil one), and one
whose debug name or Record is a development leftover, is left out and counted in `skipped`; so is a
sub contract without an `id` (it cannot be told apart) or with a leftover debug name, and an entry
that the rule on equal `guid` and `subContractGuid` above drops.

| Field | Meaning |
|-------|---------|
| `guid`, `subContractGuid` | The `id` of the contract and of the sub contract |
| `recordGuid`, `recordPath` | The `ContractGenerator` Record |
| `debugName`, `handler` | The `debugName` of the contract and of its handler |
| `templateGuid` | The `ContractTemplate` Record the contract names |
| `notForRelease`, `workInProgress` | The flags as stored, `null` when absent; the catalog does not filter on them |
| `title` | The resolved title (game text) |
| `missionTypeGuid`, `missionType` | The `MissionType` Record and its localised name (game text) |
| `contractor` | The `Contractor` string parameter (game text) |
| `pickupPlaces` | Up to five starmap objects the contract is offered at, each `starmapObjectGuid`, `objectType`, `place` (game text) |
| `faction` | The `factionReputation` of the handler as stored: `guid`, Record `name` and `displayName` (game text), `null` when the handler names none |
| `reputationGains` | The reputation granted on success; an amount may be negative |

**Layers.** Sub contract, contract and handler each override the template: the first of them that
states a value wins, the template last. A level states its overrides in `paramOverrides` (sub
contract and contract) or `contractParams` (handler); a sub contract that has no `paramOverrides`
is read for the same fields directly. The `ContractTemplate` Records are found by their type, not
by their folder: on LIVE two sit outside `contracts/contracttemplates`.

On LIVE (2026-10-23, 109 `ContractGenerator` Records, 281 handlers, 2,988 contracts, 2,190 sub
contracts) the layout this reading assumes holds: `contractResults` exists on the contract only,
never on a sub contract or a handler; `factionReputation` (and `reputationScope`) exists on the
handler only, on 215 of the 281; the `contractParams` of a handler and the `paramOverrides` of a
contract have the same nine fields; a sub contract has no `debugName`. No handler `debugName` is a
development leftover, so only the contract and the Record are checked for that.

**Title.** The `Title` string parameter of the layers, else the first display string of the
template's `contractDisplayInfo`. A `~mission(Token)` or `~mission(Token|Variant)` in it is
replaced, in each language, by what the first layer (sub contract, contract, handler, template
`contractProperties`) with that `extendedTextToken` says:

- a string-hash property: the text of the first option's `textId`;
- an organisation property: the first organisation's `stringVariants` entry, with a variant the one
  whose tag's `tagName` equals it, without a variant the only entry of an organisation that has
  exactly one (on LIVE 31 of 69 organisations), else none, so the token stays unresolved: the first
  of several entries is tied to a tag (`_from`, `_title_001`, `_desc_001`) and is not the plain
  name of the organisation;
- a location, AI name or ship spawn property, which the game picks when the mission starts:
  `[LOCATION]`, `[TARGET]` or `[SHIP]`;
- anything else, a token no layer states and a text the language lacks: the token in upper case in
  brackets, e.g. `[CARGO]`.

A replacement that itself holds a `~mission(…)` is expanded up to three levels deep; below that,
and past 64 replacements or about 4 KiB in one title, a token becomes `[TOKEN]`. Leading and
trailing blanks are removed from the result; the `key` is that of the title text. The tokens are free text of the game, not facts of the Archive;
the placeholders are the catalog's own convention.

**Broker entry.** A contract that names a `missionBrokerEntry` and has no template (on LIVE the 419
`Covalex_HaulCargo_Career` contracts and their two sub contracts) takes what the layers and the
template leave open from that `MissionBrokerEntry` Record: its `title`, its `type` and, when no
pickup place is left from the prerequisites (none named, or only stars or objects that are not in
`starmapObjects`), its `locationMissionAvailable` as the pickup place.

**Mission type.** `missionTypeOverride` of the first layer that states one, else the `type` of the
template's `contractDisplayInfo`, else of the broker entry; `missionType` is the
`LocalisedTypeName` of that Record.

**Reputation.** Of the results of the contract (`contractResults.contractResults`), those of type
`ContractResult_LegacyReputation` whose first `missionResults` entry (success) is set, one
`reputationGains` entry per `contractResultReputationAmounts` element, in stored order: the
faction and scope Records with their Record names (the last part of the record path) and the
`reputationAmount` of the reward Record. A result that does not fire on success is not listed. An
amount can be negative: on LIVE the 48 negative amounts (of 2,809 success amounts) all belong to
the `CFP_OOS`, `CFP_OOS_Intro`, `HH_OOS` and `HH_OOS_Intro` handlers, twelve contracts each. In
every one the success result names the rival faction (a `CFP_` handler deducts
`factionreputation_unlawful_headhunters`, a `HH_` handler `factionreputation_lawful_citizensforprosperity`)
with the reward `repuationrewardamount_negative_bigdrop` (-190500); they are listed as stored, next
to the handler's own faction in `faction`. The catalog does not add the amounts up (ADR-0065): a
faction scope and an affinity scope are separate entries. All sub contracts of a contract list the
contract's results.

**Contractor.** The `Contractor` string parameter of the first layer (sub contract, contract,
handler) that states one: the text the game shows as the giver, e.g. of an agency that has no
faction reputation (game text).

**Pickup places.** The starmap objects named by the prerequisites of the sub contract
(`additionalPrerequisites`), or, when it names none, of the contract and then of the handler
(`defaultAvailability.prerequisites`): the `availableLocations` of each
`ContractPrerequisite_Locality` (a `MissionLocality` Record) and the object of each
`ContractPrerequisite_Location`, in stored order, none twice. A star (`type` `Star`) is left out and
what is not in `starmapObjects` is skipped. The list is ordered by rank, stably: the star system
(`SolarSystem`) first, then planets and moons (`Planet`, `Moon`), then every other type, and holds
at most **5** places (`MAX_PICKUP_PLACES`). Each place has the `starmapObjectGuid`, its `objectType`
and the `place` name (game text). The places come from the starmap objects of the same build: a
Filter that leaves them out leaves `pickupPlaces` empty.

**Known gaps (LIVE, 2026-10-23, 5,175 entries before the template fix).** None of them is stated
elsewhere in the Archive:

- 27 entries have no title: 23 name a title key that has no text in the language, and four name no
  title at all (two `FoxwellEnforcement_MWTEST` contracts whose template Record does not exist, and
  two whose template holds only `@LOC_UNINITIALIZED`).
- 4 entries had no mission type: the two contracts that named a template outside the template
  folder now have one; the other two (`FoxwellEnforcement_MWTEST`) name a template Record that does
  not exist and a type nowhere else.
- 22 titles keep a token in upper case brackets: `[RACETYPE]` (14) and `[ITEM]` (2) are
  `MissionPropertyValue_MissionItem` properties (a `MissionItem` Record, e.g. the race type, or
  an empty list the game fills in); the catalog does not read `MissionItem` Records;
  `[CONTRACTOR]` (6) is an organisation property whose list of organisations is empty (chosen at
  start) or whose organisation has several string variants and the title names none. The tokens
  `[CARGOGRADETOKEN]` and `[REPUTATIONRANK]` (411 entries each), `[CLAIMNUMBER]` and `[TARGETNAME]`
  are run-time tokens the game fills in.
- 642 entries have no pickup place: 206 have no location prerequisite and no broker entry, 175
  name only a star, 87 only a broker entry located at a star, 83 a broker entry without a
  location, 85 only a `ContractPrerequisite_LocationProperty` (a variable the game fills in) and
  6 a locality that lists no object of the starmap.

Not exported: a computed reward (`ContractResult_CalculatedReward` holds no value, ADR-0065) and
descriptions.

### Starmap objects

A starmap object (`starmapObjects`) is a `StarMapObject` Record under `starmap/pu` (ADR-0054: a
new kind, so `schemaVersion` stays 1). The kind is facts only: no description, no lore, no media.
The `name` is a game text, the localisation key with its English and German text, and carries
`"x-gameText": true` like a description; the Basetool keeps it from third parties.

| Field | Source |
|-------|--------|
| `guid`, `className`, `recordPath` | The Record, as for every entity |
| `name` | `name`, a localisation key |
| `type`, `typeGuid` | The `StarMapObjectType` Record the Reference `type` links to: `type` is its `name` when that is a plain string, else its Record name without the type, and `typeGuid` its GUID; `null` for a null or dangling Reference. The schema keeps `type` a string, so a new type is additive |
| `parentGuid` | The Reference `parent`, the `guid` of another entry unless that Record is missing or left out; `null` for none |
| `hideInStarmap` | `hideInStarmap`; `null` when the Record states none |
| `system` | The folder of the star system below `Data/ObjectContainers/PU/system` that places the object, e.g. `stanton`; `null` when no container places it |
| `entityName` | The `entityName` of the Child element that places the object, e.g. `jumppoint_stanton_pyro`; `null` when it is not placed or the Child names none |
| `placedUnderGuid` | The `guid` of the nearest object above this one in the container hierarchy; `null` when it is not placed or hangs directly below the system. It is a fact of the containers; `parentGuid` is the Record's own and the two need not agree |
| `position` | `[x, y, z]` in metres in the frame of the system container, rounded to a millimetre; `null` when no container places the object |

**Where a position comes from.** The main container of a star system is
`Data/ObjectContainers/PU/system/<folder>/<folder>system.socpak` (else the first `*system.socpak`
of the folder). It is a container (ZIP) whose XML, named like the container, lists
`ChildObjectContainers/Child` elements. A Child places an object with `starMapRecord` (the GUID of
a `StarMapObject`), `pos` (`x,y,z`) and `rot` (a quaternion `w,x,y,z`), may nest further
`ChildObjectContainers`, and with `external="1"` names another container (`name`) whose Children
are placed in the frame of this Child. Positions compose from the top: the position of a child is
rotated by the rotation of its container before it is added, and the rotations multiply, so
`world = parent.position + parent.rotation * child.pos` and `rotation = parent.rotation *
child.rot`. A Child without `rot` has the identity rotation, one without `pos` the origin. Children
without a `starMapRecord` only take part in the composition. ScDataDumper adds the child positions
without rotating them by the container rotation (`Commands/LoadStarmap.php`); the catalog does
not.

A record that more than one Child places keeps its position when every placement lies within 1 m
of the first (the same place reached twice); when two lie further apart the record has no position,
because no placement is the place (`StarmapSummary::ambiguous` counts these records; on LIVE
all are asteroids and wreck sets of the Nyx ring segments, whose containers are instanced several
times). A Child whose `pos` or `rot` is malformed (not numbers, not three or four parts, a
quaternion without length), or whose composed position is not a finite number or lies beyond
1e15 m on an axis (about a tenth of a light year, far outside any star system), and everything
below it are not placed. The same holds for the walk limits: Children nested deeper than 64,
more than 4,194,304 Children in one system, an external container reached through more than 32
others, and an external container that is already open above the Child that names it (a cycle) are
counted and not followed. All of these, and a container that cannot be read, are warned about by
`p4kx catalog` (exit code 1); the counts `unreadable_containers` and `malformed_children` are
counted per reference, not per distinct container, so one container named by ten Children
counts ten times. A container that a Child names and the Archive does
not hold (`missing_containers`, also counted per reference; LIVE: two, an event container and a
module of a Lorville entry) is a fact of the game data, like a patch file the Archive lacks: it is
counted and listed in `CatalogSummary::starmap_report()` but is no warning. The report keeps at
most 300 lines of containers and 300 of issues per star system, each at most 300 characters.
**Rotation order.** `rot` is read as `w,x,y,z`. The raw values of LIVE settle it: the identity occurs as
`1,0,0,0`, the rotations of the Nyx ring segments are `a,0,0,b` (about the z axis, the axis of the
ring) and the small tilts of surface structures `0.98,~0,~0,0.17`; read as `x,y,z,w` the identity would
be a half turn about x and the ring segments rotations about x. Everus Harbor lies 1.04e6 m from
Hurston.

**What no container places.** On LIVE (2026-10-22) 2,093 objects are selected, 1,239 have a
position and 854 have none. Searching all 9,668 object containers of the Archive for the 854 GUIDs gives:

- **816 appear in no object container at all.** No container places them, so no position exists to
  export. By type: Outpost 416 (350 hidden), Asteroid 279 (249 hidden), Asteroid_ValidQT 57 (34
  hidden), Manmade 49, Manmade_VisibleOnInteraction 12, Anomaly 12 (8 hidden; the
  jump point records of Pyro and Nyx), PointOfInterest 23, the 3
  SolarSystem records, and one each of Moon, Planet and Outpost_InvalidQT; 666 of the 854 are
  `hideInStarmap`. They are the Nyx and Pyro points of interest (`Nyx_Kaboos_*`, the Pyro Lagrange
  `*_Enterance` points), mining and quantum-travel markers, rest-stop records, quantum beacons of
  players and missions (`PARTY_MEMBER_QT_Beacon`, `MISSION_QT_*`) and the star systems themselves:
  the game positions them at run time or by other data than an object container.
- **9 appear more than 1 m apart** in the system containers (the instanced Nyx ring segments, see
  above) and are left without a position on purpose.
- **29 appear only in containers that no system container reaches**: `nyx/delamar/delamar.socpak`
  (the private mining points and asteroid clusters of Delamar), the Pyro rest-stop modules under
  `loc/mod/pyro/station/ser/reststop_ext` (the clinics), the Orison Fleet Week 2022 platforms, the
  New Babbage IAE 2023 lobby, and the Early Access maps (`EA/...`, which place `Ellis3` and
  `Stanton4_L1_02`). Their own position in the star system is unknown, because no `external`
  Child of a system container names them, so the position of what they place cannot be composed.

These objects keep `position` `null`; the catalog never guesses a place from the parent.
The kind is selected by the prefix `libs/foundry/records/starmap/pu/` and the type `StarMapObject`
like the others, with the development leftovers left out. The positions do not depend on the
Filter: the containers are read whole. The `StarMapObjectType`, `Jurisdiction`, amenity and radar
Records are not kinds; the type enters as `type` and `typeGuid`.

Size, measured on LIVE (2026-10-22): the 2,093 objects are about 1.2 MB of JSON (570 bytes each), 4.5 % of the 26.5 MB of the
catalog, and a smaller share compressed.

**No released marker.** The Archive states no structural marker that tells a released starmap
object from an unreleased one (ADR-0067, #417): the three star system Records carry the same flags,
`hideInStarmap` is set on helper content of the released systems too, and the type suffixes
(`_ValidQT`, `_InvalidQT`) describe quantum travel. The catalog therefore has no such field; the
Basetool decides from `system`, `position`, `hideInStarmap` and `type`.

### Mineable elements and refining processes

The raw facts only (ADR-0065): the catalog states no instability per commodity, because the
relation is not one to one. The Basetool applies its own rule.

A mineable element (`mineableElements`) is a `MineableElement` Record under `mining/mineableelements`
(46 Records on LIVE, 43 entries). `resourceTypeGuid` is the `ResourceType` its Reference
`resourceType` links to; `null` for a null or dangling Reference. It equals a commodity's
`subtypeGuid` only when the catalog has that commodity: commodities are backfilled only for the
`ResourceType`s a blueprint ingredient uses, so 20 of the 41 linked `ResourceType`s have one on LIVE.
Several elements may share one `ResourceType` (41 distinct for the 43 entries), so a commodity can
have several elements with different values; every element is listed and none is picked. The numbers
are those of the Record, unchanged, and `null` when it states none. The ranges are those of the 43
entries on LIVE:

| Field | Record field | LIVE range |
|-------|--------------|------------|
| `instability` | `elementInstability` | 0 to 1000 |
| `resistance` | `elementResistance` | -1 to 1 |
| `optimalWindowMidpoint` | `elementOptimalWindowMidpoint` | 0.3 to 1 |
| `optimalWindowMidpointRandomness` | `elementOptimalWindowMidpointRandomness` | 0 to 0.3 |
| `optimalWindowThinness` | `elementOptimalWindowThinness` | -1 to 10 |
| `explosionMultiplier` | `elementExplosionMultiplier` | -1000 to 260 |
| `clusterFactor` | `elementClusterFactor` | 0 to 1 |

A refining process (`refiningProcesses`) is a `RefiningProcess` Record under `refiningprocess`
(9 on LIVE, one per pair of speed and quality). `refiningSpeed` (`Slow`, `Normal`, `Fast`) and
`refiningQuality` (`Wasteful`, `Normal`, `Careful`) are closed sets kept as strings, so a new value
is additive; `processName` is the localised label of the method, a game text. On LIVE (2026-10-22)
the English `global.ini` holds all 9 keys and the German one none, so `de` is `null` there. The
DataForge holds no yield per commodity and method: yields are not in the Archive and stay with
other sources.

The 46 element Records of LIVE give 43 entries: the development-leftover rule leaves out
`MinableElement_Ship_template`, `MinableElement_GroundVehicle_template` and
`MinableElement_FPS_template`, counted as skipped. The 9 `RefiningProcess` Records are all entries.
### Shops

A shop (`shops`) is one `ShopBackup` entry of the shop registry `Data\Libs\CosmoTools\
LayerRecovery\LayerBackups.xml` (a CryXml file), one placement of an `SCShop` Entity in the world
(ADR-0069; a new kind, so `schemaVersion` stays 1). The kind is facts only: identifiers, internal
names and places. It carries no price, no stock and no description; the item GUIDs of a shop come only from an
inventory file that its containers name (below). The names in it
are names of containers and entities, not localisation keys, so no field is a game text.

| Field | Source |
|-------|--------|
| `key` | The `ID` attribute of the entry: the CryGUIDs of the containers from the system root down to the `SCShop` Entity, comma separated, each element in lower case. Unique (1,184 distinct on LIVE) and the identity of the shop |
| `entityGuid` | The last element of `key`, the CryGUID of the `SCShop` Entity, lower case. A shop module placed in several places keeps it (235 distinct for 1,184 shops on LIVE), so it is no identity; it is the CryGUID that `metadata\entitycomponentshop.xml` of the shop container names |
| `system` | `PlatformName` of the `LayerBackup` without the suffix `system`, lower case: `stanton`, `pyro` |
| `entityName` | The last element of `SuperName`, the name of the `SCShop` Entity, e.g. `SCShop_CommEx_TDD_NewBabbage` |
| `registryName` | `ShopName`, e.g. `Pharmacy_RestStop_ARC-LEO1`; `null` when empty. A name, not a localisation key; many shops share one (360 distinct non-empty on LIVE) |
| `defaultValue` | `DefaultValue`, a GUID; `null` when empty. Shops with the same `registryName` share it (1 to 1 on LIVE). What it refers to is not known |
| `containers` | The elements of `SuperName` before the entity, each without its `_{GUID}` or `_{GUID}_socpak` suffix and surrounding blanks, from the system root down |
| `body`, `location` | The second and third element of `containers` (`OOC_Stanton_4_Microtech`, `Area18_City_objectContainer`; Pyro: `pyro5`, `rs_ext_pyro6_l3`); `null` when the chain is shorter |
| `starmapObjectGuid` | The `guid` of the `starmapObjects` entry whose `entityName` or `className` equals `location` ignoring ASCII case; `null` when none does or when more than one object does |
| `kind`, `brand` | From the path of the shop container whose `entitycomponentshop.xml` names `entityGuid`: below `Data\ObjectContainers\PU\Shops` the first folder is the kind (`commex`, `clothing`, ...), the second the brand (`tdd`, `mcraes`), the brand `null` when the file lies directly in the kind folder; a container in a `shopcomm` or `shopfood` folder below `PU\loc` has that folder as kind and no brand. `null` for both when no container names the Entity or containers that differ in kind or brand do. The kind is a closed set the game may extend; a reader maps an unknown value to `null` |
| `commodityExchange` | `true` when `entityName` starts with `SCShop_CommEx_TDD_`, or starts with `SCShop_Platinumbay` and ends with `_Commodity` (ignoring case): the commodity exchanges and terminals |

**What the registry holds.** LIVE has two `LayerBackup` elements in the one file: `stantonsystem`
(813 shops) and `pyrosystem` (371), 1,184 entries in all. An entry whose `ID` and `SuperName` chains
differ in length, hold an element of `ID` that is no GUID or an empty container name is left out,
and so is an entry whose `ID` an earlier one already has after lower-casing
(`ShopSummary::malformed` and `duplicate`; on LIVE the 1,184 `ID`s are distinct and the two chains
agree in length everywhere). The chains have 4 to 9 elements.

**Warnings.** An empty `shops` must not read as "the shops were removed", so `p4kx catalog` warns
(exit code 1, `warnings` in the JSON summary) when the registry exists but cannot be read
(`unreadable_shop_registry`), when the Archive holds starmap objects but no registry at the known
path (`missing_shop_registry`, the registry may have moved in the Build) and when shop containers
cannot be read (`unreadable_shop_containers` with a `count`; their shops may lack kind and brand)
or inventory files cannot be read (`unreadable_shop_inventories` with a `count` of files and
`details`, the path and the reason of each; a file with no content at all, such as
`inv_shopping_test_featuretest.json` on LIVE 4.10.197, is one of them, ADR-0141), when
inventory files that containers name are not in the Archive (`missing_shop_inventories` with a
`count` of distinct files; a Build may have moved or renamed them) and when shop containers name
shops but no `.soc` holds a shop component (`no_shop_components`; the layout may have changed).
`ShopSummary::registry_missing` and `registry_unreadable` carry the same facts. With a Filter no
shop is read and none of these warnings is raised.

**On LIVE (Build 4.10.197)** 1,000 of the 1,184 shops link to a starmap object (Pyro: 307 of 371)
and 184 do not; 54 of those have a location name that more than one object carries. The unlinked
locations are mostly Pyro asteroid-cluster and stash containers (`pbay_cluster_001_sand`,
`ab_pyro_asmbl_01_enctr_001a`) and some Stanton ones (`ObjectContainer_RestStop`,
`Jumptown_EchoIsle`) that no starmap record names. 590 shops have a kind and a brand, 60 have a
`null` kind because containers of different kinds name their Entity, and 534 because no container
below `PU\Shops` or the `shopcomm`/`shopfood` folders names it. 16 are commodity exchanges. The 360
distinct `defaultValue`s are no DataForge Record GUIDs.

**The link to the starmap** goes by the name of the location container, as the spike (#398)
found. The summary lists the distinct location names without a link (up to 200) in
`CatalogSummary::shops()`. A shop's exact position inside the interior is not exported: it needs the
`.soc` of the container.

**Kind and brand** need the shop containers: the catalog opens every socpak below `PU\Shops` and
the `shopcomm` and `shopfood` folders below `PU\loc` and reads `metadata\entitycomponentshop.xml`
(CryXml or text). A container that cannot be read is counted, not an error
(`containers_unreadable`).

**Items** (ADR-0071). The inventory files `Data\Scripts\ShopInventories\*.json` list item GUIDs of
`EntityClassDefinition` Records. The `ShopID` chain inside a file is no link to a shop (ADR-0069).
The link is set on the shop side: the `.soc` of a container holds the `SCShop` component of the
Entity (`EntityComponentShop`, `SShopComponentParams`) with the parameter `offlineInventoryJSON` or
`offlineInventoryXMLName`, whose value is a path of an inventory file. The file stores each string
once, so a later component of a file repeats neither the component name nor, often, the GUID of its
Entity; the catalog ties a path to the Entity whose `CryGUID` (`entitycomponentshop.xml`) or object
name (`<container>_editor.xml`) is the closest anchor before it, with only known structure strings
in between (at most one position, and a GUID only right behind `SCShop`), and leaves everything else unknown (ADR-0073). It reads the `.soc` of every socpak below
`PU` and `EA`; kind and brand still come from the shop folders only. The `offlineInventory*`
attributes of the shop prefabs below `Data\Prefabs\Shops` name the Entity through its editor `Id`,
which is the container `CryGUID` with the bytes reordered; a prefab is one more place. A shop gets an
item list when every place that names its `entityGuid` names the same file (compared by lower-case
stem; `.xml` and `.json` name the same file), the Archive holds it and it is no placeholder. Several
`.soc` files of one container follow the same rule. A `.soc` that cannot be read leaves only the
inventory of its container unknown; kind and brand do not depend on it.

| Field | Source |
|-------|--------|
| `inventory` | The lower-case stem of that file, e.g. `inv_admin_newbabbage`; `null` when no place names a file, the places disagree (or only some name one), the file is a placeholder, or the file is missing or unreadable |
| `itemGuids` | The `ID.ID` GUIDs of the elements of the file (`Collection.Inventory`, or `Inventory` in the shape without `Collection`), lower case, ordered, without repeats, and possibly `[]` for a file without items; an `ID` value that is no GUID is left out and counted in `item_ids_dropped`; `null` exactly when `inventory` is. `BuyPrice`, `SellPrice`, `CurrentInventory`, `MaxInventory` and `RentalOfferings` are never read or exported |

The placeholders are `temp_outpost_shopinventory` (the same six commodities for bars, refineries,
showrooms and an expo hall; 264 shops) and `inv_cryastro_dummy` (three GUIDs no Record has; 136
shops): they say nothing of what a shop sells, so those shops have no list
(`inventory_placeholder`).

`ShopSummary` counts `with_items`, `inventory_ambiguous`, `inventory_placeholder`,
`inventory_unscanned` (no place has the component in a readable `.soc`: nothing is known, which
differs from a component that names no file), `inventory_missing` and `inventory_unreadable`
(shops), `soc_components` (found components), `inventories_named` and `inventories_in_archive`,
`inventories_missing`, `inventories_unreadable` (files), `item_ids_dropped`, and the containers
(`other_containers_*`) and prefabs (`prefabs_*`, `prefab_entities`) read besides the shop folders.
On LIVE (Build 4.10.197) 294 of the 1,184 shops get an item list (Stanton 275, Pyro 19) from 33
distinct files; 399 shops name a placeholder, 43 have ambiguous places, 64 are unscanned and one
names an empty file. 57 of the 119 inventory files are named by a container or prefab; the other 62
(event sales, `mining_*`, most landing-service, city and station shop files) are named by nothing
in the Archive and by no Record, and the `ShopID` they carry is a stale design-time value, so those
shops have no item list. 2,395 of the 2,422 item GUIDs of the exported files (distinct per file) are
the `guid` of an entry of `items`, `ships`, `commodities`, `paints` or `loadouts`; the other 27 (all
21 of `inv_commex_tdd_area18`, 6 of `inv_astroarmada_area18`) are no Record of the DataForge. Prices,
stock and demand are never exported.

**Filter.** Shops are selected by no record path. With a Filter no shop is read and `shops` is empty
with the status `partial`.

### Crafting data

`p4kx_export::read_crafting(&forge, filter, cancel)` returns a `Crafting` with everything the
`blueprints` kind and the crafting kinds carry; the catalog is built from it, so library and
catalog never drift. It reads the DataForge in parallel (0.3 s on LIVE, the scan for the reward sources included,
against 14 s for the whole catalog) and leaves out a Record it cannot read, listing it under
`failures`. Every list is ordered by Record GUID. The values are facts and identifiers only: no description, no
lore, no media.

The blueprints are selected by the one rule behind the `blueprints` kind (`selects_blueprint`:
prefix, type, Filter, leftover tokens). A `Filter` keeps only the blueprints and pools it matches;
the other lists are always read, and the catalog then keeps the properties, quality distributions
and dismantle processes whose record path matches.

The reward sources come from one scan of the Record tree of every Record for a Reference to a pool
(ADR-0154), so nothing is capped. A Record the scan cannot read is no `failures` entry, because it
is not known to be selected; it could hide a source, so `rewardSources` may be incomplete exactly
when `Crafting::warnings` holds `UnreadableRewardRecords` with the count. Only an unreadable Record
whose struct type is that of a Record with a reward source counts, and when no source is found at
all every unreadable Record counts. `p4kx catalog` prints the warning and exits with code 1
(`CatalogSummary::warnings()`; `is_complete()` is then `false`); the windows of the GUI list it.

A modifier range (`valueRanges` of a gameplay-property modifier) is read only for the two types
`CraftingGameplayPropertyModifierValueRange_Linear` and `..._LinearIntegerAdditive`. A range of any
other struct type is left out of the modifiers of its tier and counted:
`Crafting::warnings` then holds `UnknownModifierRanges` with the number of such ranges over all
selected blueprints and the sorted, distinct struct types in `details`
(`BlueprintTiers::unknown_range_types` has them per blueprint). A build without such a range has
no warning. The shape of the catalog does not change, so this follows ADR-0054 as an additive
extension: only the `warnings` of `p4kx catalog --json` gain the kind `unknown_modifier_ranges`.

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

LIVE holds exactly one tier per blueprint; the library
reads as many as a Record holds. The dismantle Record is generic, not one per item: the link from
an item to its returns runs through the blueprint that produces it.

The assumptions of `dismantle_return()` are those under "Dismantle returns" above: the first tier,
no resolved `Select`, no applied quantity multiplier, the resource blacklist exposed as
`excluded_resources` and never applied to an amount.

A blueprint's `name` is almost always a placeholder key. The Basetool names a blueprint through
`producedItemGuid`, the item it produces, not through `name`.

Manufacturer GUIDs are always taken from References, never from embedded Records (ADR-0004).

### Ship store slugs

The Archive does not carry the slugs of the RSI pledge store, and the class names of the ships do
not match them. `shipStoreSlugs` maps the slug of a store ship to the catalog ship (ADR-0117). It
is curated data that ships with the tool (`crates/p4kx-export/data/ship-store-slugs.json`), not
extracted data. Only identifiers are kept: no description, price or media.

| Field | Meaning |
|-------|---------|
| `slug` | The last segment of the ship's pledge URL, e.g. `Aurora-Mk-I-ES`; the entries are ordered by it |
| `rsiId` | The ship id of the RSI store |
| `storeName` | The name the RSI store shows |
| `className` | The `className` of the mapped ship; `null` for a retired slug |
| `shipGuid` | The `guid` of the mapped entry of `ships`; `null` for a retired slug |
| `retired` | `true` when the slug's ship no longer exists in the game data |

The kind lists only slugs that map to a ship: the slugs of ships the game does not have (the
store lists concept ships) are not exported. A slug is exported when its `shipGuid` and `className`
resolve to a ship of the catalog, or when it is `retired`; a retired slug stays in the data file
and in the catalog so that a pledge of it is recognised as retired and not as unknown. A mapping
that no longer resolves and is not retired is *stale*: it is not exported, `p4kx catalog` prints
it and lists it in `store_slugs.stale` of `--json`, and the real-Archive test
`every_mapped_store_slug_resolves_to_a_ship_of_the_archive` fails. With a `--filter` the ships are
a selection, so only the slugs that resolve are exported, no retired one, and none is stale; the
kind is `partial` then. When slugs are mapped but none resolves (an Archive that is not the
game's, such as a test fixture), every mapping would be stale: the check is skipped, `stale` stays
empty and `store_slugs.stale_check_skipped` is `true`.

The mapping is made by name, manufacturer and variant against the ships of the `LIVE` Archive whose
`playability` is `player`, and it never guesses:

- the store name and the catalog `name` are compared after dropping the manufacturer, punctuation,
  case and the word "Edition";
- when several player ships share the name, the one whose `className` carries none of the
  suffixes of a display, test, event or reward variant (`_BIS…`, `_Display`, `_TEMP…`, `_Unmanned`,
  `_AI_Super`, `_Tutorial`, `_Collector_…`, `_Prison`, `_NoInterior`, `_Hijacked…`, `_Exec_…` and the
  like) is taken; if more than one is left, the slug stays unmapped;
- a store name that is a part of exactly one catalog name (`M50` for "Origin M50 Interceptor", `C2
  Hercules`, `Mercury`, `Ares Ion`, `85X`, `C8R Pisces`) is accepted after review of each such slug,
  and so is `F7C-M Super Hornet Heartseeker Mk I` for `ANVL_Hornet_F7CM_Heartseeker`, the only ship
  of that name apart from the word "Super".

On `LIVE` 4.7 the 256 slugs of the store (227 flight-ready, 29 in concept) give 205 mapped slugs
and 51 unmapped: the 29 concept ships, 5 whose name fits several equal ships (the four Constellation
Mk IV variants and the Hammerhead, each present once as `_GS` and once without), and 17 flight-ready
ones the Archive holds under another name or not at all (e.g. `Mustang Alpha Vindicator`, `Carrack
w/C8X`, `600i Explorer`, `ATLS`).

Seven of the unmapped slugs have a ship under the store name that the player-only rule excluded,
because its `playability` is `null` (no rule classifies it): `GAMA_Railen`, `GAMA_Tyilui`,
`GLSN_Basher`, `KRIG_S65_Stingray`, `RSI_Skylark`, `RSI_Constellation_Mk5_Centaurus` and
`AEGS_Sabre_Raven_EX`. They can be mapped once their playability is settled. 119 ships with
`playability` `player` have no slug; most are event, reward and display variants of a ship that
has one.

To refresh the file, a human fetches the public ship list of the store, reduces it to the
identifiers above (`id`, name, pledge URL), and adds new slugs to `ship-store-slugs.json` with
`className` and `guid` `null`; the tool never fetches anything. Then `p4kx catalog` on the
current `LIVE` Archive and its `store_slugs.stale` and `unmapped` lists show what is left to do: a
new ship is mapped by the rules above, a mapping that went stale is corrected, or marked
`"retired": true` when the ship is gone. The real-Archive test must pass before the file is
committed.

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
| `loadouts` | Those of `ships`, and of `items` whose Record defines ports or names a default Loadout | `EntityClassDefinition` |
| `starmapObjects` | `libs/foundry/records/starmap/pu/` | `StarMapObject` |
| `contracts` | `libs/foundry/records/contracts/contractgenerator/` | `ContractGenerator` |
| `mineableElements` | `libs/foundry/records/mining/mineableelements/` | `MineableElement` |
| `refiningProcesses` | `libs/foundry/records/refiningprocess/` | `RefiningProcess` |
| `shops` | None: the `ShopBackup` entries of `LayerBackups.xml`, no Records | |

The blueprints and the four crafting kinds below them are read by `read_crafting`, with the one
selection rule behind it (`selects_blueprint` for the blueprints); `Kind::rule()` of
`qualityDistributions` names the first of its two types.

The resource-group prefix also holds the 207 `ResourceType` Records and the `ResourceTypeDatabase`
Record of LIVE, and the category prefix a `BlueprintCategoryDatabaseRecord`; the type selects only
the groups and the categories.

The blueprint prefix leaves out `crafting/blueprints/dismantle/`, whose Record is no recipe. The
`$templates` folders below it hold 27 real blueprints, each the only recipe for its item, and
stay: the token `templates` is not `template`.

The 46 `mining/mineableelements` Records hold no commodity of their own and are the kind
`mineableElements`; the `ResourceType` Records enter through the ingredients that use them.

An item without an `SAttachableComponentParams` component, such as a door or a shop counter, is
still an item; its definition fields are then `null` and `tags` is empty.

The prefix ignores case and the kind of separator; the type is the Record's struct, matched
exactly. `--filter` further selects by record path with the rules of `p4kx search` and marks
every kind partial.

Development leftovers are left out and counted as skipped: a selected Record whose record path or
name holds one of the tokens `test`, `template`, `dummy`, `debug`, `deprecated` or `dev` as a
whole word, ignoring case. Words are the runs of ASCII letters and digits, so `paint_test_01` is
left out while `contestant` stays. For the ships kind the words `temp`, `display` and `hijacked`
count as well, so `glsn_shiv_temp_cosa` and `aegs_avenger_stalker_display` are left out; the other
kinds keep their `display` and `temp` Records (radar displays, shop displays and the like). This
changed the selection of `ships` for `schemaVersion` 2 (ADR-0167): 21 ships on LIVE are skipped
that version 1 listed.

A selected Record that cannot be read, such as one whose struct layout is corrupt, does not stop
the catalog: it is left out, counted as a failure and reported with its record path, GUID and
reason (a ship whose loadout cannot be resolved is the exception: it stays in `ships`, see
"Loadouts"). `p4kx catalog` prints each failure on stderr, lists them under `failed` in `--json` and
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
small fraction of it. The `loadouts` kind (ADR-0064) then grew the plain catalog of LIVE to
132 MB (153 MB by 2026-10-26), which is **over the 64 MB upload limit: the upload must be
`.json.zst`** (2.7 MB), and
plain `.json` is for inspection only. `p4kx catalog` and the window warn about a plain file over
the limit and advise `.json.zst`. The real-Archive tests write the catalog as `.json.zst`, assert
that the compressed size is within 64 MB and the decompressed size within 1 GB, decode the frame
and compare it with the plain catalog, and print both sizes and the time per zstd level.

Measured on LIVE on 2026-10-26 (complete catalog of every kind, English and German texts; Bytes
is the whole catalog, the other kinds are in `p4kx catalog --json`):

| Channel | Bytes | Manufacturers | Items | Ships | Commodities | Blueprints | Resource groups | Blueprint categories | Properties | Distributions | Pools | Dismantle | Paints | Loadouts |
|---------|------:|--------------:|------:|------:|------------:|-----------:|----------------:|---------------------:|-----------:|--------------:|------:|----------:|-------:|---------:|
| LIVE | 152,812,265 | 1,201 | 23,280 | 1,087 | 145 | 1,618 | 19 | 21 | 29 | 21 | 160 | 1 | 1,064 | 10,708 |

Measured on 2026-10-22, LIVE holds 10,708 loadouts (1,087 ships, 9,621 items) with 293,451 ports
(nested ports counted for every ship that carries them), 152,081 of them holding an Entity, 65 docked vehicles and 505
unresolved attachments (`unknown_class` 446, `dangling_reference` 54, `cycle` 5); 61 attached
Entities have no entry in `items` or `ships` (templates of doors, screens, beds and the like, and
HUD projectors); 193 loadouts list an issue (`unknown_class` 153, `dangling_reference` 30,
`xml_loadout` 4, `cycle` 3, `vehicle_implementation` 3; a loadout can list several).

Written as `.json.zst` (level 19, window log 27), measured on LIVE on 2026-10-26:

| Channel | Uncompressed | Compressed | Ratio |
|---------|-------------:|-----------:|------:|
| LIVE | 152,812,265 | 2,725,280 | 56.1 |

The compressed catalog is 4.3 % of the 64 MB upload limit; the decompressed one is 15.3 % of the
1 GB cap. The level trades time for size (measured before the loadouts, LIVE, window log 27; the
plain build then took 2.1 s):

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

On 2026-10-22 the loadouts took about 105 MB of the 132 MB of LIVE. A catalog builds on LIVE in 4.7 s with the
ship base fields and the loadouts (5.5 s before both). Without the loadouts, with the ship base fields,
LIVE builds in 3.2 s in a release test run (the `p4kx ships` export alone: 2.9 s). A first version
of the ship base fields that gave every split of the parallel scan its own ship reader took
10.5 s, because each reader began with empty Entity caches; the scan now keeps the readers in a
pool, so a reader keeps its caches across splits. Writing the `.json.zst` takes 19 to 22 s per
Channel in all. Left out as development leftovers on
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
