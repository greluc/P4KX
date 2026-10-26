# M5 verification against real Archives

Measured on 2026-10-19 with the Archive of the LIVE Channel, release build. The tests are the `#[ignore]`d real-Archive tests of
`p4kx-extraction` (`real_archive/texture_survey.rs`, `real_archive/texture_known.rs`,
`real_archive/localization.rs`) and the ones of M5.1 to M5.12 beside them; run them as
`CLAUDE.md` describes. Everything they write lands under `TMP`.

## Textures

| Channel | Textures | Alpha tails | Cubemaps | Format without decoder | Several images per mip |
| --- | --- | --- | --- | --- | --- |
| LIVE | 46,891 | 7,430 | 82 | 28 | 86 |

Reading and parsing all header Entries of an Archive takes 1 to 2 s. Merging and decoding a
sample of at most 16 headers of every format, plus cubemaps and textures with an alpha tail,
takes about 1.5 s. Every merged sample has exactly the size its header implies.

Formats on LIVE (headers including alpha tails): BC1 16,037, BC4 12,922, BC3 10,874, BC5 (SNORM)
9,874, BC7 sRGB 1,536, BC7 674, B8G8R8A8 657, B8G8R8X8 400, R8 270, BC6H 272, BC5 (UNORM) 212,
B8G8R8 213, R16 160, R8G8 73, R16G16B16A16F 48, R16G16F 28, R16G16 26, R16F 26, A8 9, R32F 5, and
a few single ones.

Unsupported textures are counted, never failed:

- no decoder: `R16G16_FLOAT` (28); `R16_FLOAT`, `R32_FLOAT` and sRGB variants only occur as
  volumes or arrays;
- several images per mip that are not one cubemap: volumes and arrays (noise, vector fields,
  signed distance fields), which have no single image to preview.

Every sampled BC1, BC3, BC4, BC5, BC6H and BC7 texture decodes.

### Volumes, arrays and the two formats (#257)

Measured on 2026-10-25 on LIVE with `texture_survey.rs` (every volume and
array is decoded and exported, the other formats are sampled): the Texture of a volume or array
decodes per image, the preview tiles the images on a sheet and the Extraction writes one PNG per
element (`_layerN`) or slice (`_sliceN`).

| Channel | Several images per mip | Volumes | Arrays | Decoded and exported per image | Format without decoder |
| --- | --- | --- | --- | --- | --- |
| LIVE before | 86 | 81 | 5 | 0 | 28 |
| LIVE after | 86 | 81 | 5 | 86 | 0 |

`R16G16_FLOAT` decodes to red and green and `R32_UINT` to grey scaled by the largest 32-bit
value, as the other unsigned integer formats are. A tail in `A8_UNORM` keeps its value in the
alpha channel of the decoded tail, and `merge_alpha` takes it from there.

### Texture gaps (#280)

Measured on 2026-10-25 on LIVE with `texture_tails.rs`,
`texture_float.rs` and `texture_cube_order.rs`; the decisions are in ADR-0123.

**Alpha tail layouts.** Every `.dds.a` holds a DDS header, none has the `DDS ` magic:

| Channel | `.dds.a` | Header and numbered parts `.1a` … `.Na` | Header only | Raw, without header | Numbered part without a header |
| --- | --- | --- | --- | --- | --- |
| LIVE | 7,430 | 7,428 | 2 | 0 | 0 |

The headerless layouts of StarBreaker do not occur, so none is implemented; the survey fails if
one appears.

**Floating-point textures.** 107 on LIVE: `R16G16B16A16_FLOAT` 48
(vector fields, colour charts, nebula faces), `R16G16_FLOAT` 28 (flow maps), `R16_FLOAT` 26 and
`R32_FLOAT` 5 (signed distance field volumes). Before, all 107 were written as clamped 8-bit PNG;
now each is detected as high range and written as OpenEXR, one file per image, 8-bit output
tone-mapped. The alpha of the 48 four-channel textures is written to the EXR. None has an alpha tail. The formats `R11G11B10_FLOAT`, `R9G9B9E5_SHAREDEXP`,
`R32G32_FLOAT`, `R32G32B32_FLOAT` and the signed 8-bit formats do not occur in LIVE; they decode
with known-pixel tests. The format survey still reports 0 formats without a decoder.

**Cubemap tail order.** 46 split cubemaps of LIVE hold two or more mips in the header Entry.
For each mip pair from the last part down, the mean colour of each face of the smaller mip was
compared with the face of the larger mip it derives from, and with the next face. In all 123
pairs where the faces differ enough to tell, the own face is closer: the per-image order is
right.

### The CryEngine trailer

`Data\Textures\cubemaps\inventory_setup\cm_inventory_probe_cm.dds` is in LIVE and was
refused by `p4kx texture merge|png` (`TailMismatch`): its header Entry, which holds all seven
mips of the 256 x 256 BC6H cubemap, is 8 bytes longer than the mips (524,264 instead of
524,256 bytes after the header). The 8 bytes are the ASCII markers `CExt` and `CEnd`, an empty
CryEngine extension chunk and the end marker. It is the only such texture in LIVE.
The trailer is not pixel data: `DdsHeader::part_count` accepts exactly 8 extra bytes after a
whole run of mips, `DdsHeader::data_len` names the length without them, and merging and mip
reading leave them out. Any other surplus (7, 9 or 16 bytes, even a `CExt` chunk with data) is
still a `TailMismatch`.

Where the bytes are verified: `part_count` and `data_len` look at the size only, so opening a
texture stays header-only. `merge` reads the 8 bytes from the reader it already has (streaming
path and several-images path) and refuses anything but `CExtCEnd` with
`TextureError::UnknownTrailer`. `read_mip` does the same when the mip comes from the header
Entry's tail, by skipping the last image's smaller mips; when the mip comes from a part, it
checks the size only, because the bytes would need an extra read of the header Entry. Every
header of LIVE now fits a run of mips, and the real test reads the probe's last 8 bytes
and asserts `CExtCEnd`.

## Localisation

LIVE holds the `global.ini` of all 11 languages. Keys are those of the union over the
languages; a key is translated when its text in the language is not empty.

| Channel | Keys | en | de | fr | it | es-ES | es-419 | pt-BR | ja | ko | zh-Hans | zh-Hant |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| LIVE | 90,946 | 99.1 % | 25.3 % | 27.9 % | 25.7 % | 27.9 % | 26.2 % | 25.9 % | 17.3 % | 26.8 % | 27.1 % | 3.9 % |

The other languages translate about a quarter of the keys, so a Record preview in German shows
the key's missing marker for most keys unless the English fallback is chosen. `ui_Abandon`
resolves to `Abandon` in English and `Aufgeben` in German.

### Record previews in a language

A real ship Record (`aegs_avenger_titan`) is previewed in English, then in German, then with
the keys only: each switch re-renders the open Record, the same keys stay, and
the DataForge is read once for all three. The GUI texts table, the image preview and the Record
language selector tests pass.
