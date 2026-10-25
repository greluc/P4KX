# The two bounding boxes of a mesh chunk

A `Skin` or `Skin2` mesh chunk holds two axis-aligned boxes after its counts: the first (the
model box, bytes 24 to 47) and the second (the scaling box, bytes 48 to 71). Cryengine-Converter
dequantises with the second box for `.cgf` and `.cga` and with the first for `.skin` and `.chr`;
`p4kx-model` uses the second box for every mesh.

## Where the boxes differ

`crates/p4kx-extraction/tests/real_archive/dequantisation_boxes/` reads every `.cgfm` and
`.cgam` companion. Only static models are quantised: all 20,131 `.skinm` meshes store float
positions (20-byte vertices) and no `.chrm` mesh is quantised, so for skins and characters no
position is dequantised and the box does not matter on this data; the rule of Cryengine-Converter
for `.skin` and `.chr` cannot be tested on it. The `MeshInfo` box of the model file equals the
first box in 99 % of the companions (LIVE: 323,297 of 325,778 `.cgfm`), so the first box is the
model box and says nothing about how the vertices were quantised.

The boxes are equal in 295,190 of 325,778 `.cgfm` chunks and 14,104 of 45,127 `.cgam` chunks of
LIVE; they differ in 30,588 and 31,023.

## Which box the vertices were quantised against

The raw 16-bit values of every companion span the full range (the encoder fits them to a tight
box), so the extent of the decoded vertices equals whichever box is used and says nothing. The
test therefore decodes the positions with each box and judges the result by the stored normals:
the face normal of a triangle must agree with the normal of its vertices (the tangent frames of
the same chunk), which an unevenly scaled mesh does not. The score of a chunk is the mean cosine
over up to 200 triangles per submesh.

Of the chunks whose boxes differ, in LIVE:

| Companion | first box scores better by > 0.02 | second box scores better by > 0.02 | tied | first better by > 0.1 | second better by > 0.1 |
|---|---|---|---|---|---|
| `.cgfm` (30,588) | 24 | 3,704 | 26,860 | 7 | 716 |
| `.cgam` (31,023) | 17 | 2,285 | 28,721 | 0 | 365 |

PTU, TECH-PREVIEW and EPTU give the same counts within a few chunks. Where the test can decide,
the second box wins in 99 % of the cases and by a clear margin in 716 of 723 and 365 of 365.
Most differing chunks (about 90 %) cannot be decided this way: their faces are axis-aligned
(rooms, panels), so scaling the axes unevenly leaves the face normals unchanged. For those the
rule rests on the decidable ones and on Cryengine-Converter. The 24 and 17 chunks where the first
box scores better are not examined further; the test fails if clear first-box wins exceed a
twentieth of the clear second-box wins.

## Model box

Positions of interior `.cgf` files placed by `IncludedObjects` are authored in the model (first)
box space, after StarBreaker. `Model::read_with_box` takes a `PositionBox`: `Scaling` (the default
of every other read method) dequantises with the second box, `Model` with the first. The placement
code chooses; nothing in the model says which a file needs. The placement code of the interiors
(`ArchiveModels`) keeps the scaling box: of 400 interior CGFs of LIVE read both ways,
`crates/p4kx-extraction/tests/real_archive/model_details.rs` finds by the face-normal fit above
the scaling box clearly better in 12 and the model box in none, so the model box would distort
them. The test also checks that the two readings differ and stay finite.
