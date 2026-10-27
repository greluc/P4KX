# An unskinned worn item hangs on the bone of its port

Decided for issue #735. Evidence: the LIVE Archive, `Criminal_Gang_Guard_Heavy_Boss.xml` with the
backpack `doom_combat_medium_backpack_01`.

## What the data says

The core armour `slaver_armor_heavy_core_01_01_01` has the port `backpack`, a `Bone` attachment to
the helper `backpack_attach_1_override` with the item helper `backpack_attach_1_loc` and no
offset. The helper is a bone of the core's skin (below `Spine3`), not of the body skeleton. The
backpack model has no bone weights, so the Armour wrote it as a root node at the origin, between
the feet.

The bone has the pose of a Z-up skeleton, a turn of 180 degrees about the vertical. The item helper
`backpack_attach_1_loc` is stored with the axes of a Y-up helper: the same turn followed by 90
degrees about X. Placing the helper on the bone without that turn lays the backpack on its back;
with it, the backpack stands upright with a tilt of five degrees, its outer side behind the wearer.

## Decision

- A worn piece that sits on a port with a `Bone`, `Entity` or `StatObj` attachment records the
  port: the helper, the offset, the item helper, and the piece that holds the port, or the body.
  A port with a `Skin` attachment is unchanged (ADR-0224).
- A piece without a skinned mesh at the exported level is placed once the skeleton is complete:
  the helper is looked up among the bones of the holding piece (the body skeleton for the body);
  the piece hangs on the bone of the body skeleton with that name, else on its nearest ancestor
  that the skeleton has, as a rigid child of that bone node in the GLB, so it follows the bone.
- Its transform below that bone is the bone's bind pose to the helper's, the turn of 90 degrees
  about X from the frame of a bone to the frame of a helper, the offset of the port, and the
  inverse of the item helper.
- A helper that is no bone of its holder is listed as `helper_not_found` and the piece stays at the
  origin. A placed piece is no longer listed as `not_skinned`.
- Skinned pieces, and pieces that are not worn armour or clothing such as holstered weapons, are
  not changed; drawing those is the general bone-attachment work of #655.
