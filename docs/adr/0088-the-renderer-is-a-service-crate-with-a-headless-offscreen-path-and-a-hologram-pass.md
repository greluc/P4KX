# The renderer is a service crate with a headless offscreen path and a hologram pass

Decided for issue #172 (M14.7); amended for #573 (the see-through hologram). It supersedes the part of ADR-0031 that kept the viewer's GPU code
inside `p4kx-gui` and added no renderer crate; the rest of ADR-0031 (the picture drawn offscreen
and blitted, the slots, the error scopes), ADR-0078 and ADR-0085 stand and now describe
`p4kx-render`.

## Decisions

- **The viewer's scenes and wgpu drawing move into the service crate `p4kx-render`.** The CLI may
  depend only on service crates (ADR-0003), and the code was welded to `eframe::egui_wgpu`. The
  crate holds `Scene` and `AssemblyScene` with their `DrawList`, the `Camera`, the `Skeleton` and
  its `Pose`, and the `Renderer` with its pipelines, slots and `GpuReport`. The move changed no
  behaviour: the viewer tests and the pixel checks of the render tests passed before and after,
  and the GUI draws the same picture.
- **The GUI keeps a thin paint-callback adapter.** `viewer/paint.rs` holds the `Renderer` in the
  callback resources of egui, passes a `Frame` (subject, camera, size, look, colours, shading) to
  `prepare` and the serial of the scene to `paint`, and fills the colours of the picture from the
  theme palette, so the renderer knows no egui. `Playback`, the loading thread of a model and the
  viewer support stay in the GUI.
- **One wgpu, the version eframe brings.** `wgpu` 30.0.1 is a direct dependency of `p4kx-render`
  with the `std`, `wgsl`, `vulkan`, `dx12` and `metal` features; `egui-wgpu` 0.37 asks for 30.0, so
  `Cargo.lock` holds one wgpu. The crate forbids `unsafe`; the DWM module of `p4kx-gui` stays the
  only place that has some (ADR-0006).
- **The offscreen path is the same renderer on a device without a surface.** `Offscreen::new`
  asks for the most capable adapter, or only for the software one with `Adapters::Software`, and
  sets the `Renderer` up on it with an sRGB blit format that it never uses. `draw` prepares and
  submits the frame, then copies the resolved picture, now also a copy source, into a row-aligned
  buffer inside validation and out-of-memory scopes, maps it and returns a `Picture` of straight
  RGBA that writes itself as a PNG. A device lost, a scene the buffers refuse, a size of zero or
  beyond the device and a failed read-back are typed errors. The futures of wgpu are run by a
  small `block_on` that parks the thread, so no executor crate is needed.
- **The hologram is a second shading of the Assembly instances.** `Shading::Hologram` draws the
  same shared meshes and instances in one pass, every surface of the model with no depth test
  and no depth write, so the inner structure and the far side show through the near side. The
  shell is two-sided and shaded from the angle between the normal and the eye. Each layer is
  weaker the deeper it lies: its body and its edge fade exponentially with the distance of the
  fragment from the eye between the near and the far plane of the camera (carried in the `w` of
  the `eye` and `key_light` rows of the frame uniform), the edge more slowly than the body, so
  the glow stays on the silhouette. The blend is `constant * source + (1 - source alpha) *
  destination`, with the blend constant the cap `opacity + 0.2 * glow`: the layers add up to that
  opacity and never past it, however many surfaces overlap, so coplanar overdraw (a decal on a
  hull, the thousands of parts of an interior) cannot make a hot spot or turn the shell solid.
  The default opacity is 0.25, which gives a cap of 0.45 at the default glow of 1. The frame
  uniform grows from 96 to 128 bytes for the tint and the two shares of the cap; the other
  shaders read their 96-byte prefix. The pipeline is created inside an error scope: a device that
  refuses it draws the viewer as before and says `OffscreenError::Shading` for a hologram. The
  earlier design drew the nearest surface only (a depth pass, then the shell with `LessEqual`),
  which showed a nearly solid hull.
- **Premultiplied in the picture, straight in the PNG.** The picture is cleared to transparent,
  so the multisample resolve in linear space stays right at the edges; the read-back puts the
  background under the layers, then divides each partly covered pixel by its alpha, in linear
  space, before it is encoded again.
- **The camera is the one of the game.** `HologramCamera::read` in `p4kx-assembly` reads
  `cameraFOV`, `ownDefaultViewAngle` and `ownCameraDistanceScaler` of the one `UIHoloVehicle_Config`
  Record (10, 60 and 1.5 in the current game; a field it lacks keeps that value, a database
  without the Record has none and the CLI uses the same values). `Camera::holographic` stands the
  camera on the view angle from the nose (+Y) towards +X, at an elevation of 20 degrees by
  default (an option, since the Record has none), at the distance at which the bounding sphere of
  the Assembly fits the narrow lens times the distance scaler. Values from an Archive are
  untrusted: a value that is not finite is replaced by the default and the angles and the factor
  are clamped.
- **`p4kx ui hologram <archive> <entity> --out <file.png>`** resolves the Entity with its Loadout
  like `entity glb`, flattens it into an `AssemblyScene` with plain colours, draws it and writes
  the PNG. `--size`, `--background` (`transparent` or `#RRGGBB[AA]`), `--view-angle`,
  `--elevation`, `--fov`, `--distance-scaler`, `--tint`, `--opacity`, `--glow`, `--lod`,
  `--software` and the Loadout changes are options; the report names the adapter and the time of
  each step. The silhouette parameters of the Record (`silhouetteParams`) are not read: the look
  is an option of this tool, not a copy of the game's shader.
- **CI draws on a software adapter.** The test jobs install `mesa-vulkan-drivers` (lavapipe) on
  Linux, WARP is part of Windows, and `P4KX_REQUIRE_SOFTWARE_ADAPTER` makes a missing adapter fail
  the tests that would otherwise print why they checked nothing. The reference image of the
  synthetic ship (`crates/p4kx-render/tests/reference`, never game data) is drawn on the software
  adapter and compared with a tolerance: the covered area within 10 % of the reference, and every
  premultiplied channel within 24 for all but 5 % of the covered pixels, since rasterisers
  differ at the edges; the ship fills the frame, so the budget is measured on the ship, not on
  the background. `P4KX_BLESS_REFERENCE=1` redraws it.

## Considered Options

- The CLI depending on `p4kx-gui` or on eframe: front-ends may not depend on each other
  (ADR-0003) and a command-line tool would link a window toolkit.
- A CPU rasteriser for the hologram, as StarBreaker does: a second renderer with its own scene
  and camera for a picture the GPU draws in milliseconds.
- Additive blending of every layer for an X-ray look: order independent, but the interior of a
  ship holds thousands of parts, which turns the picture white.
- Reading `silhouetteParams` to copy the game's glow: the shader of the game is not known and
  the parameters (edge width in pixels, brightness 5) have no meaning for another renderer. The
  Record in the current game holds one `SSilhouetteParamsDef` with `Enable="false"`,
  `FillIntensity="0"`, `EdgeIntensity="1"`, `EdgeWidth="4"`, `Brightness="5"` and a near-white
  tint: a disabled screen-space post effect, while the hologram itself is the material
  `VehicleHologram.mtl`. The look therefore keeps its own defaults (tint `#4DC7FF`, opacity 0.25,
  glow 1, whose edge intensity happens to equal the game's 1) and the options `--tint`,
  `--opacity` and `--glow`.
