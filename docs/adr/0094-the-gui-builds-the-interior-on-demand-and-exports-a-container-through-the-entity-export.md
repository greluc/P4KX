# The GUI builds the Interior on demand and exports a container through the Entity export

Decided for issue #139, which builds on ADR-0085 (the 3D view of the Assembly), ADR-0086 (an
Entity places its object containers) and ADR-0081 (the Interior).

## Decision

**The view builds the Interior once it is switched on.** The checkbox Interior in the bar of the
3D view asks for it. The view builds the hull as before and, only when asked, adds
`EntityInterior::build_watched` and `Assembly::with_interior` on the same background thread, with
the stage "Reading the Interior" (container `n` of `m`, model files read) and the existing Stop. The
Interior is never built for the view without the checkbox: it can be as large as the hull, and the
view starts light like its level of detail (ADR-0085).

**Switching it off hides, it does not rebuild.** The scene keeps the parts of the Interior as
`PartInfo::is_interior` parts and the hidden mask hides them with their descendants, so off and on
are instant while the scene holds them. A scene built without the Interior gets it with the next
build. Another level of detail, materials or Loadout builds with the Interior exactly while the
checkbox is on, so a scene never holds an Interior the user switched off before the build. The facts
count the Interior's parts only while it is shown, with an own fact `Interior`.

**One scene, one list of pieces.** `AssemblyScene::build` flattens a list of pieces: the parts of
the Assembly and, below the root, the parts of each container of its Interior with the transform
`root.world * placement * part.world` (the one the GLB nodes compose). Models, the sharing of
meshes, the culling and the material rules are the same for both; lights are not drawn.

**Stopping reaches into the Interior.** `Interior::build_watched` and
`EntityInterior::build_watched` take an `InteriorWatch` (a `cancelled` check and a `progress`
callback) and end with `AssemblyError::Cancelled`. The check runs per container and per wave of
model files; `build` and `EntityInterior::build` are the same calls with `InteriorWatch::none()`.

**One export dialog, two subjects.** The Entity export gets the group Interior with a checkbox
(`AssemblyExportSettings::interior`, remembered; the dialog starts with the view's choice once the
user changed it there). The export of a socpak container on its own is the same setup, request,
job, progress and summary with the subject `Container` instead of `Entity`: the Entry paths from
the Archive down to the container, opened in the job off the UI thread, then
`Interior::build_watched` and `Interior::write_glb_with`, which takes the textures like
`Assembly::write_glb_with`. The dialog drops the Loadout lines and the Interior group and keeps the
level of detail, materials, texture size and vertex colours. Offered for a `.socpak` Entry beside Copy
of the preview and in the Export menu.

## Open points

The hull is opaque, so the rooms are seen only beside it or after hiding hull ports; a cutaway or a
translucent hull is a separate issue. The progress of a container counts its distinct model files,
not the walk of its description files. Lights are written to the GLB but not drawn in the view.
