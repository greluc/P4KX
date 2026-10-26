# Armour takes the tint palette of each worn piece

Decided for issue #732. It extends ADR-0080 (palettes of parts) to Armour and follows the rule of
ADR-0195: a part keeps its own palette and inherits none.

## What the data says

Checked on the LIVE Archive, imported into Blender and compared with the in-game item render: the
Defiance Core Tactical (`slaver_armor_heavy_core_01_01_01`), worn by the Loadout
`Scripts/Loadouts/Character/Human/Armor/NPC/Criminal/Pyro/Criminal_Gang_Guard_Heavy_Boss.xml`, was
drawn with white plates; in the game it is dark gunmetal.

- `m_slaver_heavy_armor_core_01_01_01.mtl`, material `core_plate_m`, `BaseLayer3`:
  `TintColor="1,1,1" PaletteTint="1"`, which takes the primary entry of the tint palette.
- The item Record names the palette on its per-body sub-geometry (`Male`, `Female`), not on its main
  geometry. The default palette of the material library names no Record.
- `Armour::build` resolved no palette; only `Assembly::build_with_palettes` did, for ship parts, so
  the layer kept its white `TintColor`.

## Decision

`Armour::build_with_palettes` draws each piece with the palette its worn geometry names, the
sub-geometry chosen for the body variant, else the default palette of its material library, as an
Assembly does for its parts. The palette entries are applied to the materials of the piece's
library first, and the choices of its Character (`Look`) after, so a choice always wins. A piece
never takes the palette of another piece, of the body or of anything else it is worn with; each
body variant takes the palette of its own sub-geometry.

A palette is only resolved for a piece whose library has a material that takes a palette slot, so
body skins, heads and hair raise no palette issue. A palette that cannot be resolved is listed as a
`palette` issue, once per message, and the piece is drawn without it. `Armour::build` resolves no
palette and is unchanged.

`armour glb`, `character package`, `chf glb` and the Characters tab use `build_with_palettes`.

## Consequences

Armour whose layers take a palette entry (`PaletteTint`) is drawn in the colours of its palette in
the GLB and in the Blender package. Armour whose layers take none, such as the CDS legacy medium
armour, is unchanged.
