# The expressions of a lower-level head are those of level 0 carried over its surface, and slot 17 is not applied

Decided for issue #533, on top of ADR-0091 (expressions), ADR-0098 (face animation), ADR-0200
(what follows the head) and ADR-0281 (a story character's own head). Evidence: the stock male head
`protos_human_male_face_t1_pu` and Miles Eckhart of the LIVE Archive
(`crates/p4kx-assembly/tests/real_archive/real_head_expressions.rs` and `real_npc_face.rs`).

## What the data says

**Slot 17 of a DNA rig.** The slot of the stock male head is 2,769,664 bytes, exactly 62 x 5,584 x 8:
eight bytes per vertex for each of the 61 heads and one more block. Of its 2.77 million bytes 1.32
million are zero and 48,191 are 255. At the 24,274 moves of the blendshapes compared in ADR-0281 the
eight bytes are never all zero and never 255; their mean is about 32 of 255. The `BShapesGpu` chunk
of the head skin agrees with the unattenuated cells of slot 19 for head 0 within one quantisation
step (ADR-0281). A multiplication by slot 17 would have shrunk them to about an eighth, so the chunk
the game ships for the GPU does not carry it. The meaning of the eight bytes is not known.

**The lower levels.** The rigs `_lod1.dna` and below hold slots 0, 1, 12, 18 and 22 only (names,
neutral morphs and region masks), no slot 19. The meshes of the lower levels are remeshes, not
subsets, of level 0: of 3,158 vertices at LOD 1 only 2 coincide with a vertex of level 0 and the
nearest vertex is 1.3 mm away on average, 11 mm at most. They lie on its surface, though: the
distance to the nearest triangle of level 0 is 2 micrometres at the median, 0.07 to 0.23 mm at the
95th percentile and 2.4 mm at most, for LOD 1 to 3. The expressions move vertices by millimetres
(2.5 mm for `blinkL`), so a transfer by the nearest vertex would add error as large as the signal,
and a transfer over the surface adds almost none.

## Decision

1. The targets are the cells of slot 19 as before. Slot 17 is not applied: the data of the head
   itself, the `BShapesGpu` chunk, is unattenuated, and what the eight bytes mean is not known well
   enough to scale targets by them.
2. A head at LOD 1 and up gets the targets of its level 0, carried over: the shaped head at level 0
   (`shaped`, or the head's own geometry for a story character) is made with its targets, each
   vertex of the lower level takes the nearest point of the level-0 surface and the move there,
   interpolated by the barycentric weights of the triangle. A vertex whose move is exactly zero is
   left out of a target and a target that moves no vertex is dropped (listed as
   `empty_expression` if it was named). The number of each blendshape travels with its target, so a
   face animation drives them as at level 0.
3. `armour glb`, `character package` and `chf glb` write all expressions at every level by default;
   `--expression` and `--no-expressions` apply at every level.

At LOD 1 to 3 of the stock male head the peak move is 9.9 to 10.2 mm against 10.4 mm at level 0, and
399 of 399 targets stay. Eckhart keeps 399 of 399 at LOD 1 and 398 at LOD 2.

## What is not in it

- The facial joints of the lower levels: their rigs have no delta records and their skeletons no
  facial bones, so the face animation drives the morph weights only (ADR-0098).
- The hair, brows, lashes and beard at a lower level: they follow the head at level 0 only
  (ADR-0200) and have no targets there.
- The cost of the search is a pass over the triangles of level 0 for each vertex, pruned by
  bounding spheres and parallel; a lower level of a head takes tens of milliseconds.

## Considered options

- No expressions below level 0 (the state before): a head at a distance would not blink while its
  face animation sets weights.
- The nearest vertex of level 0: the error is as large as the signal.
- Apply slot 17: the shipped chunk says it is not baked in, and the meaning is unknown.
