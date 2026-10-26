# Leftover-like ships are not separated by their references and stay `player`

Decided for issue #423. Builds on ADR-0002 and the playability rules of `docs/basetool-catalog.md`.

## Decision

- **No structural rule separates the leftover-like ships.** About 20 ships on LIVE (`temp`,
  `display` and `hijacked` variants such as `GLSN_Shiv_TEMP_CosA`, `RSI_Meteor_TEMP_LaserRepeater`,
  `AEGS_Avenger_Stalker_Display`, `ORIG_890Jump_Hijacked`) read as `player`, and they keep doing
  so. A rule that moves them to another value waits for an owner decision on a name-based rule.
- **Reverse References do not tell them apart.** The Reference index of the DataForge was read for
  the 323 `player` ships of LIVE: the leftover-like ships have no referrer, but 162 of the 323
  have none, among them `AEGS_Avenger_Titan`, `AEGS_Sabre` and `ORIG_400i`.
- **The Records give no other marker.** `AEGS_Reclaimer_BIS2024_Temp` is a copy of its base hull:
  its Record differs from `AEGS_Reclaimer` only in GUID, class name and the links to itself, so
  Tags, components and the `vehicleDefinition` equal the base, and every `player` ship lies in
  `entities/spaceships/`. 33 `player` ships share name and description keys with their base
  hull, and event ships (`BIS2950`, `Plat`, `Fleetweek`) are among them, so that is no marker
  either.
- **The Basetool already treats `player` as candidates** (`docs/basetool-catalog.md`), so the
  leftover-like ships cost it a review, not wrong data.

## Consequences

- The catalog is unchanged; the known limit stays documented with this evidence.
- A name-based exclusion (more tokens in `EXCLUDED_TOKENS`) needs the owner's decision first.
