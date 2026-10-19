# tiny-skia rasterises UI movie frames from a backend-free scene

`p4kx ui render` draws the first frame of a UI movie. `p4kx-gfx` first composes the frame into a
`Scene`: every shape converted once into fill and stroke paths in twips, its placements with
their affine transforms, colour transforms and masks, and a list of what was skipped. The scene
knows no raster type, so the SVG export (#168), text (#169) and screens (#170) build on it
without touching the rasteriser. `Scene::rasterize` then draws it with tiny-skia 0.12.0
(BSD-3-Clause, the Linebender project, released February 2026, resvg's renderer), with
`default-features = false` and `std` and `simd` only, since `p4kx-gfx` writes PNG itself.
tiny-skia already covers what SWF shapes need: quadratic paths with the even-odd rule, strokes
with caps, joins and miter limits, linear and two-point conical gradients with pad, reflect and
repeat, bitmap patterns with nearest or bilinear filtering, and coverage masks. BSD-3-Clause sits
beside the BSD-3-Clause `exr` the workspace already ships. OSV and RustSec list no advisory for
tiny-skia or tiny-skia-path.

tiny-skia is not free of `unsafe`: its `wide` SIMD types (adapted from the Zlib-licensed `wide`)
call `core::arch` intrinsics behind the `simd` feature, and a few constants and `bytemuck`
impls use unchecked constructors. Its code also holds asserts and `unwrap`s, about 65 sites.
Every fill, stroke and mask fill therefore runs inside `catch_unwind`; a panic skips that layer
and is listed as `raster` in the report. `cargo xtask lints` keeps `panic = "unwind"` for this,
as for lewton and zune-jpeg (ADR-0013, ADR-0015).

Movie data is untrusted, so the scene and the canvas are bounded. A frame visits at most
`MAX_SCENE_PLACEMENTS` (2^18) placements and draws at most `MAX_SCENE_PATHS` (2^20) layers,
which also bounds sprites placing themselves or fanning out; sprites nest at most
`MAX_SPRITE_DEPTH` deep; a frame holds at most `MAX_SCENE_MASKS` (256) masks, and a mask inside
a mask is not applied. A gradient has at most `MAX_GRADIENT_STOPS` (15) stops, as many as SWF
can store. A canvas is at most `MAX_CANVAS_SIDE` (16384) pixels a side and `MAX_CANVAS_PIXELS`
(2^25) pixels. Masks are built when first needed and dropped after their last item, within 256
MiB; colour-transformed bitmap copies stay within twice `MAX_BITMAP_PIXELS`, and the bitmaps of
one rendered frame, decoded in parallel, hold at most twice `MAX_BITMAP_PIXELS` together.
Whatever exceeds a limit is skipped and listed, never a panic.

## Considered Options

- vello_cpu 0.3.0 (MIT OR Apache-2.0, October 2026): `forbid(unsafe_code)` in itself, but its
  SIMD comes from `fearless_simd`, which uses `unsafe` as tiny-skia does. It has had a breaking
  release every one or two months (0.1 in July, 0.2 in August, 0.3 in October 2026), pulls in
  `glifo`, `skrifa`, `read-fonts`, `peniko`, `kurbo` and `guillotiere`, and `epaint` already
  locks 0.2.0, so it would add a second version.
- raqote 0.8.5 (BSD-3-Clause): last released in September 2024, with less maintenance than
  tiny-skia and no feature tiny-skia lacks.
- zeno 0.3.3 (MIT OR Apache-2.0): only rasterises paths into coverage masks, so gradients,
  bitmap patterns, colour handling and compositing would all be ours.
- An own scanline rasteriser: no dependency, but anti-aliasing, curve flattening, strokes and
  gradients would be written and tested here for no gain.
