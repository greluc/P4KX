# Quality quantizations link to resource types from the resource type side

Decided for issue #692. Builds on ADR-0002, ADR-0004 and ADR-0054.

## Decision

- **The catalog has a kind `qualityQuantizations`** for the `CraftingQualityQuantizationRecord`
  Records under `crafting/qualityquantization` (63 on LIVE, 62 without the `TEMPLATE` leftover).
  Each carries its `bands` (`start`, `end`, `mappedValue`) and `resourceTypeGuids`. It is part of
  the unreleased `schemaVersion` 2, with no further bump.
- **The link is derived from the `ResourceType` Records.** A quantization Record holds no
  Reference to a resource; each `ResourceType` names its Record in
  `properties[].qualityQuantization.qualityQuantizationRecord`. The reader collects those links
  from every `ResourceType` Record whatever the Filter and stores them as GUIDs, never embedded
  (ADR-0004). One Record may serve several `ResourceType`s (`Carinite`, `CarinitePure`).
- **An unused Record stays in the kind** with an empty `resourceTypeGuids` (20 on LIVE). It is
  ordinary game data, not a defect, so it is only counted in `CatalogSummary::quantizations()`
  (`unused`) and in the `quantizations` object of `p4kx catalog --json`, like `paints.unassigned`;
  it raises no warning and does not change the exit code.
- **Links that reach no quantization are counted, not dropped silently:** a link of a
  `ResourceType` to a Record of another type, to a Record that failed, or to a development
  leftover counts in `unresolved_links` (0 on LIVE); a Reference to a GUID no Record has reads as
  null and cannot be told from a null one. A `ResourceType` that is itself a
  development leftover neither links nor counts.
- **A Record that cannot be read is a failure and left out:** no bands, a band without whole
  `start`, `end` or `mappedValue`, or a band that ends before it starts.
- **Only identifiers and numbers are exported.** No text, description or media.

## Consequences

- A `resourceTypeGuids` entry equals a commodity's `subtypeGuid` only when the catalog has that
  commodity: 18 of the 43 links on LIVE do.
- `p4kx catalog` on LIVE is unaffected: the kind adds no warning.
