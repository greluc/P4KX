### Added

- Basetool catalog: modifier ranges of a blueprint with a struct type the reader does not know are
  counted and reported as the crafting warning `unknown_modifier_ranges` (a `count` and the
  `details` types) in `p4kx catalog --json`, on stderr and in the catalog windows of the GUI, in
  German and British English. A Build without such a range has no warning.
