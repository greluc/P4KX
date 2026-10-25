### Changed

- Basetool catalog: the reward sources of a blueprint pool come from one scan of every Record
  instead of the reference index (ADR-0154). Nothing is capped any more, so the warning
  `RewardSourcesTruncated` is gone, and `UnreadableReferenceRecords` became
  `UnreadableRewardRecords` (`unreadable_reward_records` in `p4kx catalog --json`), which counts
  only unreadable Records of a type that holds a reward source elsewhere. An unrelated unreadable
  Record no longer makes `p4kx catalog` exit 1.
- `p4kx-export`: `read_crafting_limited` is removed. `CatalogProgress` has a `phase`
  (`CatalogPhase::Crafting` or `Records`), and `CraftingWarning` has `kind()`, `count()` and
  `details()`. `p4kx-dataforge`: `ReferenceIndex::build_limited`, `build_cancellable` and
  `build_limited_cancellable` are replaced by `build_with(forge, limit, stop)`.
- Basetool catalog: a gameplay property whose name is a placeholder key keeps the key in `name`,
  like every other kind.
