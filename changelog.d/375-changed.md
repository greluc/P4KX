### Changed

- Loadout: ports that only the vehicle implementation declares are listed in the tree with the
  new `source` value `empty`, so `--replace` can attach an Entity to them. A script that checks
  `source` must accept new values.
