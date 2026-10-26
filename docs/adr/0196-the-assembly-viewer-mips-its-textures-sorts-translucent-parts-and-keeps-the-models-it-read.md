# The Assembly viewer mips its textures, sorts translucent parts and keeps the models it read

Decided for issue #530, the follow-ups of the review of the Assembly viewer (#131). It amends
ADR-0085.

## Decisions

- **Textures get a mip chain, built on upload.** The texture is read at mip 2 as before; the GPU
  upload builds every level below it down to one texel with a 2 × 2 box filter in linear light
  (sRGB decoded, averaged, encoded again; alpha averaged as it is), in parallel per row, and the
  sampler filters trilinearly. A minified texture is sampled from a level that fits the view, so
  it neither aliases nor shimmers. The chain adds a third of the base pixels on the device; the
  256 MiB budget of ADR-0085 still counts the base level only. Choosing a smaller mip from the
  Archive per view size was left out: it would read a texture again whenever the view is resized.
- **Translucent items are drawn from far to near.** Opaque groups stay one run per group.
  Translucent items (glass, opacity below 1) are picked per part: they follow the opaque runs
  ordered by the depth of the part's bounding sphere centre, farthest first, and neighbours of
  one group share a run. The depth is the distance along the view for a perspective projection
  and the clip depth for an orthographic one. The order is by part, not by triangle, so two
  intersecting translucent parts or the triangles of one part can still blend wrongly; that is
  the usual limit of sorting by object and costs no per-triangle work. Instances stay within the
  capacity of one per draw item. Translucent runs become one draw call per change of group
  instead of one per group.
- **The models read are kept within a bound.** `ModelCache` (p4kx-assembly) holds the
  `ProvidedModel`s a build read, up to 256 MiB by an estimate of their buffers, least recently
  used out first; a model above the bound is not kept. `CachedModels` wraps a `ModelProvider`
  with it. A model read at level of detail `lod` that came out at level `found` answers every
  request from `found` up to `lod`, and, when it has no level up to `lod`, every request below;
  another level is read from the Archive, since its geometry is not in memory. A change of the
  Loadout, and a return to a level read before, reuse the models; a first visit to another level
  reads them. Each `AssemblyView` owns its cache, so it is dropped with the Loadout session. The
  models of the Interior are read by `EntityInterior` itself and are not cached; textures are
  not either.
- **Small fixes.** "Show all (n)" counts the hidden branches that are still ports of the shown
  Assembly, not paths hidden before their port was left out. A port that is replaced and hidden
  shows both chips, "Hidden" then "Replaced". The row width reserves the gap between them from
  the item spacing of the row.
