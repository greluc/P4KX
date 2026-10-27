# A relief decal is hidden only when its diffuse map is grey, and the add-on does not repeat the rule

Decided for issue #774. It refines ADR-0222 (a material's own detail map is no diffuse map, and
white relief decals are hidden). Evidence: the LIVE Archive, every `.mtl` file below
`Objects/Spaceships` (all ships, including the Carrack, Idris, Cutlass Black and 890 Jump), and
the Blender packages of those four ships and of the Hammerhead.

## What the data says

- ADR-0222 decided by the material: a `MeshDecal` with `PARALLAX_OCCLUSION_MAPPING`, a
  displacement map, a white diffuse colour and `ETintModeType` 0 or missing. It noted that a
  white, untinted parallax decal whose map holds colour would be hidden too.
- The material cannot tell them apart. `Decal_Buttons` (switch buttons), `Damage_Impacts`, the
  Vanduul `Decal_POM_A` and `decals_pom` of the medium structure, the BEHR `trim` and the Grin
  `Glow_Headlights` have the same shader, mask, parameters and white colour as the grey
  `pom_decals`, and each is drawn from a diffuse map with colour.
- The mean saturation of the diffuse maps of the candidates (alpha-weighted, at mip 3) falls in two
  groups. Thirty-eight maps lie between 0 and 0.032 (`misc_line_decals_diff` 0.0001, `rsi_pom_decals_diff`
  0.015, `crus_pom_diff` 0.025, `anvl_pom_sheet_01_diff` 0.032, the white stitching
  `trim_upholstery_1_diff` 0.018). Seven lie from 0.077 up (`grin_headlight_glow_diff` 0.077,
  `lrg_pom_dcal_01_diff` 0.092, `anvl_switches_buttons_diff` 0.096, `med_structure_pom_dcal_diff` 0.143, `behr_trim_1_a_diff`
  0.152, `gen_pom_metal_dmg_atlas_01_diff` 0.153, `aegs_switches_buttons_diff` 0.155).
- `p4kx entity package` leaves a material that draws nothing out of the package: the GLB writer
  drops its submeshes and `drawn_materials` never lists it. The add-on therefore never met a
  relief decal the Rust rule hides, and its copy of the rule (`_relief_decal`) could not fire.

## Decision

- **The gate.** A relief-decal candidate is hidden only while its diffuse map (`TexSlot1`) is grey:
  its mean saturation, weighted by alpha, is at most 0.05 (`RgbaImage::mean_saturation`). The gap
  between the two groups is the largest in the data. `Material::is_relief_decal` is the candidate
  test and `Material::diffuse_map_is_grey` is the verdict.
- **Where it is judged.** At load time, in `MaterialStatus::load`: `CompanionResolver` gained
  `texture_is_grey`, which the Archive resolver answers from mip 3 of the DDS (the last mip of a
  smaller texture). The verdict is stored on each candidate, so the GLB writer, the Blender package,
  the viewer and the armour path all see the same material. A resolver that cannot tell (`None`: the
  default, the disk resolver, a test closure) keeps the old behaviour, the candidate is hidden. A
  texture that cannot be read counts as unknown.
- **One rule.** The add-on's copy of the rule, `_relief_decal` and `WHITE_FROM`, is removed. The
  package carries the decision by leaving the hidden material out, which is the same mechanism as
  for any other material that draws nothing. The package format does not change.
- **Suffix helper.** `texture_suffix_lowercase` in `p4kx-model` replaces the two wrappers in
  `p4kx-blender` and `p4kx-gltf`. `texture_suffix` skips only tails of plain digits (`_01`, not
  `_01a` or `_v2`).

## Consequences

- Library loads read one small mip per candidate material. A texture is decoded again for each
  library that names it; the real-Archive test judges every ship material file of the LIVE
  Archive in under two seconds.
- Eight kinds of candidate in the ship files are now drawn: `Damage_Impacts`, `Decal_Buttons` (two files),
  `Decal_POM_A` and `decals_pom` of the medium structure, `Glow_Headlights` of the Grin, `buttons`
  of the Gladius, `pom_decal_b` of the large structure and `trim` of BEHR.
- Packages written before this change (format 1.3 and earlier from builds with ADR-0222) still
  contain no hidden material, so the add-on needs no rule for them either. Packages from builds
  before ADR-0222 show the white contour lines again; writing them anew fixes that.
- The 0.05 limit is a measured gap, not a derived value. A new decal atlas between 0.032 and 0.077
  would need a look, and `cargo test -p p4kx --test real_archive -- --ignored relief` lists every
  candidate with its verdict.
