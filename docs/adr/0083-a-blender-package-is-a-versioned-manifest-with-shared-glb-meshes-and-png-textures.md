# A Blender package is a versioned manifest with shared GLB meshes and PNG textures

Decided for issue #157, the tracer of M13 (#13); it builds on the shared meshes of ADR-0077 and the
palettes and textures of ADR-0080.

## Decision

**What a package is.** A folder with `scene.json`, one GLB file per distinct mesh below `meshes`
and one PNG file per distinct texture below `textures`. It is written by the new service crate
`p4kx-blender` from an Assembly and by `p4kx entity package`. There is no `.blend` writer: that
format changes with every Blender version, and a decomposed package can be read by any tool.
`docs/blender-package.md` is the description of the format.

**The manifest is typed and versioned.** `scene.json` is the serialisation of
`p4kx_blender::Manifest`; its JSON Schema (draft 2020-12) is generated from the same types and
checked in as `schemas/blender-package.schema.json`, and a test fails when the two differ. The
manifest has `format` (`p4kx-blender-package`) and `version` (`<major>.<minor>`). A major version
changes the meaning of a field, a minor version only adds what a reader can ignore. Readers refuse
a higher major version and warn about a higher minor one.

**Meshes are the Assembly's sharing.** Two nodes draw the same mesh when their Parts have the same
model, material library and palette (the references the Assembly hands out, ADR-0077 and
ADR-0080). The first node of each distinct mesh writes the GLB, through `write_scene_glb` with one
node at the identity transform, so the shared GLB writer is not changed. The node transforms of
the manifest are the Assembly's local transforms converted to glTF axes by the same function the
GLB writer uses (`node_transform`), so a reader composes node transforms and the mesh's own root
and gets what `entity glb` writes.

**Textures are not baked.** The GLB meshes carry plain colours with the palette applied and no
images. The textures of their materials are written once each, identified by their path in the
Archive, as the pixels the Archive holds (8-bit PNG at the chosen mip); the manifest says which
texture each material names for diffuse, normal and specular. A reader that rebuilds the
materials from these inputs (the layered materials of #158) is not tied to the one image per slot
that glTF can express. High-range textures will be a new `format` of a texture in a later minor
version.

**The format is frozen on the points a reader cannot guess.** Every texture says its colour space
(`srgb` for diffuse maps, `linear` for normal, specular and other data) and what its alpha channel
holds (`none`, `opacity`, `smoothness`), so an importer never infers them from a file name. Normal
maps are written Y up like glTF and OpenGL: the green channel is inverted exactly as the GLB writer
does it. A mesh lists its materials with the index of the glTF material in its file, because names
can repeat; a test compares the indices with the writer's order. `format`, `basis`, `unit`, the
texture format, the node kind and the issue origin are enumerations in the schema and `version`
has a pattern; readers refuse an unknown `basis` or `unit`. A node has a `kind` and an optional
`entity`, so Interiors can add nodes of other kinds in a minor version. Skeletons are reserved as
a manifest-level `skeletons` array that meshes and nodes reference.

**Package vocabulary.** CONTEXT.md's Part is a placed Entity, so the package calls its nodes
nodes and its GLB files meshes (`meshes/m0001.glb`, `mesh` on a node).

**Names never become file names.** Meshes and textures are `m0001.glb` and `t0001.png`; the names
from the Archive are only strings in `scene.json`. Texture paths that are not safe paths are not
loaded and are listed as issues.

**Writing is atomic and guarded.** The output is checked (installation guard and replace rule,
`check_output`) before the Archive is opened, before anything is created and again just before it
is moved into place. The package is written into a temporary folder next to the output and moved
into place when complete; the earlier package, if any, is moved aside and removed only after the
new one is in place, and moved back when the move fails. The temporary folder is removed when the
run ends in an error or is cancelled. An output that exists is replaced only when it is empty or
holds nothing but a package whose `scene.json` has the format name, a `meshes` folder of
`m<digits>.glb` files and a `textures` folder of `t<digits>.png` files; anything else is refused
(exit code 2), because the folder is the user's. When the earlier package cannot be moved back
after a failed move, the error names where it is.

## Consequences

The first package of a ship has as many GLB files as the ship has distinct meshes and as many PNG
files as its materials name distinct textures. Interior and Character packages (#161, #162) add
fields and files to the same manifest in minor versions. The add-on (#159) checks `format` and
`version` and builds objects from `nodes` and `meshes`, linking one mesh to every node that names
the same mesh.
