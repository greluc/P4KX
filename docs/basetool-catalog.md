# Basetool catalog

The Basetool catalog is the JSON Export the basetool's P4K import consumes (ADR-0002). Its contract
is the JSON Schema (draft 2020-12) in
[`schemas/basetool-catalog.schema.json`](../schemas/basetool-catalog.schema.json), generated from
the Rust types in `crates/p4kx-export/src/contract.rs`. A test fails when the checked-in schema and
the types drift apart; `P4KX_BLESS=1 cargo test -p p4kx-export --test catalog_schema` regenerates
it. Every change of the shape raises `schemaVersion`.

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

## Selection

| Kind | Record-path prefix | Type |
|------|--------------------|------|
| `manufacturers` | `libs/foundry/records/scitemmanufacturer/` | `SCItemManufacturer` |

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
