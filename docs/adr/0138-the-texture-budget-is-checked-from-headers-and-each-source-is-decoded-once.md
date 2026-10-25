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
base colour and metallic-roughness). The jobs run four at a time. For each such chunk the distinct
sources of its jobs are decoded first, in parallel and without any job waiting on another, and the
jobs are then made from that set; a decode that is itself parallel therefore cannot deadlock on a
shared source. After the chunk only the sources the next chunk uses stay in memory, and only up to
2^27 pixels together; the rest are dropped and decoded again if a later chunk needs them. The peak
is the sources of one chunk (at most twelve) plus that cap, and nothing grows with the number of
materials. A texture that fails to load fails once per chunk and is reported for every material
that needs it.

**Priority.** Pixels held back for a chunk's jobs are returned before the images are counted in job
order, so an earlier job is never refused because a later job's image was reserved. A
metallic-roughness image whose size depends on whether the normal map has smoothness is not
reserved; it is checked once made.

**Mip and alpha tail.** The archive loader clamps the requested mip to the smaller mip count of the
colour texture and its alpha tail, so a tail with fewer mips gives a smaller normal map with
smoothness instead of losing the whole map to `AlphaTailMips`. A tail whose size at the chosen mip
differs from the colour texture's is refused before anything is decoded.

**JSON report of a model on disk.** `p4kx model glb --materials textures --json` for a model file
without an Archive reports `texture_fallbacks: []` and one `textures_note`, as the text report
prints one note, instead of one fallback per texture.

## Considered Options

- Check only the per-texture limit: leaves the per-file budget to the post-decode check; rejected.
- Make normal and metallic-roughness one job: couples two images with different keys and shares
  nothing for base colour and metallic-roughness; the cache covers both.
- Keep every decoded source for the build, or until its last user: unbounded for a ship with
  hundreds of textures; rejected for the chunk-wise hold under a cap.
- A per-source once-cell shared by the running jobs: a parallel decode waiting inside one can steal
  a job that needs the same cell and deadlock; rejected.
