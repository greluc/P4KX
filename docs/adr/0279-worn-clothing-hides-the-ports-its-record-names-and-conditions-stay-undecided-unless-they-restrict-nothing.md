# Worn clothing hides the ports its Record names, and conditions stay undecided unless they restrict nothing

Decided for issue #638. Evidence: the Record XML of every Entity below `entities/scitem/characters`
of LIVE 4.10 (5017 Records, 4424 with `HiddenParts`), and the real-Archive tests in
`crates/p4kx-assembly/tests/real_archive/real_clothing_hidden.rs`. Completes the parts ADR-0140
left open.

## HiddenParts

`SCItemClothingParams.HiddenParts` is an array of `SCItemClothingHiddenPartsParams`, each with a
`PortName`. The names are the ports of the Loadout, not zones of the body: `Hat_ItemPort`,
`Hair_ItemPort`, `Beard_ItemPort`, `Facewear`, `Clothing_Torso_0`, `Clothing_Torso_1`,
`Clothing_Legs`, `Clothing_Hands`, `Clothing_Feet`, `weapon_sidearm_attach`. Typical lists on LIVE:

| Item type | Hides |
|-----------|-------|
| `Char_Clothing_Hat` | `Hair_ItemPort`, sometimes `Beard_ItemPort`, `Eye_Accessories_ItemPort` |
| `Char_Armor_Helmet` | `Facewear`, `Hat_ItemPort`, `hair_flair_ItemPort`, and for a closed helmet the hair, beard, brows, lashes, eyes and teeth |
| `Char_Armor_Undersuit` | `Clothing_Torso_0`, `Clothing_Torso_1`, `Clothing_Legs`, `Clothing_Hands`, `Clothing_Feet` |
| `Char_Clothing_Torso_1` (jacket) | `Clothing_Torso_0`, often `weapon_sidearm_attach` |
| `Char_Armor_Arms`, `Char_Armor_Legs` | `Clothing_Hands`; `Clothing_Legs`, `Clothing_Feet` |

**Decision.** A [`Piece`](../../CONTEXT.md) knows the port it sits at (`Piece::at_port`, set by the
Loadout, by the head of a story character and by a dressed head). A worn piece whose Record names
that port in `HiddenParts`, compared ignoring ASCII case, is left out of the Armour and reported as
`part_hidden` with the port and the piece that hides it. The rules:

- Every piece hides by its Record, whether it is hidden itself or not: the result does not depend
  on the order of the pieces.
- A piece never hides itself.
- A hidden piece contributes no model, no materials and no zones: its chunks cover nothing of the
  body, so the body shows where a hidden shirt covered it.
- A piece without a port (a piece given with `--piece`, the head, the teeth) is never hidden. The
  head and the teeth are the base of the face rig and the skeleton; `Head_ItemPort` and
  `Teeth_ItemPort` in a closed helmet's list therefore hide nothing.
- A piece mounted on the port of a hidden holder stays and is reported as `helper_not_found`, as for
  any holder that is not worn.

This is also the answer to garments under garments at the level of whole garments: the game hides
the shirt under a jacket and the clothing under an undersuit by these lists.

## Garments under garments, zone by zone

Not possible from the data, as in ADR-0140: the skin of a garment carries one submesh per zone but
no zone name, and no file holds the table of its node numbers; only the static model of a garment
(`*_prop.cgf`) names its nodes. The body's table was found by comparing two skins of one body type
(ADR-0140); no second skin per garment exists to compare. A garment covered only partly by another
is therefore drawn whole.

## VisibilityConditions

Every condition on LIVE is an `SCItemClothingVisibilityConditionParams` holding
`PortVisibility`: entries of `SCItemClothingPortVisibilityConditionParams` with `Ports`,
`DisableTags`, `SkipTags` and `ActiveTags`. Of 281 entries, 48 name no port and no tag, 33 name one
port only, 7 one port and `DisableTags`, and 193 name ports and `ActiveTags`; none names a
`SkipTag`. Which chunks carry conditions:

- `theta_*_zone` and `omega_*_zone` (the zones of an armour's own model, 207 chunks), the pouch
  zones of one core, and
- `hips_zone` (19 chunks), the only body zone with a condition; it has no geometry in either body.

**Decision.**

- A condition that names no port and no tag restricts nothing; such a chunk is not conditional and
  covers its zone like any other chunk.
- A condition that names a port or a tag depends on the items in ports and on tags of those items.
  The data does not state how the tags are matched (any or all, against which tags of the item) or
  what an empty `Ports` or `ActiveTags` means beside non-empty ones; it is evaluated by the game.
  Such a chunk stays non-covering, as in ADR-0140, and when it names a zone of the body's table the
  Armour reports `clothing_conditions_undecided` with the zones, which stay whole.
- On LIVE no conditional chunk names a zone of the body's table, so the report does not appear and
  no export changes; it is there for a patch that adds one.

## Consequences

- `armour glb`, the Blender package and the GUI leave hidden pieces out, with `part_hidden` in the
  report; the Eckhart cut of ADR-0140 is unchanged (his jacket hides no port he wears).
- The entity assembly (ADR-0228) does not apply `HiddenParts`: its tree has no ports of a body.
  A follow-up is proposed for it.
- `Wearable::hidden_ports` and `Piece::hidden_ports` give the lower-case port names;
  `IssueKind::PartHidden` and `IssueKind::ClothingConditionsUndecided` are new, with the codes
  `part_hidden` and `clothing_conditions_undecided`.
