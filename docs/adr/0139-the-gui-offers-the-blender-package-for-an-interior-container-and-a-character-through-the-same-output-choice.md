# The GUI offers the Blender package for an Interior container and a Character through the same output choice

Decided for issue #570, which builds on ADR-0104 (the GUI exports an Entity as a Blender package),
ADR-0126 (an Interior is packaged on its own) and ADR-0107 (a Character is one rigged GLB mesh
behind a character node). D.54 of the desktop spec. It replaces the scope paragraph of ADR-0104,
which left both out until the library wrote their packages.

## Decision

**An Interior container offers the package in its existing dialog.** The export setup of a
container shows the same "Output" choice, "GLB file" or "Blender package", as the setup of an
Entity. The package form hides the materials, as for an Entity, and keeps level of detail, texture
size and vertex colours. The job reads the Interior exactly as for the GLB file and calls
`write_interior_package` with the models and textures wrapped to stop at Cancel. The progress
window and the summary carry the titles of the package; the summary counts the containers opened
and the nodes of the package.

**A Character gets a setup dialog.** The Characters toolbar button is renamed from "Export as GLB…"
to "Export…" and opens a setup with the same "Output" choice, the folder or file row and the same
notices as the Entity (an earlier package is named and replaced, anything else is refused, an
installation is refused). The level of detail and the materials stay the choices of the view in the
toolbar, because the Character is built with them. The job writes the package with
`write_character_package`, recording the CHF file name and the Loadout the tab holds as
`source.character`. The chosen form and folder are remembered for the next export of the tab.

**One output choice for the three subjects.** The form, the file, the package folder and what the
dialog says about them (`Output`) are one type that the Entity setup and the Character setup both
hold; the dialog rows, the notices and the failure handling of a package (`verdict`) are shared.
Nothing of the Entity flow is copied.

**No animation clips in the Character package.** The tab offers no choice of animation clips and
its GLB export writes none, so the package is written with no clips: the skeleton, the shape keys
of the expressions and the materials. The setup says so. Animation clips need the lookup of the
clips of a Character, which lives in the front-end of the command line; moving it to a service
crate is a separate decision.

**A Character package has no byte progress.** As for an Entity package, the writer reports none:
the stage "Writing" shows the name and Cancel.
