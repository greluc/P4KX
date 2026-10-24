# Armour is skinned to the body skeleton by bone name

Decided for issue #149. Evidence: `p4kx armour glb` over `rsi_odyssey_undersuit_01_01_01` and
`rsi_odyssey_undersuit_helmet_01_01_01` of LIVE on both bodies
(`crates/p4kx-assembly/tests/real_archive/real_armour.rs`): one skin of 220 bones, the skins of the
body and of the pieces on it, no part skipped, the GLB passing the checker of the testkit.

## The body

A **Body variant** is a skeleton file and a default body skin: `male_v7/export/bhm_skeleton_v7.chr`
with `male_v7/body/m_body.skin`, `female_v2/export/bhf_skeleton_v2.chr` with
`female_v2/body/f_body.skin`. The character definitions next to the body (`m_body.cdf`) name skins
that are no longer in the Archive, and the body Entities (`m_body_01`, `f_body_01`) differ only in
their material variants, so the files are the variant's constants and the skeleton is the only
file whose absence stops the export (`AssemblyError::Body`).

## The geometry of a piece

An Entity's Record names its main geometry and sub-geometry per tag (`SubGeometry` of the
`SGeometryNodeParams`). Armour and clothing give the skin to wear as the sub-geometry tagged `Male`
and the one tagged `Female`; the main geometry is a crate or a prop. A piece is worn with the
sub-geometry that carries the body variant's tag (ignoring case); sub-geometry for other tags
(`heldEntity`, `inventoryStoredEntity`) is never worn. A Record that names sub-geometry for a body
but not for this one cannot be fitted and is skipped; a Record with no sub-geometry for any body
is worn with its main geometry, if it has one.

## Skinning

The skins of the pieces carry their own `CompiledBones2`, a subset of the body skeleton plus cloth
and helper bones (34 of the undersuit's bones are in none of the body's), and their bone maps
index that list. The GLB writes the body's bones once, one glTF skin over them with the inverse
bind matrices of the body skeleton, and every skinned mesh uses it. The bone indices of each
piece are matched to the skeleton by name, ignoring case (`BoneMatch`); a bone the skeleton lacks
takes its nearest ancestor in the piece's skeleton that it has, so the vertices follow the
nearest rigid parent instead of collapsing to the root, and the piece gets a `bones_not_in_skeleton`
note. Weights of bones that meet are added up, so no vertex lists a joint twice.

`p4kx-gltf` writes this as `write_rigged_glb` with `RigPiece`s; it reuses the layout, materials
and node writers of the other exports, and the plain colours of the scene writer. The pieces of
later work, such as the pieces of a Loadout and the head of a CHF, are `Piece`s as well
(`Piece::file`), so they join the same Armour.
