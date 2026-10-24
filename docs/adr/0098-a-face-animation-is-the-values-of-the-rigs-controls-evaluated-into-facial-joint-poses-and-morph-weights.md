# A face animation is the values of the rig's controls, evaluated into facial joint poses and morph weights

Decided for issue #154, which also does the facial-joint part of #533. Evidence: `p4kx chf glb` over
`masculinedefault.chf` and `femininedefault.chf` of LIVE, with the animations
`facerom_t0_face`, `ai_civ_neutral_idle_08_face` and `nw_neutral_stand_getupback`
(`crates/p4kx-assembly/tests/real_archive/real_head_animation.rs`): all 141 facial animations of
`Animations/Characters/Human/heads/male` evaluate without an issue, and the GLBs pass the checker
of the testkit. The layouts of the rig logic and of the delta records are format knowledge from the
MIT-licensed StarBreaker project, checked against the Archive.

## What a face animation is

A face `.caf` has one track per control of the head rig. The track's controller ID is the CRC-32
(IEEE) of the control's name as the `RigLogic` chunk of the head skin spells it, e.g.
`CTL_expressions blinkL`; 172 of the 183 tracks of an idle clip and 87 of 88 of a range-of-motion
clip name a control. The control's value, 0 to 1, is the first component of the track's translation
keys; the rotation keys are a constant. The frames are 30 per second. The reader of ADR-0046
decodes these tracks without change.

## Why the facial joints are needed (#533)

The 495 blendshapes of ADR-0091 move vertices by millimetres (at most 2.5 mm for `blinkL`). The
joints do the large part: for the peak frame of `facerom_t0_face` the delta records move
`FACIAL_LOD1_R_NasolabialFurrow` by 26 mm and turn it by 0.37 rad, and the 15 strongest joints move
by 1 to 3 cm. A face animation of morph weights alone would hardly move, so the facial joints are
part of the skeleton now.

## The skeleton

`p4kx_model::extend_skeleton` appends to the body skeleton the bones of the head skin it lacks (183
of 198, matched by name ignoring case), in the skin's order. Each keeps the skin's bind pose relative
to its parent, moved by the Character's own joint offset, and hangs below its nearest ancestor of
the skin that the result has (`FACIAL_C_FacialRoot` below `Head`). The world poses follow from the
parents' by forward kinematics, so the inverse bind matrices are consistent with the skeleton the
GLB writes whatever the skin's own world poses say; the head mesh is not moved, the joints are
re-bound to the shaped face. The skeleton has 403 bones with a head at LOD 0 (220 and 183). The
weights of the head bind to the facial joints by name (ADR-0079), so `bones_not_in_skeleton` no
longer appears for the head.

The rig's 179 joints are the bones whose CRC-32 is listed in section F of the `RigLogic` chunk, in
the order of the joints of the DNA rig; all 179 are found among the bones of the skin. The offset of
a joint is `DnaRig::neutral_joint_offsets` (slot 15, folded by the joint weights of slot 21), added
to its parent-local translation. Only LOD 0 has the data: the rigs of the lower levels have only
the neutral morphs and region masks, so the heads of other levels are extended without offsets and
cannot be animated.

## The evaluation

`FaceRig` (p4kx-character) holds, per joint axis, the blendshapes that move it and their summed
coefficients. A control animation is turned into the 495 activations: the controls, with the
constant control 0 at 1, then each corrective as the product of the five controls of its row in
section C. The translation of a joint is the sum over the records of the main delta set, the Euler
rotation (x, y, z, applied in this order, radians) the sum over the secondary set, each coefficient
times the step of its head (`scale / (2^22 - 1)`) times the activation, mixed over the heads of the
DNA by the joint weights of the regions. What the heads leave of 1 in a region belongs to the base
head, as for the expression morphs (ADR-0091); a region without blends has the records of the base
head. The result is added to the bind translation of the joint and multiplied onto its bind
rotation. The 495 activations are also the weights of the morph targets: target `n` of the head is
blendshape `n`.

## Sampling

An animation is sampled at the frames where any of its control tracks has a key, because the
inputs change linearly between keys and the viewers interpolate linearly; a frame is dropped when
the line between its kept neighbours misses it by less than 0.1 mm or 0.001 in a weight on every
output. The face clips key nearly every frame (`ai_civ_neutral_idle_08_face` has 4,524 samples of
its 4,530 frames), so the reduction saves little and a long clip is large: 34 MB for 151 s. Joints
that never leave their bind pose and morph targets that never leave 0 get no channel; the weights
are one `weights` channel of the head's mesh with a run of all targets per sample.

## What is not in it

- Slot 17 of the rig, the per-vertex attenuation of the blendshapes, and the comparison with the
  `BShapesGpu` chunk of the skin (#533): not made, the targets are the cells of slot 19 as in
  ADR-0091.
- The expressions below LOD 0 and the joints of the lower levels (#533).
- The wrinkle maps that sections G and H of the rig logic drive.
- The sign and order of the Euler angles are the engine's as StarBreaker documents them; they are
  checked by the clips moving the right joints, not against a rendering of the game.

## Body animations and where they are found

Body animations are the `.caf` files and `.dba` databases below the folders that the skeleton's
parameters list (`bhm_skeleton_v7.chrparams`, `bhf_skeleton_v2.chrparams`): `Animations/Characters/Human/<body>`,
`Animations/pu/<gender>` and `Animations/Characters/Human/heads/<gender>`, nested folders included
(`BodyVariant::animation_folders`). The parameters themselves are still not read: the folders are
constants of the variant, like its skeleton and skin. `Armour::animate` fits each animation: tracks
whose controller ID is that of a bone of the skeleton move it by their keys (ADR-0046), tracks
whose controller ID is that of a control of the head rig are evaluated as above, and the controller
IDs that fit neither are reported in the summary of the GLB. An animation that fits nothing is not
written and is listed as `animation_not_used`. `p4kx chf glb` and `p4kx armour glb` take `--anim`
and `--anim-from`. Additive animations that nothing names are no longer listed one by one when a
name picks the animations (a character's folders hold hundreds).

## Considered options

- Morph weights only: millimetres of motion, the face would not move.
- Bake the joints into more morph targets: the targets would hold the joint motion and the
  skin could not follow the head.
- Sample every frame: larger files for nothing, the inputs have keys.
- Sample at a fixed lower rate: loses the keys the files hold.
