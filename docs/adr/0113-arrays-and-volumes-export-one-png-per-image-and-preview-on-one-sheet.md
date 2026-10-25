# Arrays and volumes export one PNG per image and preview on one sheet

Decided for issue #257. Evidence: `docs/m5-verification.md` (Volumes, arrays and the two
formats).

A DDS mip holds one image per cubemap face, array element and volume slice. Until now only a
plain texture and a single cubemap had a PNG; the 86 volumes and arrays of every Channel (461 on
TECH-PREVIEW) were counted as unsupported.

- **One PNG per image.** An Extraction and `p4kx texture png` write one file per image next to
  the requested name: `_layerN` for an array element, a cubemap face's name (`_posx` …) or
  `_faceN` for a partial cubemap, `_sliceN` for a volume slice, in that order, with the numbers
  padded with zeros to the width of the last. A texture with one image keeps its name. A cubemap
  array is written per element and face, or with `--cross` as one cross per element. A volume's
  slices are those of the exported mip: the depth halves per mip like width and height.
- **One sheet for the preview.** A preview shows one image, so the GUI and
  `Texture::decode_preview` tile the images of an array, a volume or several cubemaps row by row
  from the top left in the smallest square number of columns; a cubemap stays a cross, and the
  empty cells are transparent. `DdsHeader::preview_size` gives the size without decoding, so the
  mip selector and the pixel cap judge the sheet. The facts bar names the number of layers. A
  layer selector would need a second request axis through the decoding thread for textures
  whose images are small and few; the sheet shows them all at once.
- **Bounded headers.** A header is refused with more than 2048 slices or array elements, the
  Direct3D 11 limits, and when a volume is also a cubemap or an array. So an untrusted header
  names at most 12,288 images, which bounds the file names and the sheet; the data of every image
  is still read from the Archive, so no allocation follows a header's claim alone.
- **Formats.** `R16G16_FLOAT` decodes to red and green like the other two-channel formats.
  `R32_UINT` decodes to grey scaled by `u32::MAX`, as `R16_UINT` and the other unsigned integer
  formats already are scaled by their largest value; the single texture of that format is on
  TECH-PREVIEW.
- **The array alpha tail.** Merging an alpha tail into an array was right: a tail in `A8_UNORM`
  keeps its value in the alpha channel of the decoded tail, `merge_alpha` takes it from there,
  and the real-Archive test compared the red channel. The test now picks the channel by the
  tail's format and no longer skips volumes and arrays.
