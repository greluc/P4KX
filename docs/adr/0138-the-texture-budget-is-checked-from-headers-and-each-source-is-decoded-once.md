# The texture budget is checked from headers and each source is decoded once

Decided for issue #379 (follow-up of the review of #377). Builds on ADR-0042 (textured materials).

## Decision

**Budget before decode.** ADR-0042 caps a GLB at 256 images and 2^29 pixels. The cap was checked
only after an image had been decoded and encoded, so a model whose images all exceeded it still
decoded up to four of them at a time and each of its textures, which peaks near 3 GiB.
`TextureLoader` has a second method, `dimensions(path, mip)`, that returns the size `load` would
give without decoding; its default returns `None`. Before a job runs, the writer asks it for the
size of the job's image and holds back those pixels in the tally. A job that does not fit is
refused without loading anything and is listed as over budget. A loader that cannot tell, or a job
whose image turns out larger than expected, is checked after the image is made, as before. The
archive loader answers from the header and the alpha tail's header and applies the per-texture
limit of 2^26 pixels there too.

**Decode once.** Normal and specular maps feed two images each (normal and metallic-roughness;
base colour and metallic-roughness). A per-build cache decodes each source once, however many jobs
use it, and drops it when the last job that needs it has finished, so its memory stays bounded by
the jobs in flight and their neighbours. A texture that fails to load fails once, and is still
reported for every material that needs it.

**Mip and alpha tail.** The archive loader clamps the requested mip to the smaller mip count of the
colour texture and its alpha tail, so a tail with fewer mips gives a smaller normal map with
smoothness instead of losing the whole map to `AlphaTailMips`.

**JSON report of a model on disk.** `p4kx model glb --materials textures --json` for a model file
without an Archive reports `texture_fallbacks: []` and one `textures_note`, as the text report
prints one note, instead of one fallback per texture.

## Considered Options

- Check only the per-texture limit: leaves the per-file budget to the post-decode check; rejected.
- Make normal and metallic-roughness one job: couples two images with different keys and shares
  nothing for base colour and metallic-roughness; the cache covers both.
- Keep every decoded source for the build: unbounded for a ship with hundreds of textures;
  rejected for the release on last use.
