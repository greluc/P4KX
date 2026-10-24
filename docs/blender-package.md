# Blender package

A Blender package is a folder that describes an Assembly in files any importer can read: a scene
description, shared GLB meshes and PNG textures. It does not depend on Blender's file format, so
new Blender versions do not break it; the [Blender add-on](../blender_addon/README.md) imports it
([ADR-0093](adr/0093-the-blender-add-on-is-an-extension-that-reads-and-validates-a-package-without-blender.md)),
and so can any other tool. `p4kx entity package` writes it ([ADR-0083](adr/0083-a-blender-package-is-a-versioned-manifest-with-shared-glb-meshes-and-png-textures.md)).

The JSON Schema of `scene.json` is generated from the Rust types of `p4kx-blender` and checked in
as [schemas/blender-package.schema.json](../schemas/blender-package.schema.json); a test fails
when the two differ (`P4KX_BLESS=1 cargo test -p p4kx-blender --test schema` regenerates it).

A note on words: a **node** of the package is a placed Entity (a **Part** of the Assembly in the
glossary); a **mesh** of the package is a GLB file with the geometry of one model.

## Layout

```text
<package>/
  scene.json            the manifest
  meshes/m0001.glb      one GLB per distinct mesh
  meshes/m0002.glb
  textures/t0001.png    one PNG per distinct texture
```

- The folder holds nothing but these. Every file name is an id made by the writer (`m` or `t` and
  at least four digits); a name from the Archive never becomes a file name, so no name can lead
  out of the package. Mesh ids number the meshes from 1 in the order of their first node. Texture
  ids number the textures in the order of their first use, and a texture that could not be
  written leaves a gap, so ids are not always consecutive.
- `scene.json` is written last. A folder with a `scene.json` whose `format` is
  `p4kx-blender-package` is a complete package.

## Writing a package

`p4kx entity package <archive> <entity> --out <folder>`. The Loadout, the parts, `--lod`,
`--exclude` and `--replace` are those of `p4kx entity glb`; `--texture-mip <n>` picks the mip of
every texture and `--no-vertex-colours` leaves the vertex colours out of the meshes.

- The output is checked before the Archive is opened (`p4kx_blender::check_output`). The folder
  must not exist, or be empty, or hold a package an earlier run wrote: `scene.json` with the
  format name, a `meshes` folder holding only `m<digits>.glb` files and a `textures` folder
  holding only `t<digits>.png` files. The earlier package is then replaced. Any other folder or
  file is refused with exit code 2 and left alone.
- A folder inside a Star Citizen installation is refused with exit code 2.
- Everything is written into a temporary folder `<name>.p4kx-<process id>.part` next to the
  target, which replaces the target once the package is complete; the check above is repeated
  just before. The earlier package is moved aside to `<name>.p4kx-<process id>.old` and removed
  after the new one is in place; if the move fails it is moved back, and if that fails too the
  error names the `.old` folder. After an error or a Ctrl+C the temporary folder is removed and
  an earlier package is still there. A second Ctrl+C ends the process at once and can leave the
  temporary folder behind.

## `scene.json`

All lengths are metres, all transforms are relative to the parent node. Writers add fields in
minor versions, so a reader ignores fields it does not know. Absent optional fields are not
written (there are no `null`s).

| Field | Meaning |
| --- | --- |
| `format` | Always `p4kx-blender-package`. |
| `version` | `<major>.<minor>` of the package format, matching `^[0-9]+\.[0-9]+$`; `1.0` today. |
| `generator` | `name` (`p4kx`) and `version` of the program that wrote the package. |
| `source` | `channel` (the Channel folder, e.g. `LIVE`), `build` (`branch`, `version`, `public_patch`, `change_number`, `build_id` of the Archive's build manifest) and `entity` (`class_name`, `guid`). `channel` and `build` are absent when unknown. |
| `coordinates` | `basis` is `gltf_y_up` and `unit` is `metre`. A reader refuses a `basis` or `unit` it does not know. |
| `options` | What the package was made with: `lod`, `texture_mip`, `vertex_colours`, `exclude` (ports) and `replace` (`port`, `entity`). |
| `nodes` | The scene hierarchy, see below. |
| `meshes` | The GLB meshes, see below. |
| `textures` | The PNG textures, see below. |
| `issues` | What could not be resolved or written. |

`format`, `basis`, `unit`, `Texture.format`, `colour_space`, `alpha`, `Node.kind` and
`Issue.origin` are enumerations in the schema.

### Coordinates and composition

The meshes are glTF files, so they are Y up. The writer converts the transforms of the nodes from
the game's Z-up axes to the same axes (`x` stays, `z` becomes `y`, `y` becomes `-z`), so
`basis` is `gltf_y_up` for everything in the package and an importer only maps glTF's axes to its
own (Blender's glTF importer does exactly that).

A node's local matrix is composed as in glTF: `M = T · R · S`, with the translation `T`, the
rotation `R` from the quaternion and the scale `S`. The matrix of a node in the scene is
`M(parent) · M(node)`, from the root down. A mesh is placed by that matrix times the identity
transform of the root node of its GLB file; the mesh file's own nodes are in the space of that
root. A negative X scale is a mirror; a reader that cannot mirror flips the winding of the mesh.
A shear in the game's transform is not stored.

### Nodes

`nodes` is the scene tree in parent-before-child order.

| Field | Meaning |
| --- | --- |
| `kind` | What the node places: `entity` in version 1. Later minor versions add kinds (containers, objects, lights); a reader skips nodes of a kind it does not know, with their children. |
| `name` | `<port>/<entity>` (the class name for the root); use it as the object name. |
| `path` | The port names from the root joined by `/`; empty for the root. |
| `parent` | Index of the parent in `nodes`; absent for the root. |
| `entity` | `class_name` and `guid` of the Entity; present for the kind `entity`. |
| `mesh` | Id of the mesh drawn at the node; absent when the node draws nothing. |
| `translation`, `rotation`, `scale` | The local transform in glTF axes: a vector, a unit quaternion `[x, y, z, w]` and a vector. |

### Meshes

A mesh is one GLB file with the geometry of one model. Nodes that draw the same model with the
same material library and the same tint palette draw the same mesh, so a gun that appears eight
times is one file with eight referencing nodes: an importer links one mesh data block to eight
objects. The sharing is that of the Assembly ([ADR-0077](adr/0077-scene-meshes-are-shared-by-model-level-and-material-library.md),
[ADR-0080](adr/0080-assembly-materials-take-the-palette-of-their-part-and-share-textures.md)).

| Field | Meaning |
| --- | --- |
| `id`, `file` | The id and the file relative to the package folder (`meshes/m0001.glb`). |
| `geometry` | The geometry file the model was read from, below `Data`. |
| `palette` | The name of the tint palette the mesh's colours were taken from; absent without one. |
| `instances` | How many nodes draw the mesh. |
| `bytes`, `vertices`, `triangles` | Size of the file and of its geometry. |
| `materials` | The materials of the mesh in the order of the glTF materials in its file: `index` is the index in the file's `materials` array, `name` the glTF material's name, and `textures` the ids of the `diffuse`, `normal` and `specular` textures that were written for it, each absent when the material names none or it could not be written. Match by `index`; names can repeat. |

The file is a plain GLB: one scene whose root node (named after the Entity's class name) has the
identity transform, the nodes of the model below it, plain-colour materials with the tint palette
applied and no embedded textures.

### Textures

Each texture the materials of the meshes name is written once, whichever meshes and materials use
it, as the pixels the Archive holds: no palette colour is baked in and the diffuse, normal and
specular maps are not merged. The importer builds the materials from them.

| Field | Meaning |
| --- | --- |
| `id`, `file` | The id and the file relative to the package folder (`textures/t0001.png`). |
| `source` | The texture's path in the Archive, below `Data`. |
| `format`, `bit_depth` | `png` and `8` in version 1. High-range textures (EXR, 16 bits) will be new `format` values in a later minor version. |
| `colour_space` | `srgb` for a diffuse map, `linear` for normal and specular maps (and any data texture): the importer sets Blender's colour space of the image accordingly. The role is that of the first material that names the texture. |
| `alpha` | What the alpha channel holds: `none` (opaque, ignore it), `opacity`, or `smoothness` (the alpha channel of a `_ddna` normal map). |
| `width`, `height` | Pixels of the mip level that was written (`options.texture_mip`, or the smallest the texture has). |

**Normal maps** are written for glTF and OpenGL, Y up (the green channel points up the image):
the writer inverts the green channel of the engine's maps, exactly as the GLB writer does. The
red channel and the reconstructed blue channel are as the Archive has them. A reader that wants
the DirectX convention inverts green again.

### Issues

`issues` lists what was left out or fell back: the issues of the Assembly (`origin` is
`assembly`: parts that could not be resolved, missing geometry, palettes that could not be
resolved, as for `p4kx entity glb`) and textures that could not be loaded or whose file name is
no safe path (`origin` is `texture`, `kind` is `texture` or `texture_path`). Each has `kind`, a
stable code, `path`, `name`, `skipped` (the node is left out of the scene) and `reason`, text for
people that comes partly from the Archive and is untrusted.

## Versions

`version` is `<major>.<minor>`. The major version changes when a field changes its meaning or
disappears, or a rule above changes; the minor version changes when fields or values are added
that a reader can ignore. A reader

- refuses a package with another `format`, `basis` or `unit` than it knows;
- refuses a package with a higher major version and says that it needs a newer reader;
- reads a package with a higher minor version and warns that parts of it may be ignored;
- reads every lower minor version of its major version.

The schema file is the schema of the version in `FORMAT_VERSION`: a change of the Rust types that
changes the schema is a change of the format, and the checked-in schema changes with it. A schema
of a later minor version can contain values (a texture `format`, a node `kind`) that an older
schema rejects; readers follow the rules above, not the older schema.

## Reserved for later versions

These are the directions later issues take; they add fields and files in minor versions and keep
the rules above.

- **Skeletons and animations (#162).** A manifest-level `skeletons` array. A skeleton has an id
  and its bones; a mesh that is skinned and a node that poses a skeleton reference it by that id.
  Animation clips are lazy sidecar files. Version 1 has no skeletons and no skinned meshes.
- **Interiors (#161).** Nodes of other `kind`s (containers, placed objects, lights) next to the
  `entity` nodes; they need no `entity`.
- **Materials (#158).** Layered material descriptions, palettes and decals, as a further field
  of the mesh's materials and further files.
