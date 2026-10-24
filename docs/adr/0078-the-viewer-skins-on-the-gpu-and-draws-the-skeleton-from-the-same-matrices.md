# The viewer skins on the GPU and draws the skeleton from the same matrices

Decided for issue #132 (M9.9). It builds on ADR-0031 (the offscreen viewer), ADR-0039 (skeletons
and bone weights) and ADR-0046 (animations).

## Decisions

- **One matrix per bone, computed once per frame on the CPU.** `Skeleton` (the bones of the most
  detailed level, ordered so that every bone follows its parent) evaluates an animation at a
  time into `Pose`: for each bone the posed world transform times the inverse of its bind world
  transform, as a column-major 4 × 4 matrix. Rotations are interpolated spherically along the
  shorter arc (the reader keeps consecutive keys in one hemisphere), translations linearly, a
  channel an animation lacks keeps the bind pose, and a time before the first or after the last
  key holds that key. The scratch transforms and the matrices live in buffers that are reused: a
  playing animation allocates nothing, and `Pose::serial` changes only when the matrices do.
- **The GPU blends.** A scene keeps, per vertex, the four strongest bones (ADR-0039 stores up to
  eight, strongest first) and their weights as 12 bytes in a second vertex buffer, and the
  matrices go into a read-only storage buffer. `skinned.wgsl` blends the four matrices by their
  weights, divided by their sum, and moves position and normal; a vertex without weight stays
  where it is, so a rigid part of a skinned model is drawn as stored. The scene vertices stay the
  bind pose, so the pipeline without skinning draws the same picture for a model without a
  skeleton. A skinned mesh ignores the transform of its node, as the glTF export does
  (ADR-0039): its vertices are in the space of the skeleton.
- **The skeleton overlay is the same data.** `skeleton.wgsl` draws one screen-space quad of
  `boneStroke` points per bone from its parent, instanced over the bones, reading the bone's bind
  position and parent from a static buffer and the matrices from the storage buffer; nothing is
  built on the CPU per frame. It is drawn after the model without a depth test, in
  `color-primary`. A static model with helper bones (`CompiledBones`) has the overlay but no
  weights; its animations drive nodes (ADR-0046), not vertices, and the viewer does not play them.
- **Only what can move the skeleton is listed.** Animations are read on the loading thread with
  the model, additive ones left out (ADR-0046: their keys are offsets), and only those with a
  track for a bone of the skeleton kept. The list is offered for a skinned model only. A model
  without bones has no playback and no controls.
- **Playback is plain state.** `Playback` holds the selected animation, the time, playing,
  looping and the overlay switch; `Control` is what the user asks for (play or pause, select,
  loop, seek, skeleton). Selecting an animation plays it from its start, an animation that
  ends without looping stops at its last key and starts again on play, a seek keeps playing
  state, and one frame advances time by at most a quarter second so a stalled window does not
  jump. Time advances only while the Model tab is shown and requests a repaint only while
  playing.
- **Repaints stay cheap.** The picture is drawn again only when the camera, size, pose serial or
  overlay changes, so a paused animation costs nothing. The matrices are written to the GPU only
  when the pose serial is new.
- **The GPU side degrades.** The skinning and overlay pipelines are created inside an error
  scope of their own; if the device refuses them, or the matrices or weights exceed its limits, the
  model is drawn without skeleton and playback instead of the viewer being unavailable. The
  buffers belong to the scene's slot and are destroyed with it (ADR-0031).
- **At most 4,096 bones.** A skeleton with more is left out; the matrices are 64 bytes a bone.
- **Layout.** The controls live in a bar of their own under the 3D view, like a video player's;
  the list of animations sits beside the view; the facts bar keeps its hint. The bar costs 44
  points, so the panel opens higher while it shows a model (349 points, remembered apart from the
  other previews, like the soundbank list) and the view keeps 160 points against its minimum of
  96. New metrics and the decision are in the desktop spec (D.46).

## Considered Options

- Skin on the CPU every frame: millions of vertices of a ship against a 60 Hz budget.
- Eight weights per vertex: a second vertex attribute for the heads; four carry almost all the
  weight and the overlay and playback do not depend on it.
- A bone-matrix uniform buffer: 256 matrices at most with the default limits; skeletons of
  characters have more.
- Controls in the facts bar, to keep the panel at 236 points: the owner preferred a taller panel
  and a bar under the view.
- Playing the node animations of static models in the viewer: needs a node transform per mesh
  per frame; left to a follow-up.
