### Changed

- Interiors: with the DataForge, every model and prop without a palette of its own is drawn with
  the default tint palette of its material library, as the parts of an Assembly are, so existing
  Interior exports and the 3D view change their colours. Interior lights are brighter, because the
  stored colour is linear and is no longer converted a second time.
- Library: `SocEntities::other` no longer counts the entities that name a class Record; they are
  listed in the new `SocEntities::classed`.
