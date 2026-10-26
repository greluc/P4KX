# A head's frame turns with its surface and its body type is checked against its rig

Decided for issue #525, which follows up ADR-0084 (the shaped head). Evidence: the ignored
real-Archive test `real_chf_body_type` over the 39 CHF files with face DNA in the LIVE Archive of
build 4.10.2: every male rig (`0xDD6C67F6`) CHF holds the body type
`25f439d5-146b-4a61-a999-a486dfb68a49` and every female rig (`0x9EF4EB54`) CHF
`d0794a94-efb0-4cad-ad38-2558b4d3c253`, with no exception; the `variant_hash` takes 9 values on the
male rig and 6 on the female one.

## The frame follows the surface

ADR-0084 added the change of the surface normal to the stored normal and left the tangents alone,
so the tangent frame drifted from the normal on every moved area and normal-mapped shading skewed
there. Now `carry_normals` rotates the stored normal and the tangent by the smallest rotation from
the old surface normal at the vertex to the new one, then makes the tangent perpendicular to the
normal again (Gram-Schmidt) and keeps its handedness `w`. Seams and smoothing groups of the
authored frame stay, as before, because the stored frame is turned and not replaced by the surface
normal. Where the old and new surface normals are opposite, or a surface normal is degenerate, the
vertex keeps its frame.

## The body type is checked

`Head::from_character` knows the body type GUID of each head rig. A Character whose body type is
not the one of the rig its `gender_hash` names, including the all-zero body type of a CHF with a
face, gets the head piece with the issue `body_type_mismatch`. The head is still made, shaped by
the DNA, and the exit code is unchanged: the file is inconsistent, not unusable, and the DNA decides
the rig and the body variant.

The `variant_hash` is not checked. It is a stamp of the build of the rig (the low half of the rig's
identity) and differs between CHFs of one body type, so a mismatch says only that the CHF was made
with another build of the rig.

## Considered options

- Fail the head on a body type mismatch: the DNA is still readable and the head is right for it.
- Derive the body variant from the body type: the DNA's `gender_hash` names the rig and is the
  key to the files; the body type is checked against it, not the other way round.
- Keep the additive normal and only orthogonalise the tangent: leaves the normals approximate on
  large moves; the rotation costs one more vector operation per vertex.
