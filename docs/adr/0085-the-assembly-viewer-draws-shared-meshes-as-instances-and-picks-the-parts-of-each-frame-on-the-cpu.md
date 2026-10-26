# The Assembly viewer draws shared meshes as instances and picks the parts of each frame on the CPU

Decided for issue #131 (M9.8). It builds on ADR-0031 (the offscreen viewer), ADR-0040 (the
Assembly), ADR-0077 (shared meshes), ADR-0078 (the Model tab's viewer) and ADR-0080 (palettes and
textures of Assemblies).

## Decisions

- **The view is the Assembly.** `AssemblyScene` is built from the same `p4kx_assembly::Assembly`
  the GLB export writes: the same parts, placement (`Part::world`), level of detail, material
  library, tint palette and node transforms. A view option that changes the Assembly (level of
  detail, left-out ports, replaced Entities) builds it again; the view never has data the export
  lacks. The Export dialog starts with the detail and materials of the view.
- **Shared meshes are flattened and uploaded once.** Models are told apart by the reference of
  the `Arc<Model>` the Assembly shares between parts (ADR-0077), flattened in parallel with the
  code of the Model tab (node transforms applied, skinned meshes in the skeleton's space) and
  merged into one vertex and one index buffer. A part is an instance: three rows of its world
  transform and a colour, 64 bytes. The GPU holds three buffers and one texture per image for
  the whole ship, however many parts it has.
- **Draw groups.** A mesh range (one material of one mesh) with its texture and blend mode is a
  group. The parts that show the same group are one contiguous run of the instance buffer and one
  `draw_indexed` with that many instances. The colour is per instance, so liveries of one mesh
  share a group. Opaque groups are drawn first with depth writes, translucent ones (glass,
  materials with opacity below 1) after them with alpha blending and without depth writes,
  ordered far to near by part since ADR-0196.
- **The frame is picked on the CPU, bounded by the parts.** `DrawList::of` makes one pass over the
  placed parts: hidden ones are skipped, then those whose bounding sphere is outside the six planes
  of the view-projection matrix, then those whose projected radius is below 0.75 pixels. The
  rest are counted into their groups and written into the instance buffer, whose capacity is the
  number of draw items of the scene, so a frame never writes more. The cost follows the parts of
  the view, never the triangles; a repaint with the same camera, size, hidden set and scene
  draws nothing (`Drawn` carries the list's serial). The pick is cached by the UI until the
  camera, the size or the hidden set changes.
- **Materials.** Colours: the base colour factor of the sub-material the export would choose
  (`library.single` or the material ID, else the engine default), replaced by the tint palette of
  the part: `p4kx_gltf::base_colour_factor`, the specular colour of a metal and else the diffuse
  colour, the same function the GLB writer uses for plain materials. A base colour the writer bakes
  from a specular map is not reproduced. The light is normalised to a maximum of 1, so a bright
  material is not clipped, and normals of instances go through the cofactor of the transform. Textures: the diffuse texture the GLB writer uses (`diffuse_texture_path`) at mip 2,
  multiplied by that colour, uploaded once per image and shared by every part that names it.
  Hidden materials (`NoDraw`, flags, zero opacity) and decals are not drawn: a decal's shape is
  only in its texture's alpha, which the viewer does not blend. Alpha-tested materials are drawn
  solid. Normal and specular maps and layered materials are not drawn (ADR-0080 has the same
  limit for the layers). Images are read at most up to 256 MiB of pixels in total; a texture
  that does not fit or cannot be read keeps its plain colour, and the build counts it.
- **Limits.** The shared meshes hold at most 4 million vertices and 8 million triangles, and the
  scene 524,288 draw items; a model or part that would pass them is left out and counted in
  `Left`. A device that cannot take the buffers rejects the scene like the Model tab (ADR-0031).
- **Off the UI thread, cancellable, with progress.** `AssemblyView` builds on a background thread:
  it reads the models (counted as they arrive), flattens them in waves of 64, reads textures in
  waves of 16 and reports each step; every wave checks a `CancelToken`. Dropping the build or
  pressing "Stop" cancels it; the view built before stays on screen until the new one is
  installed. The Loadout resolves to an `EntityNode` that the session keeps for the view, so the
  tree is not resolved twice (#506 is about sharing the index between sessions).
- **GPU memory.** The Assembly buffers are freed when a model is drawn instead and when the window
  stops showing an Assembly (another tab, another selection); the CPU pixels of a texture are
  handed over at the upload and not kept.
- **Hiding is a view state.** "Hide" and "Show" in the Loadout tree add or remove a port path in a
  set; a part is hidden when its path or its parent's is. Nothing is built again and the export is
  not changed. Leaving a port out and replacing an Entity are changes of the Loadout and rebuild.
- **Layout.** The view panel sits on top of the Loadout tree in the Entities tab, resizable at its
  lower edge; new metrics and the decision are in the desktop spec (D.47).

## Measured

Release build, the LIVE Archive of 4.10, `RSI_Polaris` at level 1: 388 parts (279 with a model,
96 not resolved) are built in 2.0 s including the Archive reads, and share 74 meshes: 1.56 million
triangles are stored for 2.74 million placed. Picking the parts of a frame took 0.2 ms at the
worst of 200 camera positions (235 visible parts in 2,445 draw calls). `AEGS_Idris_P`: 1.1 s,
44 shared meshes, 0.77 million stored for 2.28 million placed triangles, 0.14 ms. The picture
of the Polaris at level 3 (2,746 instances in 1,633 draw calls, 28 textures with the textures
mode) is rendered by `a_capital_ship_is_drawn_in_the_window` of the real-Archive suite; the
software adapter of the test machine needs 0.4 s a frame, which says nothing about a GPU.

## Considered Options

- One scene with every part's vertices transformed on the CPU, like the Model tab: millions of
  duplicated vertices and a rebuild on every hide.
- GPU-driven culling with indirect draws: not available on every device the viewer supports
  (ADR-0031) and no gain at the part counts of a ship, where the CPU pass takes a fraction of a
  millisecond.
- A bind group per material with a uniform colour: a draw per material and part instead of a
  draw per group; the per-instance colour keeps the draws at the number of groups.
- Texture arrays or an atlas: fixed sizes or repacking, for a few dozen textures per ship.
- Keeping the models of the last build to rebuild faster after a change: a third copy of the
  geometry in memory; left to a follow-up together with #506.
