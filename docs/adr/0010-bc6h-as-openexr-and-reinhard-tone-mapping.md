# BC6H exports as OpenEXR with 32-bit float RGB; 8-bit output is Reinhard tone-mapped

BC6H textures (environment cubemaps and other lighting data) hold half floats beyond 0..=1,
signed ones negative values too, so an 8-bit PNG clips them. `p4kx texture png` writes a
BC6H mip as a single-part scanline OpenEXR file (`.exr`) instead: linear `R`, `G` and `B`
channels as 32-bit floats, ZIP-compressed in chunks of 16 lines, written by our own encoder on
top of `flate2`. Every decoded half converts to a 32-bit float exactly, so the file holds the
texture's values bit for bit, negative values and the largest half (65504) included. `--tone-map`
writes an 8-bit PNG instead, and the preview and every other 8-bit decode of BC6H use the same
mapping: Reinhard `v / (1 + v)` per channel on the linear value, then the sRGB transfer
function; negative values and NaN become 0, positive infinity 255. Decided for issue #79.

## Considered Options

- Radiance HDR (`.hdr`, RGBE): small and simple, but a shared 8-bit exponent loses precision
  against the half floats BC6H stores and it cannot hold the negative values of `BC6H_SF16`.
- OpenEXR with half-float channels: half the size and still lossless, but needs a float-to-half
  conversion for no gain in fidelity; ZIP compresses the zero low bits of the 32-bit floats well.
- The `exr` crate as encoder: complete, but pulls SIMD and complex-number crates into a format
  crate for a writer that takes one module; it stays a dev-dependency that reads our files back
  in the tests.
- Clamping to 0..=1 for 8-bit output, as the uncompressed float formats do (amended by
  ADR-0123: they are tone-mapped too): keeps mid-tones
  exact but turns every highlight of an environment map white; Reinhard keeps them apart.
