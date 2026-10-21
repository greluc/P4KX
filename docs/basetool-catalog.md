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
  content is capped at 1 GB. On LIVE, 19.3 MB becomes 1.54 MB with zstd -19 `--long=27`.
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
p4kx catalog schema
```

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
| `blueprints` | The crafting blueprints with their ingredients, ordered by GUID |

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

A manufacturer adds `code`, e.g. `ACAS`.

An item takes its fields from the components of its `EntityClassDefinition`:

| Field | Source |
|-------|--------|
| `name`, `description` | `SAttachableComponentParams.AttachDef.Localization.Name` and `.Description` |
| `type`, `subType` | `AttachDef.Type` and `AttachDef.SubType`, e.g. `PowerPlant` and `Power` |
| `size`, `grade` | `AttachDef.Size` and `AttachDef.Grade`, integers |
| `mass` | `SEntityPhysicsControllerParams.PhysType.Mass`, a number |
| `manufacturerGuid` | The GUID the Reference `AttachDef.Manufacturer` links to, `null` for a null Reference |
| `tags` | The words of `AttachDef.Tags`, split at white space |
| `displayType` | A text: `SCItemPurchasableParams.displayType`, e.g. `@item_displayType_Pants` |
| `itemClass` | `Civilian`, `Competition`, `Industrial`, `Military` or `Stealth`, or `null` |

`itemClass` takes the options of the DataForge enum `EItemClass` (without `UNDEFINED`). No
DataForge field carries it: it is read from the first line of the English `description` that
starts with `Class:`, its value trimmed of white space and matched ignoring case. Any other value
(FPS weapons use the line for the damage type, e.g. `Ballistic`), an item without that line and
an item without an English description leave it `null`. `p4kx catalog` counts the items with a
class, with another value, and without the line or an English description. The schema types the
field as a string; the five options are documented, not enumerated.

A ship takes its fields from `VehicleComponentParams`:

| Field | Source |
|-------|--------|
| `name`, `description` | `vehicleName` and `vehicleDescription` |
| `role`, `career` | Texts: `vehicleRole` and `vehicleCareer`, e.g. `@vehicle_class_lightfreight` |
| `crewSize` | `crewSize`, an integer |
| `manufacturerGuid` | The GUID the Reference `manufacturer` links to, `null` for a null Reference |
| `vehicleDefinition` | `vehicleDefinition`, the path of the vehicle XML |

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

Not represented:

- The count of a selection ("choose n"): every option is listed, though 3 of the 1,618 LIVE
  blueprints let the player choose fewer than they offer.
- The `CraftingCostContext_QuantityMultiplier` (factor 1.15 on 3 lines of LIVE): the quantities are
  the stored ones, the multiplier is ignored.
- All tiers, quality modifiers, quality distributions, reward pools and dismantle returns.

The library reads all of this already: see "Crafting data" below. The catalog shape for it follows
in #396.

### Crafting data

`p4kx_export::read_crafting(&forge, filter, cancel)` returns a `Crafting` with everything the
`blueprints` kind does not carry. It reads the DataForge in parallel (0.6 s on LIVE, plus the
reference index for the reward sources) and leaves out a Record it cannot read, listing it under
`failures`. Every list is ordered by Record GUID. The values are facts and identifiers only: no
description, no lore, no media.

| Field | Source |
|-------|--------|
The names below are the Rust names of the library, not contract names; the catalog names follow
in #396. A `Filter` keeps only the blueprints and pools it matches; the other lists are always
read.

| Field | Source |
|-------|--------|
| `blueprints` | The same set as the `blueprints` kind. Each has `produced_item` and `tiers`: per tier `craft_time_seconds`, `mandatory` and `optional` as a tree of `Cost` |
| `Cost::Select` | `count` ("choose n") with its `options`, `slot` and `slot_key`; `chooses_fewer()` tells whether the player may choose fewer than offered |
| `Cost::Resource`, `Cost::Item` | As an ingredient (`resource` / `item`, `quantity_scu` / `quantity`, `min_quality`), before the context |
| `CostContext` | `quantity_multiplier` (`CraftingCostContext_QuantityMultiplier`), `composition_inclusion` (`Include` or `Exclude`) and `modifiers` (`CraftingCostContext_ResultGameplayPropertyModifiers`): per gameplay property the linear ranges `start_quality`, `end_quality`, `at_start`, `at_end` as a factor (`Multiplier`) or a summand (`Additive`) |
| `properties` | `CraftingGameplayPropertyDef` Records under `crafting/craftedproperties`: GUID, `class_name`, `record_path` and the localisation key of the name |
| `quality_distributions` | The Records under `crafting/qualitydistribution`: a default distribution (`min`, `max`, `mean`, `stddev` of a normal distribution) or the list of location overrides, each a location GUID with its distribution |
| `pools` | `BlueprintPoolRecord` Records under `crafting/blueprintrewards`: entries with a blueprint GUID and a `weight`; `total_weight()` is the sum of the positive weights |
| `reward_sources` | Every struct of any Record that holds a Reference to a pool (on LIVE the `BlueprintRewards` result of a contract): the Record, the `id` and `debugName` of the nearest enclosing contract, the pool and the `chance` |
| `reward_chances()` | Per blueprint the chance of each source: `chance` x `weight` / `total_weight()`; a source whose chance is outside 0 to 1 and an entry with a weight of zero or less give none |
| `dismantle` | The `GenericCraftingProcess_Dismantle` Records under `crafting/blueprints/dismantle`: `efficiency` and `dismantleTime` in seconds (LIVE: one process, 0.5 and 15 s) |
| `global` | `CraftingGlobalParams`: the default blueprints (`defaultBlueprintSelection`), the resources dismantling never returns and the entities that cannot be dismantled |
| `dismantle_return()` | Links a blueprint to the first dismantle process and to the item it produces, unless the item is on the item blacklist |

LIVE holds exactly one tier per blueprint (also PTU, EPTU, TECH-PREVIEW and HOTFIX); the library
reads as many as a Record holds. The dismantle Record is generic, not one per item: the link from
an item to its returns runs through the blueprint that produces it.

Assumptions of `dismantle_return()`, not verified against the game:

- A dismantled item returns the share `efficiency` of the ingredients of the blueprint that
  produces it. The library does not compute the amounts and does not say which tier applies; the
  consumer picks one (the first tier is the only one on LIVE).
- A `Select` ("choose n") of the ingredients is not resolved: which options a dismantle returns is
  unknown.
- The resource blacklist (`global.dismantle_blacklist_resources`) is not applied to any amount;
  only the item blacklist decides whether a link exists.

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

`UPLOAD_LIMIT` in `crates/p4kx-export/src/summary.rs` is the largest catalog the basetool accepts,
64,000,000 bytes (64 MB). It must match the basetool's upload limit: when one changes, the other
changes with it. The owner kept the limit at 64 MB after the measurement below: the largest
catalog (21.1 MB) is about a third of it, so the catalog can grow by a factor of three before the
limit is reached. The real-Archive tests assert that the limit is at least twice the catalog and
print the size (`catalog size: … bytes`).

Measured on 2026-10-19 with every installed Channel (complete catalog of every kind, English and
German texts):

| Channel | Bytes | Manufacturers | Items | Ships | Commodities | Blueprints |
|---------|------:|--------------:|------:|------:|------------:|-----------:|
| LIVE | 21,052,438 | 1,201 | 23,280 | 1,087 | 145 | 1,618 |
| PTU | 20,727,734 | 1,172 | 22,945 | 1,054 | 145 | 1,618 |
| EPTU | 20,339,175 | 1,154 | 22,788 | 1,049 | 145 | 1,607 |
| HOTFIX | 21,052,440 | 1,201 | 23,280 | 1,087 | 145 | 1,618 |
| TECH-PREVIEW | 21,112,018 | 1,203 | 23,635 | 1,035 | 145 | 1,618 |

A catalog builds in 2 to 7 seconds. Left out as development leftovers on LIVE: 2 manufacturers,
1,145 items and 57 ships (1,204 Records).

### Verification against the live data

Checked on all five Channels; `crates/p4kx-export/tests/real_archive_verification.rs` repeats it
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

  The `p4kx catalog` summary counts them over all texts: English 37,963 resolved, 160 unresolved,
  German 25,497 resolved, 12,626 unresolved, 23,658 placeholders in each.

`p4kx catalog` warns on stderr when a catalog reaches 90 % of the limit and when it exceeds it;
the file is written either way.
