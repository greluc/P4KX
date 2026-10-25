### Removed

- `p4kx catalog --json`: the warning kind `reward_sources_truncated` no longer exists, because the
  reward sources are no longer capped (ADR-0154). `p4kx-export`: `read_crafting_limited` and
  `CraftingWarning::RewardSourcesTruncated` are removed.
