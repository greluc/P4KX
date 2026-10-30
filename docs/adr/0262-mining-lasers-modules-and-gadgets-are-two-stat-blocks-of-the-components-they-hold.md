# Mining lasers, modules and gadgets are two stat blocks of the components they hold

Decided for issue #895. Builds on ADR-0054, ADR-0065 and ADR-0072.

The catalog covered a mining laser only through `stats.weapon.fireActions[].damagePerSecond`. A
miner compares lasers, modules and gadgets by their throttle, their beam ranges, their charges and
lifetime and the modifiers they apply to the laser or the rock. All of it is in the Archive, in
components the stats did not read.

## Decision

- **`miningLaser`** mirrors `SEntityComponentMiningLaserParams`: `throttleLerpSpeed`,
  `throttleMinimum`, `usesPowerThrottle`, the `MiningLaserGlobalParams` Record as
  `globalParamsGuid`, the `miningLaserModifiers` and the `filterParams.filterModifier`. Ship lasers
  and the FPS multi-tool carry the same component, so one block serves both.
- **`miningModifier`** mirrors the `EntityComponentAttachableModifierParams` of an attachment that
  holds a mining modifier: `activationMethod`, `charges`, `canInterrupt`, `isInterruptible` and, per
  modifier struct, the first `ItemMiningModifierParams` (`laser`, the change to the laser, which
  modules carry), the first `ItemMineableRockModifierParams` (`rock`, the change to the rock, which
  gadgets carry) and the first `MiningFilterItemModifierParams` (`filter`). Each carries the
  `modifierLifetime.lifetime`; `laser` and `rock` the `MiningLaserModifier` values. The
  `ItemWeaponModifiersParams` entries become `fireActionModifiers` with `fireActionIndex`,
  `lifetime` and the `damageMultiplier` of their `weaponStats`.
- **Modifier values are the stored percent.** Each value is the `value` of a
  `FloatModifierMultiplicative`; an unset modifier is `null`, not 0. A consumer that wants "no
  change" as 0 maps `null` itself.
- **The beams stay fire modes.** A fire mode gains `hitType`, `fullDamageRange` and
  `zeroDamageRange` (`null` for a mode that states none), next to the `damagePerSecond` of
  ADR-0072. The mining beam and the extraction beam of a laser are two fire modes told apart by
  `hitType`; `fireActionIndex` of a module's multiplier indexes the fire modes as stored.
- **No derived values.** No total modifier of a laser with modules, no effective damage and no
  "active" versus "passive" flag: a module is passive when `activationMethod` is
  `ActivateOnAttach` (ADR-0065).
- **Archive data only.** Every value comes from the Archive; no value of a third-party source
  enters the repository, the tests or CI.
- **Additive in version 2.** Two blocks and three fire-mode fields are new fields (ADR-0054). Items
  are grouped into variants by their stats; on LIVE 4.10.2 none of the 832 variant groups (4,828
  items) changes.
