# The viewer binds the meshes of a static model to the helper bones that drive them

Decided for issue #507, the follow-ups of the review of the skeleton playback. It amends ADR-0078
and builds on ADR-0046 (animations) and ADR-0039 (skeletons and bone weights).

## Decision

- **A static model plays its node animations in the viewer.** A model with helper bones
  (`CompiledBones`) and no bone weights gets a rigid skin: every mesh is bound wholly (weight 255)
  to the bone that drives its node, or else to the bone of the nearest ancestor node that a bone
  drives. A mesh under no driven node stays where it is. The same skinning pipeline, matrices and
  playback bar as for a skinned model move the parts, so doors, turrets and landing gear swing in
  the viewer; the list offers the animations that move at least one helper bone.
- **The node transform still applies to such a mesh.** The vertices of a rigid part are in model
  space, not in the space of a skeleton, so `Part::rigid` places them with the world transform of
  their node and the matrices carry only the change from the bind pose. A mesh with bone weights
  still ignores its node transform, but only when a usable skeleton exists: without one (more than
  4,096 bones, a cycle) its node transform applies, as `Part::of` decides by the skeleton, not by
  `mesh.influences`.
- **The panel height follows the Entry, not the tab.** The panel of a model Entry opens 349 points
  high on every tab and may not be dragged below 285, where the 3D view keeps its 96 points
  (`modelPreview.panelMinHeight`). Switching between the Model and Info tabs no longer moves it.
- **No inert controls when the device refuses skinning.** The report of the renderer says per scene
  whether its skeleton is on the device (`GpuReport::skeleton_held`). When it is not, the bar, the
  list and the Skeleton checkbox are left out, the playback does not tick, and a pose no longer
  causes a new pass: the scene is drawn once.
- **Controls that cannot act are disabled.** Play, the position slider and Loop are dimmed and deaf
  while the bind pose shows or the animation has no length (`Playback::can_play`). The heading of
  the list takes the height of its eyebrow line instead of a slider metric, and a frame time that
  is not finite counts as zero.

## Considered Options

- Binding to the node tree instead of the helper bones: needs a skeleton of nodes with scale and a
  second overlay; the helper bones are what the animations name (ADR-0046).
- Keeping the height by tab and rewording D.46: the panel jumps when the tab changes, which is the
  defect.
- Hiding the controls until the first upload is known: the bar would flash in the other direction;
  the controls show until the report says the skeleton is missing, after one repaint.
