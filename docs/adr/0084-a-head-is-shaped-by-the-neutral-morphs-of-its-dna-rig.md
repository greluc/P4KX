# A head is shaped by the neutral morphs of its DNA rig

Decided for issue #151. Evidence: `p4kx chf glb` over the default characters the Archive ships
(`masculinedefault.chf`, `femininedefault.chf`) and the mission-giver CHF
`Data\Libs\CharacterCustomizer\PU\MissionGivers\Male\Macken.chf` shipped in the LIVE Archive, at
LOD 0 and 2: smooth, distinct human heads on the matching body, no part skipped, the GLB passing
the checker of the testkit (`crates/p4kx-assembly/tests/real_archive/real_head.rs`, which reads
the two defaults). No player's file was used. The file layout of the rigs is
format knowledge from the MIT-licensed StarBreaker project, checked against the Archive.

## The rig

A head rig is the `.dna` file beside the head skin (`<name>_head.dna` for LOD 0,
`<name>_head_lod<n>.dna` from LOD 1 on; eyes and teeth have their own). Its header names the rig
(the CRC-32C of the rig's name, which is the `gender_hash` of the DNA of every Character made for
it), the number of heads (61 male, 48 female; head 0 is the base head the skin is) and the vertex
count, in the low half of the `u32` at offset 0x30. A table of 32 offsets follows; a slot ends
where the next one starts. The reader (`DnaRig`) knows the names of the heads, the joint offsets
per head, the neutral and expression morphs, the joint and region weights and the delta records.

## The neutral face

The neutral morphs hold one control per head other than the base head and covered region, per
vertex a bit mask of the controls that move it, a cursor into the code stream and three bytes per
set control, dequantised by the control's scale and bias. A Character's DNA gives per face part
up to four blends of heads; the weights of the heads in a region are the blend values added up
(`value / 65535`, capped at 1), normalised to sum to 1 and pruned to the four strongest (`DnaMatrix`).
A vertex moves by the sum over its set controls of the weight of the control's head in the control's
region times the dequantised code. The region masks of the rig are not applied: the codes already
are per region. A region without blends leaves the base head.

The stored per-vertex data is indexed by the vertex of the geometry file's stream, but a model's
mesh keeps only the vertices its triangles use, renumbered by first use. `Mesh::source` keeps the
stream index of each vertex, so the moves are matched exactly.

## What the export does

`Piece::shaped(dna)` shapes any skin piece; `Head::from_character` makes the head piece and the
body variant from the DNA's `gender_hash`. `Armour::build` reads the rig through
`ModelProvider::file`, checks that the rig is the DNA's, that it covers exactly the vertices of the
skin (which has one mesh), that every move is finite and that the DNA names heads the rig has, and moves the vertices of the model's level of detail; the
normals take over the change of the surface normal at their vertex, so authored seams stay. A head
that cannot be shaped is left out and listed as `shape_failed`: an unshaped head would show the
base face for a character that has another. `p4kx chf glb` still writes the GLB, then exits with
code 1.

The head is skinned like armour (ADR-0079). The body skeleton has no facial joints, so the weights
on the 183 facial bones follow their nearest body ancestor; in a static export that changes
nothing. Moving the joints to the Character's own positions (`DnaRig::neutral_joint_offsets`, the
blend of the per-head joint offsets by the joint weights) and the facial morph targets and
animation need the facial skeleton and belong to the expression work (#153).

## Considered options

- Apply the morphs as glTF morph targets: the neutral face is the Character's rest shape, not an
  expression, and viewers that ignore weights would show the base head.
- Decode the vertex streams of the rig's own head skins (the archetype heads) and blend the
  meshes: needs 60 further skins, and the rig already holds the blend inputs.
- Leave an unshaped head in the GLB when the rig fails: silently shows the wrong face.
