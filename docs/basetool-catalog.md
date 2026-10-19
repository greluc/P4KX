# Basetool catalog

The Basetool catalog is the JSON Export the basetool's P4K import consumes (ADR-0002). Its contract
is the JSON Schema (draft 2020-12) in
[`schemas/basetool-catalog.schema.json`](../schemas/basetool-catalog.schema.json), generated from
the Rust types in `crates/p4kx-export/src/contract.rs`. A test fails when the checked-in schema and
the types drift apart; `P4KX_BLESS=1 cargo test -p p4kx-export --test catalog_schema` regenerates
it. Version 1 stays open until the Basetool import reads it: kinds and fields are added to it
until M3 is complete; from then on every change of the shape raises `schemaVersion`.

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
| `source.archiveSize` | The size of the Archive file in bytes |
| `source.archiveModified` | The modification time of the Archive file, UTC in RFC 3339 form |
| `source.toolVersion` | The version of P4K-Extractor that wrote the catalog |
| `source.exportedAt` | When the catalog was written, UTC in RFC 3339 form |
| `completeness.<kind>` | `{"status": "complete"}`, or `{"status": "partial", "filter": "…"}` when a Filter selected the Records |
| `manufacturers` | The manufacturers, ordered by GUID |
| `items` | The items, ordered by GUID |
| `ships` | The ships and ground vehicles, ordered by GUID |

A complete kind lists every Record of its kind in the Build, so the import may treat a Record it
knows but no longer finds as removed. A partial kind lists only what the Filter selected.

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
class, with another value, and without the line or an English description.

A ship takes its fields from `VehicleComponentParams`:

| Field | Source |
|-------|--------|
| `name`, `description` | `vehicleName` and `vehicleDescription` |
| `role`, `career` | Texts: `vehicleRole` and `vehicleCareer`, e.g. `@vehicle_class_lightfreight` |
| `crewSize` | `crewSize`, an integer |
| `manufacturerGuid` | The GUID the Reference `manufacturer` links to, `null` for a null Reference |
| `vehicleDefinition` | `vehicleDefinition`, the path of the vehicle XML |

Manufacturer GUIDs are always taken from References, never from embedded Records (ADR-0004).

### Texts

`key` is the localisation key as the Record holds it, e.g. `@manufacturer_NameACAS`; an empty key
is `null`. `en` and `de` are its texts from the Archive's `global.ini` of that language:

| Language | Entry |
|----------|-------|
| `en` | `Data\Localization\english\global.ini` |
| `de` | `Data\Localization\german_(germany)\global.ini` |

Each line `key=value` of a `global.ini` holds one text. Keys match without the leading `@` and
ignoring case; a suffix after a comma (`ASD_01,P`) is not part of the key. The text is the rest of
the line as the game stores it, except that the escape `\n` becomes a line feed; markup, a lone
carriage return and every other sequence stay as they are.

A key the language's `global.ini` lacks or holds with an empty text (`key=`), or a language the
Archive holds no `global.ini` for, leaves that text `null` and keeps the key, so the basetool can
resolve it later; it counts as unresolved.

The placeholder keys `@LOC_EMPTY`, `@LOC_PLACEHOLDER` and `@LOC_UNINITIALIZED` (ignoring case,
`PLACEHOLDER_KEYS` in `crates/p4kx-export/src/selection.rs`) mean "no text": the key stays,
`en` and `de` are `null` even where the `global.ini` holds a text for them such as
`<= PLACEHOLDER =>`, and they count as placeholders, neither resolved nor unresolved.

`p4kx catalog` counts the resolved, unresolved and placeholder texts per language in its summary.

## Selection

| Kind | Record-path prefix | Type |
|------|--------------------|------|
| `manufacturers` | `libs/foundry/records/scitemmanufacturer/` | `SCItemManufacturer` |
| `items` | `libs/foundry/records/entities/scitem/`, `libs/foundry/records/entities/legacyitems/` | `EntityClassDefinition` |
| `ships` | `libs/foundry/records/entities/spaceships/`, `libs/foundry/records/entities/groundvehicles/` | `EntityClassDefinition` |

An item without an `SAttachableComponentParams` component, such as a door or a shop counter, is
still an item; its definition fields are then `null` and `tags` is empty.

The prefix ignores case and the kind of separator; the type is the Record's struct, matched
exactly. `--filter` further selects by record path with the rules of `p4kx search` and marks
every kind partial.

Development leftovers are left out and counted as skipped: a selected Record whose record path or
name holds one of the tokens `test`, `template` or `dummy` as a whole word, ignoring case. Words are
the runs of ASCII letters and digits, so `paint_test_01` is left out while `contestant` stays.

## Determinism and size

Equal Archives and an equal export time give byte-identical catalogs. The export time is the
current time unless `--exported-at` or the `SOURCE_DATE_EPOCH` environment variable gives it in
seconds since 1970-01-01 UTC.

### Upload limit

`UPLOAD_LIMIT` in `crates/p4kx-export/src/summary.rs` is the largest catalog the basetool accepts,
now 64,000,000 bytes (64 MB). It must match the basetool's upload limit: when one changes, the
other changes with it. The limit is sized to what a full catalog of every kind needs, with enough
buffer to upload it whole; the real-Archive test prints the size of the current catalog
(`catalog size: … bytes`) as the measurement to size it from.

`p4kx catalog` warns on stderr when a catalog reaches 90 % of the limit and when it exceeds it;
the file is written either way.
