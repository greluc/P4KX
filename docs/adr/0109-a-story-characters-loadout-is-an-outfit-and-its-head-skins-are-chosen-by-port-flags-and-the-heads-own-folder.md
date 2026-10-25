# A story character's Loadout is an outfit and its head skins are chosen by port flags and the head's own folder

Decided for issue #577. Evidence: `p4kx armour glb` over `NPC_Archetypes-Male-Human-MissionGivers-MilesEckhart`
of the LIVE Archive (`crates/p4kx-assembly/tests/real_archive/real_npc.rs`), rendered from the GLB
at the front and at three quarters. Builds on ADR-0079 and ADR-0082 (armour from a Loadout),
ADR-0084 (CHF heads), ADR-0090 (look of hair, eyes and skin), ADR-0098 (animations) and ADR-0107
(Character package).

## How the Loadouts are found

The Records name them, so no fallback by the Character's name is needed. The Entity's
`SEntityComponentDefaultLoadoutParams.loadout` is not a manual or XML Loadout but an
`SItemPortLoadoutOutfitParams`: a `BaseLoadout` (the XML Loadout of body, head and hair,
`Scripts/Loadouts/Character/Human/Body/PUCast/Nyx/PU_Eckhart_T2_Body.xml`) and `Outfits`, each
with `OutfitNameTags` and `Loadouts` (the clothing,
`…/Clothing/PUCast/Nyx/pu_eckhart_t2_mission_givers.xml`). The `Character` Record
(`libs/foundry/records/character/pu/mileseckhart`) holds a gender, a name and a mannequin tag, and
the Actor component of the Entity points at it; neither names a Loadout. The resolver read only
manual and XML Loadouts, so the Loadout of such an Entity was empty.

An outfit is read as `LoadoutSpec::Layered`: the base Loadout, then the Loadouts of the outfits
without name tags, or, when every outfit has name tags, of the first outfit. Layers are laid over
each other per port; an entry that names no Entity and has entries of its own (the clothing
Loadout's `Body_ItemPort`) adds them to the nested entries of the earlier entry of that port
instead of replacing it, so the body's head and hair survive the clothing. The report lists the
XML Loadout files in the order they are laid over each other (`loadout.files`).

## The head

`LoadoutResolver::npc_head` finds the Entity of item type `Char_Head` wherever it hangs in the
resolved tree. Its sub-type (`Male`, `Female`) gives the body variant, so `--body` is no longer
needed. Its pieces are the head and the Entities on the head's own ports that have geometry:
teeth, brows, lashes, eyes and hair.

**Which sub-geometry an item takes.** The head Entity defines its ports with `Flags`, e.g.
`Eyes_ItemPort` with `Flags="eckhart_t2_head"`. The brows, lashes, eyes and teeth Entities have
`SubGeometry` nodes tagged with such a flag, one per story character. An item takes the
sub-geometry tagged with a flag of the port it sits at (`Piece::selected_by`), else the one for
the body variant as before. The `PortFlags` of the head's port container apply to all its ports
beside the `Flags` of the port: the brow port of `hurston_t3_head` has no flags of its own, but the
container has `hurston_t3_head`, and without it the brow would take the default sub-geometry,
which is Eckhart's.

**Which skin the head is.** No Record links the head Entity to its bespoke skin. The Item
`eckhart_t2_head` has `DeformerType="Protos"`, names the generic Protos head skin as its geometry
and the material library `…/heads/male/pu/eckhart/eckhart_t2_head_material.mtl`; the eyes
sub-geometry for the flag also names the generic Protos eyes skin (`DeformerType="ProtosMisc"`),
while the brows and lashes name `eckhart_t2_eyebrow.skin` and `…_eyelashes.skin`. The rule this
tool applies is the naming convention the Archive follows for the parts of Eckhart's head: the folder
of the head's material library holds the character's own skins named `<prefix>_<part>.skin`, where
the prefix is the head's class name without `_head` (`eckhart_t2`) and the part is what the
generic file's name adds to its rig's name (`…_pu_head.skin` is `head`, `…_pu_eyes.skin` is
`eyes`, `…_pu_teeth.skin` is `teeth`). The skin is used when the Archive holds it and the generic
one stays otherwise (`Piece::bespoke_in`). The real-Archive test
`no_story_character_head_takes_the_skin_of_another_character` applies the rule to the 16 mission
givers of the LIVE Archive that have a head with a folder of its own (Eckhart, Hurston, Macken,
Regen Boss, Klim, Parr, Darneely and others) and checks that each head is its own skin or the
generic one, and that Hurston's brows are his. The head's bones join the body skeleton by name like
those of a CHF head, at the positions its skin gives. When the material library a Record names for
such a part does not exist in the Archive, the part takes the material library of the head, which
has a sub-material per part (`teeth`, `eyeInner`, …), and the export reports the missing library
as a `MaterialLibrary` issue that does not skip the part.

**Skeleton and RigLogic.** The skin carries its bone table, bind poses and a `RigLogic` chunk, and
the Archive holds `eckhart_t2_head_skel.chr` and `Objects/Characters/RigLogic/eckhart_t2.RigLogic`
beside it. Whether they hold the same data was not compared. The bones come from the skin as for
every head; neither the `.chr` nor the `.RigLogic` file is read. Reading `.RigLogic` is the
follow-up #589. The expression morphs and face poses of ADR-0091
and ADR-0098 need the DNA rig (`.dna`) of the head. The Archive ships none for Eckhart's head, so
a bespoke head has a neutral face only and no face animation: bones of the body animations move
it, the rest of the face stays as modelled.

## Hair

The hair Entity `hair_02` has a material tree (`SMaterialNodeParams`) with a node per hair colour
tag, `hairVar_Brown`, and below it the body tag, `Male`, holding the library `m_hair_02_brown.mtl`.
The Loadout puts the Entity `Hair_Var_Brown` (`Char_Hair_Color`, `AttachDef.Tags` containing
`$$hairVar_Brown++`) on the hair's `Material_Variant` port. A word of a tag list stands for the
tag between its `$` and `+` marks. The hair piece takes the tags of the Entities on its own ports
plus the tag of the body variant, and the library of the deepest chain of nodes whose tags are all
among them (`MaterialVariant::resolve`).

## The eyes

The eye library `eckhart_t2_eyes_material.mtl` has four sub-materials: `eyeInner` (shader `Eye`,
the iris and sclera texture), `eyeWet` (`Eye`, `EYE_SPECULAR_OVERLAY`, no texture, white) and
`eyeAO` (`Eye`, `EYE_AO_OVERLAY`, black with an ambient occlusion map), and the lashes. The
iris mapping and the pupil are correct: drawing `eyeInner` alone shows the brown iris on the
veined sclera. The red surface with a tiny pupil was `eyeAO` drawn as an opaque shell over the
eyeball, and `eyeWet` drew white crescents. Both are shells the engine blends in as occlusion and
highlight; no plain glTF material says that. `Material::hidden` is therefore true for a material
of the `Eye` shader whose generation mask has one of the two overlay words, and the GLB writers
leave its submeshes out (reported with the other hidden submeshes). The ADR-0090 look (iris colour
of a CHF) is unchanged.

## Textures and animations

`armour glb` takes `--materials colours|textures` (default `colours`, as before) and
`--texture-mip`, like `chf glb`; `character package` has them already. `--anim` looks in the
folders of the body variant, which include `Animations/pu/<body>/<npc>/body`. For a Loadout with a
bespoke head the folder `Animations/pu/<body>/<name>`, where `<name>` is the name of the head's
own folder up to its first underscore, e.g. `…/eckhart`, is searched first when it exists, and a name or `all` that folder lacks falls back to
the folders of the body, so `--anim all` writes the character's own animations only. `--anim-folder`
(repeatable) names the folders to search instead and never falls back. `--chf` beside a `--loadout`
in `character package` keeps the CHF's head and drops the Loadout's.

## Consequences

- Every Entity whose default Loadout is an outfit resolves with its clothing in `entity` trees too.
- Bespoke heads have no expressions while the Archive ships no DNA rig for them.
- The eye overlays are not drawn by any export or render.
