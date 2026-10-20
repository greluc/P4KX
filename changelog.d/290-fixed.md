### Fixed

- Loadout: an entry without an Entity (an XML `<Item>` with an empty `itemName`, or a DataForge
  manual entry without target) that carries nested entries no longer drops them. The port stays
  empty and the nested entries appear below it as `PortNode::nested` in `p4kx-assembly`. This
  changes the output of `p4kx entity loadout`: the text lists them indented below the empty port,
  the JSON gives every Port a `nested` array, and `ports`, `attached` and `empty` count them. On
  LIVE it affects the two human body Entities (`Body_ItemPort` with 10 nested items) and
  no ship or vehicle.
