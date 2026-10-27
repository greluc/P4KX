# Ship insurance is a field of the catalog ship, read from its own Record

Decided for issue #323. Builds on ADR-0002, ADR-0054 and ADR-0065.

## Decision

- **The insurance data is the field `insurance` of every entry of `ships`,** not a kind of its
  own. The issue names a `ShipInsuranceRecord` with an `allShips` list; the Archive has no such
  type any more (LIVE). The data sits in the ship's own `EntityClassDefinition`, in the
  `SEntityInsuranceProperties` of its `StaticEntityClassData`, so the ship's `recordPath` is the
  source path and no second Record, link or ordering is needed.
- **The object holds the raw facts:** `restoreOnClaim`, `shipEntityClassName`,
  `baseWaitTimeMinutes`, `mandatoryWaitTimeMinutes` and `expediteFee` (the Record's `baseExpeditingFee`), each `null` when the
  Record states none. The object is `null` when the Record has no `SEntityInsuranceProperties`.
  The Record states no claim cost, so none is exported (ADR-0065: facts, no derived values).
- **`displayParams` stay out:** they repeat the ship's name, role, career, crew size and
  manufacturer, mostly as placeholders, and are game text.
- **The fee stays `expediteFee`, not a claim fee.** For the Idris-M the Record says 150,990 aUEC
  while the game shows just under 1,000,000 for a claim, so the number is not shown to be the
  claim fee. The claim fee of an insured Loadout is not stored.
- **The recovery multipliers are the object `itemRecovery`,** not a kind: the
  `ItemRecoveryConfigurationParams` Record `entitlementpolicies/globalitemrecovery`, its
  `economyParams` under their data names, with the override groups and the GUIDs of their entity
  classes. The Basetool computes the fee itself; the catalog computes none.
- **It is part of the unreleased `schemaVersion` 2,** with no bump.

## Consequences

- On LIVE all 1,066 catalog ships carry `insurance` with a fee.
