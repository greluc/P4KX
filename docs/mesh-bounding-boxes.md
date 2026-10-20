# The two bounding boxes of a mesh chunk

A `Skin` or `Skin2` mesh chunk holds two axis-aligned boxes after its counts: the first (the
model box, bytes 24 to 47) and the second (the scaling box, bytes 48 to 71). Cryengine-Converter
dequantises with the second box for `.cgf` and `.cga` and with the first for `.skin` and `.chr`;
`p4kx-model` uses the second box for every mesh.

## Measurement

`crates/p4kx-extraction/tests/real_archive_dequantisation_boxes.rs` decodes the quantised
(16-byte, signed 16-bit) positions of every mesh chunk with each box and asks which box the
decoded vertices fill exactly (the encoder quantises against a box that is tight around the
vertices; narrow axes included, as `dequantise` applies the same rule). Results, LIVE:

| Companion | quantised chunks | boxes equal | boxes differ | only the first box tight | only the second tight | both | decoded vertices inside the second box |
|---|---|---|---|---|---|---|---|
| `.cgfm` | 325,778 | 295,190 | 30,588 | 0 | 10,527 | 20,061 | 30,588 |
| `.cgam` | 45,127 | 14,104 | 31,023 | 0 | 18,858 | 12,165 | 31,023 |
| `.skinm`, `.chrm` | 0 | | | | | | |

PTU, TECH-PREVIEW and EPTU give the same picture (differing chunks: PTU 30,484 and 30,431,
TECH-PREVIEW 30,783 and 31,306, EPTU 30,464 and 30,249; the first box is the only tight one in
none, the second contains the decoded vertices in all).

Where the boxes differ, the positions were quantised against the second box in every case; the
first box is wider or narrower and contains the decoded vertices in only 17,888 of the 31,023
`.cgam` chunks.

All 20,131 `.skinm` meshes store float positions (20-byte vertices), and no `.chrm` mesh is
quantised, so for skins and characters no position is dequantised and the choice of box
does not matter. The rule of Cryengine-Converter cannot be tested on this data; the second box
stays the rule for quantised meshes.

## Model box

Positions of interior `.cgf` files placed by `IncludedObjects` (#283) may need the first box; the
reader skips it today. The placement code that needs it will read it from the chunk.
