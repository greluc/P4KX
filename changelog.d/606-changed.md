### Changed

- `p4kx-export`: `CraftingWarning` is no longer `Copy`, and `UnreadableVehicleFiles` and
  `UnreadableShopInventories` are struct variants with `count` and `details` instead of tuple
  variants, which breaks a `match` on them.
