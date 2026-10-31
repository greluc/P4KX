# A story character's own head has its face in its RigLogic file, its skeleton file and its geometry

Decided for issue #589. Evidence: `p4kx armour glb` over
`NPC_Archetypes-Male-Human-MissionGivers-MilesEckhart` of the LIVE Archive
(`crates/p4kx-assembly/tests/real_archive/real_npc_face.rs`), the GLB checked by the validator of the
testkit and imported by Blender 5.2 headless. Builds on ADR-0091 (expressions), ADR-0098 (face
animation) and ADR-0109 (story characters). The layouts below were read from the data and checked
against the DNA rig of the stock head; none comes from another project's source.

## The problem

A story character's head has its own skins (ADR-0109) but no DNA rig, so it was exported with a
neutral face: the joints and the morph targets of a face animation both came from the DNA rig. The
Archive holds three files per head that carry the same things in other places.

## What the files hold

**The `.RigLogic` file** is `Objects/Characters/RigLogic/<name>.RigLogic`, named like the head skin
without `_head.skin` (`eckhart_t2`). It has the layout of the `RigLogic` chunk of a head skin
(`RIG V1.9`, ADR-0091) and so reads with `RigLogic::read`: the 205 control names, the hashes, the
290 correctives and the 179 joints. Besides that it carries the delta records that move the joints:

- the header holds the scale of the main and of the secondary records as two `f64` at 0x48 and their
  counts as two `u32` at 0x58;
- the main records are the section of slot 3 of the start table, the secondary records the section
  of slot 4;
- a record is the record of a DNA rig (ADR-0098): the output (joint times three plus the axis), the
  number of entries and the entries, each a blendshape in the low 9 bits and a signed coefficient
  above; a step is the scale over 2^22 - 1.

The records of the `RigLogic` chunk of the stock male head skin are byte for byte the records of the
base head, variant 0, of its DNA rig, with the same scales (517 main and 528 secondary records,
`the_delta_records_of_a_head_skin_are_those_of_the_base_head_of_its_dna_rig`). The file of a head
without a DNA rig is therefore that head's own base: Eckhart's has 518 and 519 records and the main
scale 0.0736. All 74 `.RigLogic` files of heads in LIVE read and make a `FaceRig`; the three
`shared_wrinkle_mask_*` files in the folder are the wrinkle maps' rigs (ADR-0098) and are not heads.
The `RigLogic` chunk of Eckhart's own skin is another set (517 and 530 records); the file is the one
the issue and the folder name the head's rig by, and the chunk is not read for a bespoke head.

**The `BShapesGpu` chunk** (`0x57A38888`) of the head's `.skinm` holds the blendshapes of the head
as vertex morphs, stored for the GPU. Eight `u32` open it: the numbers of blendshapes (495),
vertices (5,584) and entries, then the offsets of four tables and of a pool of blendshape names,
which the reader does not use. The tables are:

- per vertex a `u32`, where its entries start;
- per blendshape 24 bytes, six `f32`: the scale of the codes on x, y and z, then their bias;
- per vertex 64 bytes, a bitmap with one little-endian bit per blendshape that moves the vertex;
- per entry three bytes, the codes on x, y and z.

The entries of a vertex are in ascending order of blendshape, so the code of blendshape `b` at a
vertex is the entry at the vertex's start plus the number of bits below `b` in its bitmap. The move
is `code * scale + bias` in metres, in the axes of the DNA rig's cells. The reader checks that every
table lies in the chunk and that the bitmap of every vertex counts the entries the starts give it.
Against the cells of the base head of the DNA rig of the stock male head (every seventh blendshape,
59 of them) the geometry holds 97.5 % of the moves (23,679 of 24,274) and differs by at most 2.5e-5
m on them, a step of the byte codes; the rest are moves the quantisation drops.

**The skeleton file** `<name>_head_skel.chr` has 201 bones to the skin's 198. The skin's bones are
all in it with the same names, parents and (below the root) controller IDs, but the bind poses of
all 183 facial bones differ, by up to 18.6 mm: the skin's `FACIAL_LOD1_C_Jaw` sits on the middle
plane (x of 4e-10 m), the file's 2.4 mm beside it. The skin holds the stock head's joints, the file
the character's own.

## What the export does

`Armour::build` gives a head part of a bespoke skin named `<prefix>_head.skin` the face of
`own_head`, when the Archive has the `.RigLogic` file; other heads are unchanged. Below LOD 0 only
the morph targets are made, carried over from level 0 (ADR-0293), and no joints:

- the joints of the rig are the bones of the skin whose controller ID is the CRC-32 listed in the
  file; the head's bones join the body skeleton by name as before (ADR-0098), but with the bind
  poses of the same-named bones of the skeleton file, so the joints sit where the character's face
  puts them; a head without a skeleton file keeps the poses of its skin, and so does one whose
  skeleton file lacks a facial bone of the skin or has it under another parent or with another
  controller ID, which is listed as `face_joints_failed` naming the bone;
- with `Expressions` the blendshapes of the geometry file are the morph targets, named by the file's
  controls and correctives like those of a CHF head (399 targets for Eckhart, 400 shape keys with
  the basis in Blender);
- `Armour::animate` makes the `FaceRig` of the file (`FaceRig::from_logic`) and evaluates a face
  animation exactly as for a CHF head: the control tracks of the clip name controls of the file by
  hash, the facial joints move by the poses of the rig, the head's targets take the weights.

The clip `pu_eckhart_mg_chosen_playeracceptmission_ig_002_face` of his own folder has 183 tracks, 172
of which name a control. It becomes 75 samples, 177 joints move and the farthest one by 16.3 mm; the
GLB is 7.3 MB, passes the checker and imports into Blender 5.2 with 403 bones, one action and the
shape key action.

`p4kx armour glb` and `p4kx character package` write the expressions of a `--loadout`'s head like
those of a CHF's: all at every level, `--expression NAME` for some, `--no-expressions` for none. A
failure is the issue `face_joints_failed` or `morphs_failed`, and the head is still written.

## What is not in it

- The order of the Euler angles and the side of the product (#627) are those of ADR-0098:
  `FaceRig::from_logic` shares them with the DNA path, so a decision on #627 changes both and
  nothing here makes it harder.
- The attenuation of the blendshapes (slot 17 of a DNA rig) is not applied, as for the CHF heads (ADR-0293).
- The hair, brows and lashes of a story character do not follow the head's expressions: they are
  not pieces that follow the head (ADR-0200).
- A head whose skin is not named `<prefix>_head.skin` and a head without a `.RigLogic` file keep the
  neutral face.

## Considered options

- Read the chunk of the skin instead of the file: the chunk of Eckhart's skin is not his own set of
  records, and the issue names the file.
- Offsets from the difference of the two skeletons, as the Character's joint offsets of a DNA rig:
  the skeleton file also differs in rotation, and the file's poses are the head's own, not a delta.
- Decode only the joints and leave the expressions to a DNA rig that the Archive does not ship: the
  morph targets are millimetres, but the head would have no expressions at all.

## Consequences

- Every story character with a `.RigLogic` file of its own and a `<prefix>_head.skin` has expressions
  and a face animation in `armour glb` and `character package`.
- ADR-0109 said neither file was read; both are now, and the `.RigLogic` file is what ADR-0109 called
  its follow-up.
