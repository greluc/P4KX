# M5 verification against real Archives

Measured on 2026-10-19 with the Archives of every installed Channel (LIVE, PTU, EPTU, HOTFIX,
TECH-PREVIEW), release build. The tests are the `#[ignore]`d real-Archive tests of
`p4kx-extraction` (`real_archive/texture_survey.rs`, `real_archive/texture_known.rs`,
`real_archive_localization.rs`) and the ones of M5.1 to M5.12 beside them; run them as
`CLAUDE.md` describes. Everything they write lands under `TMP`.

## Textures

| Channel | Textures | Alpha tails | Cubemaps | Format without decoder | Several images per mip |
| --- | --- | --- | --- | --- | --- |
| LIVE | 46,891 | 7,430 | 82 | 28 | 86 |
| PTU | 46,641 | 7,371 | 82 | 28 | 86 |
| EPTU | 46,452 | 7,343 | 82 | 28 | 86 |
| HOTFIX | 46,891 | 7,430 | 82 | 28 | 86 |
| TECH-PREVIEW | 48,383 | 7,737 | 82 | 29 | 437 |

Reading and parsing all header Entries of an Archive takes 1 to 2 s. Merging and decoding a
sample of at most 16 headers of every format, plus cubemaps and textures with an alpha tail,
takes about 1.5 s. Every merged sample has exactly the size its header implies.

Formats on LIVE (headers including alpha tails): BC1 16,037, BC4 12,922, BC3 10,874, BC5 (SNORM)
9,874, BC7 sRGB 1,536, BC7 674, B8G8R8A8 657, B8G8R8X8 400, R8 270, BC6H 272, BC5 (UNORM) 212,
B8G8R8 213, R16 160, R8G8 73, R16G16B16A16F 48, R16G16F 28, R16G16 26, R16F 26, A8 9, R32F 5, and
a few single ones. TECH-PREVIEW adds `R32_UINT` (1) and more `A8_UNORM` (94), `R16_UNORM` (321)
and `BC5_UNORM` (399).

Unsupported textures are counted, never failed:

- no decoder: `R16G16_FLOAT` (28) and, on TECH-PREVIEW, `R32_UINT` (1); `R16_FLOAT`, `R32_FLOAT`
  and sRGB variants only occur as volumes or arrays;
- several images per mip that are not one cubemap: volumes and arrays (noise, vector fields,
  signed distance fields), which have no single image to preview.

Every sampled BC1, BC3, BC4, BC5, BC6H and BC7 texture decodes.

### The CryEngine trailer

`Data\Textures\cubemaps\inventory_setup\cm_inventory_probe_cm.dds` is in every Channel and was
refused by `p4kx texture merge|png` (`TailMismatch`): its header Entry, which holds all seven
mips of the 256 x 256 BC6H cubemap, is 8 bytes longer than the mips (524,264 instead of
524,256 bytes after the header). The 8 bytes are the ASCII markers `CExt` and `CEnd`, an empty
CryEngine extension chunk and the end marker. It is the only such texture in all five Channels.
The trailer is not pixel data: `DdsHeader::part_count` accepts exactly 8 extra bytes after a
whole run of mips, `DdsHeader::data_len` names the length without them, and merging and mip
reading leave them out. Any other surplus is still a `TailMismatch`. Every header of every
Channel now fits a run of mips.

## Localisation

Each Channel holds the `global.ini` of all 11 languages. Keys are those of the union over the
languages; a key is translated when its text in the language is not empty.

| Channel | Keys | en | de | fr | it | es-ES | es-419 | pt-BR | ja | ko | zh-Hans | zh-Hant |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| LIVE, HOTFIX | 90,946 | 99.1 % | 25.3 % | 27.9 % | 25.7 % | 27.9 % | 26.2 % | 25.9 % | 17.3 % | 26.8 % | 27.1 % | 3.9 % |
| PTU | 90,648 | 99.1 % | 25.4 % | 28.0 % | 25.8 % | 27.9 % | 26.3 % | 26.0 % | 17.3 % | 26.9 % | 27.2 % | 3.9 % |
| EPTU | 90,343 | 99.1 % | 25.5 % | 28.1 % | 25.8 % | 28.0 % | 26.4 % | 26.1 % | 17.4 % | 27.0 % | 27.3 % | 3.9 % |
| TECH-PREVIEW | 93,283 | 99.1 % | 24.6 % | 27.2 % | 25.0 % | 27.2 % | 25.6 % | 25.3 % | 16.8 % | 26.1 % | 26.4 % | 3.8 % |

The other languages translate about a quarter of the keys, so a Record preview in German shows
the key's missing marker for most keys unless the English fallback is chosen. `ui_Abandon`
resolves to `Abandon` in English and `Aufgeben` in German in every Channel.
