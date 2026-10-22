# Commodity refining in the DataForge

Result of the spike #397 against the LIVE Archive: what the DataForge holds of the raw to refined
relation, density, instability, counting in SCU or units, and refining methods and yields. Counts
are Records of LIVE 4.10.197; the `ResourceType` Records number 207 and the `ResourceTypeGroup` Records 19.
Only facts are used (no description, no media), as the legal review of 2026-10-20 allows.

| Attribute | Found | Where | Coverage on LIVE |
|-----------|:-----:|-------|------------------|
| Raw to refined relation | yes | `ResourceType.refinedVersion`, a Reference to the refined `ResourceType` | 30 of 207 `ResourceType` Records; the other 177 are empty (refined, unrefinable or unrelated) |
| Density | yes | `ResourceType.densityType` (`ResourceTypeDensity`) `.densityUnit` (`GramsPerCubicCentimeter`) `.gramsPerCubicCentimeter` | 207 of 207, from 0.25 to 20.45 g/cm3 |
| Instability | partly | `MineableElement.elementInstability`, linked to a `ResourceType` by `resourceType` | 46 `MineableElement` Records under `mining/mineableelements`, values 0 to 1000, 42 distinct `ResourceType`s |
| Counting in SCU or units | no flag | see below | not a field |
| Refining methods | yes, as speed and quality only | `RefiningProcess` Records under `refiningprocess` | 9 Records |
| Refining yields | no | none | none |

## Raw to refined

All 30 targets of `refinedVersion` resolve to a `ResourceType`; none refines into itself and no
target has a `refinedVersion` of its own, so the relation is one step. One refined `ResourceType`
is the target of three raw ones. The catalog
carries it as `refinedSubtypeGuid` on the commodity of the raw `ResourceType`; the refined
commodity is the one whose `subtypeGuid` equals it, when the Archive has such a commodity.

## Density

Every `ResourceType` states the density in one unit only, `GramsPerCubicCentimeter`; the catalog
carries it as `densityGramsPerCubicCentimetre`. The Wiki states other units for some commodities;
a Basetool conversion is its own decision.

## Instability

Only the `MineableElement` Record holds `elementInstability` (with resistance and the optimal
window), so it belongs to a mineable element, not to the commodity. 27 of the 46 elements link to
a `ResourceType` that has a `refinedVersion`. Several elements may share a `ResourceType`, which
makes a flat commodity field ambiguous; this stays out of the catalog until a rule picks one
(follow-up). One `ResourceType` carries `properties/ResourceTypeVolatility` (1 Record,
with a `healthDecayPerSecond`), a decay of cargo, not the instability of a rock. The mining laser
`laserInstability` modifiers (names matching `*nstab*`) are item stats, not commodity data.

## SCU or units

No `ResourceType` or commodity field says whether the commodity is counted in SCU or in pieces.
Two indirect signals exist and are not a flag, so they are not exported: `ResourceContainer.capacity`
is a `SStandardCargoUnit` (1,842 of 2,336 containers), `SMicroCargoUnit` (379) or `SCentiCargoUnit`
(115), and 195 `ResourceType` Records list carryable boxes per SCU size in `defaultCargoContainers`.

## Refining methods and yields

`RefiningProcess` Records have `refiningSpeed` (`Slow`, `Normal`, `Fast`), `refiningQuality`
(`Wasteful`, `Normal`, `Careful`) and `processName`, a localisation key. Nine of them exist, one
for every speed and quality pair. They identify a method by GUID and record path. The DataForge
holds no method names of the UEX kind, no yield per commodity and method, and no price: the only
number near it is a `refiningQualityUnitMultiplier` on one Record. Refinery yields stay with UEX.

## Follow-up

- Catalog fields added with this spike: `densityGramsPerCubicCentimetre`, `refinedSubtypeGuid`.
- Open: a `refiningProcesses` kind (9 Records) and an instability field need a rule for the
  `MineableElement` to commodity relation; a new issue tracks them.
