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
- **An unlinked Record stays in the kind** with an empty `resourceTypeGuids` and is counted in the
  warning `unlinked_quality_quantizations` (20 on LIVE). It is a fact of the game data, not a
  defect of the export, so the real-Archive tests treat the warning as known.
- **A Record that cannot be read is a failure and left out:** no bands, a band without whole
  `start`, `end` or `mappedValue`, or a band that ends before it starts.
- **Only identifiers and numbers are exported.** No text, description or media.

## Consequences

- A `resourceTypeGuids` entry equals a commodity's `subtypeGuid` only when the catalog has that
  commodity: 18 of the 43 links on LIVE do.
- `p4kx catalog` on LIVE exits with code 1 because of the warning, as it already does for the
  empty shop inventory file.
