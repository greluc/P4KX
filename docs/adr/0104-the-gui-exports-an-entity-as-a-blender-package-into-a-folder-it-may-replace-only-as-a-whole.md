# The GUI exports an Entity as a Blender package into a folder it may replace only as a whole

Decided for issue #163, which builds on ADR-0083 (the package format), ADR-0094 (the GUI export of
an Entity and of an Interior container) and ADR-0099 (the Character export). D.50 of the desktop
spec.

## Decision

**One dialog, two output forms.** The export setup of an Entity gets the choice "GLB file" or
"Blender package" (`AssemblyExportForm`, remembered in `AssemblyExportSettings::package`). Level of
detail, texture size, vertex colours, the Loadout changes and the Interior are the same choices as
for the GLB file; the material choice is dropped for a package, which always carries the textures
of its materials. The Entities toolbar button is renamed from "Export as GLB…" to "Export…".

**The target is the package folder itself.** The system folder dialog picks it; it may be new,
empty or hold a package an earlier run wrote. `p4kx_blender::check_output` decides, before the job
starts, so the dialog shows the answer: an earlier package is named and replaced, anything else is
refused. Nothing is added to or removed from a non-package folder, and a path inside a Star Citizen
installation is refused as for every other export.

**Conflict policy.** The Conflict policy of the Extraction (skip, overwrite, overwrite if changed)
does not apply to a package: the package writer builds the new package in a temporary folder next to
the target and swaps it in once complete, so the earlier package stays intact on failure and
cancel. A skip or if-changed mode would leave a package mixed from two runs, which a reader of
`scene.json` cannot tell apart.

**The same job, progress, cancel and report.** The job resolves the Loadout, reads the models and
the Interior exactly as for the GLB file, then calls `write_package` with the models and textures
wrapped to stop at Cancel. The writer reports no progress, so the stage "Writing" shows no byte
count. The summary carries the `PackageSummary`; its issues are the report of the window.

**Scope.** Only the Entity export offers the package. An Interior container on its own and the
Character export write no package in the library today (#162 is open), so their dialogs stay
GLB-only; they take the form when the library has it.
