# The 3D viewer draws offscreen inside an egui paint callback

The Preview shows a Model in 3D (M9.7, #130): the most detailed level of a `.cgf`, `.cga` or
`.skin` with its nodes applied, lit and framed to fit, that the user turns, pans and zooms. The
window is eframe with the wgpu renderer (ADR-0006), so the drawing is wgpu and has to live inside
egui's frame.

## Decisions

- **wgpu through eframe, no renderer crate.** The viewer is an `egui_wgpu::CallbackTrait` paint
  callback. `eframe::egui_wgpu` re-exports `wgpu` and the callback API, so the workspace gains no
  graphics dependency; the one new crate is `bytemuck` (1.25.2, the version `egui-wgpu` already
  uses) so that vertex and index slices are cast to bytes without `unsafe`.
- **The model is drawn into its own picture, which the callback blits into the panel.** `prepare`
  records one render pass into egui's command encoder: a 4× multisampled `Rgba8Unorm` colour
  attachment resolved into a texture, and a `Depth32Float` attachment. `paint` draws that texture
  as a full-panel triangle with nearest filtering. The main window pass keeps no depth and no
  multisampling, so `NativeOptions` and egui's own pipelines stay as they are and the viewer
  cannot disturb them. The picture grows in steps of 64 pixels, so resizing the panel does not
  allocate a texture per pixel of change; the pass draws into its top-left corner and the blit
  samples only that corner. Shaders write sRGB-encoded colours; a blit to an sRGB surface decodes
  them first, so both surface kinds show the same colours.
- **One scene for the GPU, flattened off the window thread.** `ModelPreview` reads the model and
  its geometry companion on a background thread (level 0 only, `LodSelection::Only(0)`) and
  flattens it into a `Scene`: one vertex list of position, normal and texture coordinate and one
  `u32` triangle list in model space, Z up, metres, with every node transform applied. A mirrored
  transform reverses the winding and keeps the normals consistent, a missing normal is drawn flat
  from the screen-space derivatives, and the node chain is walked with a cycle guard. The camera
  looks at the box of the finite vertices; the GLB writer turns Z up into Y up for export, the
  viewer needs no conversion.
- **Untrusted sizes are capped before anything is allocated.** A model file or companion above
  256 MiB, more than 4,000,000 vertices or more than 8,000,000 triangles is refused with a message;
  a texture is only uploaded when both sides are at most the smaller of 8,192 texels and the
  device limit and the data is exactly four bytes per texel, with checked arithmetic. A mesh whose
  buffers exceed the device's `max_buffer_size` is rejected for that scene and says so.
- **GPU caches hold one entry and drop it at once.** The mesh buffers and the base-colour texture
  live in `Slot`s keyed by the scene's serial number; storing another key drops, and explicitly
  destroys, the previous resource, so switching models never holds two, while a model that stays selected is uploaded once. The
  uniform buffers are
  written with `queue.write_buffer`; a frame allocates nothing but egui's own callback box. The
  texture cache holds the neutral white base-colour texture until materials are read; the
  bind group and the shader already sample it, so textures arrive without a new pipeline.
- **A device that cannot draw, or is lost, is a message, not a crash.** The first frame probes the
  device: texture and buffer limits, 4× multisampling for both formats (else 1×), and shader and
  pipeline creation inside a validation error scope. Each frame's mesh upload and its drawing run
  inside validation and out-of-memory scopes: an upload that fails rejects that scene only, a
  drawing that fails stops the viewer; a device-lost callback sets a flag. All of it ends up in a
  `GpuReport` the window reads. `ViewerSupport` turns it into `Problem`s that the Model tab shows
  in place of the view while every other tab, table and tree keeps working.
- **Deviation from the design system: the 3D view has square corners.** Controls and surfaces have a
  4 px radius (ADR-0006), but an egui paint callback can only be clipped to a rectangle, so the
  picture and its hairline are square. Everything around it follows the design system (D.35).
- **Interaction is a pure camera plus a thin egui mapping.** `Camera` (orbit with the pitch clamped
  short of the poles, pan across the view, zoom limited to 0.02 to 60 radii, reset) is plain
  arithmetic tested without a window; the egui code maps the primary drag, the secondary, middle
  or Shift drag, the wheel and pinch and the keys of the focused view to it.

## Considered Options

- Draw in egui's own pass with `NativeOptions::depth_buffer`: changes the format of the whole
  window's pass and needs the viewer's pipeline to match whatever the host configured.
- A renderer crate (three-d, bevy): a second graphics stack beside egui's, for what is one
  pipeline.
- A CPU rasteriser (tiny-skia, ADR-0020): fine for UI movies, far too slow for models with
  millions of triangles that must stay responsive.
- Registering the picture as an egui texture: needs the renderer lock during `prepare` and a
  second texture-lifetime scheme beside the slots.