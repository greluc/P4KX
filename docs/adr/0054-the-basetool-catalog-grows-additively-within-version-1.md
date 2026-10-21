# The Basetool catalog grows additively within version 1

Decided for issue #391, by the owner on 2026-10-21. Amends ADR-0002.

ADR-0002 left version 1 open only until the Basetool import reads it, and then raised
`schemaVersion` for every change of the shape. The Basetool wants the Archive as its leading data
source and so needs more kinds and fields over time (playability, resource groups, ship base data,
full crafting data, paints). With the old rule each of them would break the import until a Basetool
release adapts it, even when the import does not use the addition.

## Decision

- **Additive changes keep `schemaVersion`:** a new kind (`paints` next to `items`), a
  new field on an entity, on `source` or on an ingredient, a new kind in `completeness`. Example:
  adding `playable` to every ship stays version 1.
- **Breaking changes raise `schemaVersion`:** renaming, removing or retyping a field, a changed
  meaning, restructuring a kind, or a changed selection or ordering of an existing kind. Example:
  renaming `crewSize` to `crew`, or turning `tags` from a list into a string, makes version 2.
- **A reader ignores kinds and fields it does not know.** The Basetool import does so, and still
  rejects a `schemaVersion` it does not know.
- **The checked-in schema accepts unknown properties.** The catalog types no longer carry
  `deny_unknown_fields`, so the generated JSON Schema has no `additionalProperties: false`. A
  reader that validates against an older schema does not fail on an additive field. Every field of
  the schema's own shape stays required; the catalog still writes `null`, never omits.
- **Loadouts (#393) go into a separate Export,** not into the catalog. The catalog is 21 MB of the
  64 MB upload limit on LIVE, and ports and Default Loadouts would grow it considerably and
  lengthen the import.
- **Game texts:** `description` (every kind) and `displayType` are game texts. Identifiers, short
  names (`name`, `role`, `career`, `code`) and fact values may reach third parties; descriptions
  and lore never. The catalog keeps its `description` fields for the Basetool's logged-in members,
  and the Basetool keeps them from third parties.
- **Channels:** a production import takes `LIVE` catalogs only. `PTU`, `EPTU`, `TECH-PREVIEW` and
  `HOTFIX` are confidential test Channels (ADR-0038).

## Consequences

- A catalog written with a new field validates against the older schema; the schema still describes
  what a version-1 reader may rely on.
- A new field is documented in `docs/basetool-catalog.md` and in the schema in the change that adds
  it; the schema test keeps the two in step.
- Deciding whether a change is breaking is part of review: a change that makes an existing reader
  misread data, even without touching a name or a type, raises the version.
