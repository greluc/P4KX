### Changed

- Basetool catalog: the reward sources of a blueprint pool come from one scan of every Record
  instead of the reference index (ADR-0154). Nothing is capped any more. The warning
  `UnreadableReferenceRecords` became `UnreadableRewardRecords`, which counts only unreadable
  Records of a type that holds a reward source elsewhere, or every unreadable Record when no source
  is found; an unrelated unreadable Record no longer makes `p4kx catalog` exit 1. In
  `p4kx catalog --json` this is a breaking change of `warnings`: the kind
  `unreadable_reference_records` is now `unreadable_reward_records`.
- `p4kx-export`: `CatalogProgress` has a `phase` (`CatalogPhase::Crafting` or `Records`), and
  `CraftingWarning` has `kind()`, `count()` and `details()`. `p4kx-dataforge`: the
  `ReferenceIndex` constructors with a limit or a stop are replaced by
  `build_with(forge, limit, stop)`.
- Basetool catalog: a gameplay property whose name is a placeholder key keeps the key in `name`,
  like items and blueprints. None of the 29 properties of LIVE has one, so no catalog of LIVE
  changes.
