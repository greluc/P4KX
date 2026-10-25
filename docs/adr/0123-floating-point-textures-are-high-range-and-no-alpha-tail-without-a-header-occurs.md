# Floating-point textures are high range, and no alpha tail without a header occurs

Decided for issue #280. Evidence: `docs/m5-verification.md` (Texture gaps). Builds on ADR-0010
and ADR-0113.

- **Floating-point formats are high range.** `R32G32B32A32_FLOAT`, `R32G32B32_FLOAT`,
  `R16G16B16A16_FLOAT`, `R32G32_FLOAT`, `R16G16_FLOAT`, `R32_FLOAT`, `R16_FLOAT`,
  `R11G11B10_FLOAT` and `R9G9B9E5_SHAREDEXP` answer `TextureFormat::is_high_range` like BC6H.
  `decode_hdr` returns their values unclipped as linear 32-bit floats, a 16-bit float widened
  exactly, and the Extraction and `p4kx texture png` write `.exr` instead of `.png`, one file
  per image as for any texture. Single-channel formats fill red, green and blue alike, two-channel
  formats leave blue zero, as their 8-bit decode already did.
- **8-bit output is tone-mapped.** `decode`, the preview and `p4kx texture png --tone-map` map
  red, green and blue of these formats with Reinhard and the sRGB transfer function of ADR-0010;
  alpha is clamped to 0..=1. The clamp that ADR-0010 left to the uncompressed float formats is
  gone, so they and BC6H share one mapping. Data textures such as signed distance fields, vector
  fields and colour charts are the textures concerned on LIVE (107 of them); their EXR files hold
  the values exactly.
- **The alpha of four-channel formats is exported.** `HdrImage` carries an optional alpha
  channel, one 32-bit float per pixel, set by `decode_hdr` for `R16G16B16A16_FLOAT` and
  `R32G32B32A32_FLOAT` and for a cubemap cross only when every face has one. `write_exr` then
  writes an `A` channel beside `R`, `G` and `B`, so the EXR and the PNG agree. BC6H and the
  formats without alpha have none. An alpha tail is not read for a high-range texture; no
  floating-point texture of any Channel has one, and a real-Archive test asserts it.
- **Signed 8-bit formats.** `R8_SNORM`, `R8G8_SNORM` and `R8G8B8A8_SNORM` decode like BC4 and BC5
  signed: -1 maps to 0 and 1 to 255, -128 counting as -127; a signed alpha is clamped at 0 and
  scaled to 255. Grey, red-green and RGBA follow the unsigned formats of the same width; the
  blue of `R8G8_SNORM`, which has none, is the signed zero 128.
- **Headerless alpha tails do not occur.** A survey of every `.dds.a` of LIVE (7,430), PTU
  (7,432) and TECH-PREVIEW (7,772) finds that each one holds a DDS header, without the `DDS `
  magic and with numbered parts `.1a` … `.Na` except two per Channel that hold all mips
  themselves; no numbered part exists without its header and no tail lacks a header. The three
  layouts of StarBreaker that carry no header (a raw `.a` split by the exact size sum of R8 or
  BC4, and a single raw payload) therefore have nothing to read, and no code for them is added.
  The real-Archive survey `texture_tails.rs` counts the layouts and fails when a raw tail
  appears, so a future game version that adds one is noticed; the exact size match against R8 and
  BC4 is how it would be read.
- **The cubemap tail order is per image.** The header Entry of a split cubemap holds its smallest
  mips; p4kx reads them image by image, StarBreaker mip by mip. On the 46 split cubemaps of LIVE
  with two or more mips in the header tail, the mean colour of every face of mip N+1 follows the
  face of mip N it is derived from in all 123 pairs in which the faces differ enough to tell the
  order apart (`texture_cube_order.rs`). A mip-major reading of the same bytes would take other
  byte ranges for each face, which does not follow the faces above, so the per-image order is
  right and the merged DDS is standard. The mip-major offsets are not scored directly. There is
  no reference rendering of a Star Citizen cubemap; the consistency of each face with the mip
  chain replaces it.

## Considered Options

- Keeping the clamp for floating-point formats and only adding the missing decoders: leaves a
  vector field's range clipped to 0..=1 in the only output the Extraction wrote.
- Leaving the alpha of four-channel formats out of the EXR: simpler, but EXR and PNG would
  disagree and data would be lost without a trace.
- Implementing the headerless tail layouts without data: untestable against the Archive, and
  every guess about a layout that does not occur is dead code.
