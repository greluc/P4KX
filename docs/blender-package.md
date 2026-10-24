# Blender package

A Blender package is a folder that describes an Assembly in files any importer can read: a scene
description, shared GLB parts and PNG textures. It does not depend on Blender's file format, so
new Blender versions do not break it; the Blender add-on of M13 imports it, and so can any other
tool. `p4kx entity package` writes it ([ADR-0083](adr/0083-a-blender-package-is-a-versioned-manifest-with-shared-glb-parts-and-png-textures.md)).

The JSON Schema of `scene.json` is generated from the Rust types of `p4kx-blender` and checked in
as [schemas/blender-package.schema.json](../schemas/blender-package.schema.json); a test fails
when the two differ (`P4KX_BLESS=1 cargo test -p p4kx-blender --test schema` regenerates it).

## Layout

```text
<package>/
  scene.json            the manifest
  parts/p0001.glb       one GLB per distinct part
  parts/p0002.glb
  textures/t0001.png    one PNG per distinct texture
```

- The folder holds nothing but these. Every file name is an id made by the writer (`p` or `t` and
  four digits at least); a name from the Archive never becomes a file name, so no name can lead
  out of the package. Part ids number the parts from 1 in the order of their first node. Texture
  ids number the textures in the order of their first use, and a texture that could not be
  written leaves a gap, so ids are not always consecutive.
- `scene.json` is written last. A folder with a `scene.json` whose `format` is
  `p4kx-blender-package` is a complete package.

## Writing a package

`p4kx entity package <archive> <entity> --out <folder>`. The Loadout, the parts, `--lod`,
`--exclude` and `--replace` are those of `p4kx entity glb`; `--texture-mip <n>` picks the mip of
every texture and `--no-vertex-colours` leaves the vertex colours out of the parts.

- The folder must not exist, or be empty, or hold a package an earlier run wrote (only
  `scene.json`, `parts` and `textures` in it); the earlier package is then replaced. Any other
  folder or file is refused with exit code 2 and left alone.
- A folder inside a Star Citizen installation is refused with exit code 2.
- Everything is written into a temporary folder `<name>.p4kx-<process id>.part` next to the
  target, which replaces the target once the package is complete. After an error or a Ctrl+C the
  temporary folder is removed and an earlier package is still there. A second Ctrl+C ends the
  process at once and can leave the temporary folder behind.

## `scene.json`

All lengths are metres, all transforms are relative to the parent node. Writers add fields in
minor versions, so a reader ignores fields it does not know. Absent optional fields are not
written (there are no `null`s).

| Field | Meaning |
| --- | --- |
| `format` | Always `p4kx-blender-package`. |
| `version` | `<major>.<minor>` of the package format; `1.0` today. |
| `generator` | `name` (`p4kx`) and `version` of the program that wrote the package. |
| `source` | `channel` (the Channel folder, e.g. `LIVE`), `build` (`branch`, `version`, `public_patch`, `change_number`, `build_id` of the Archive's build manifest) and `entity` (`class_name`, `guid`). `channel` and `build` are absent when unknown. |
| `coordinates` | `basis` is `gltf_y_up` and `unit` is `metre`. |
| `options` | What the package was made with: `lod`, `texture_mip`, `vertex_colours`, `exclude` (ports) and `replace` (`port`, `entity`). |
| `nodes` | The scene hierarchy, see below. |
| `parts` | The GLB parts, see below. |
| `textures` | The PNG textures, see below. |
| `issues` | What could not be resolved or written. |

### Coordinates

The parts are glTF files, so they are Y up. The writer converts the transforms of the nodes from
the game's Z-up axes to the same axes (`x` stays, `z` becomes `y`, `y` becomes `-z`), so
`basis` is `gltf_y_up` for everything in the package and an importer only maps glTF's axes to its
own (Blender's glTF importer does exactly that). Multiply the transforms of a node and its
parents, then the root node of the part's GLB file, which has the identity transform.

### Nodes

`nodes` is the Assembly's tree in parent-before-child order. A node is one Entity placed at a port
of its parent:

| Field | Meaning |
| --- | --- |
| `name` | `<port>/<entity>` (the class name for the root); use it as the object name. |
| `path` | The port names from the root joined by `/`; empty for the root. |
| `parent` | Index of the parent in `nodes`; absent for the root. |
| `entity` | `class_name` and `guid` of the Entity. |
| `part` | Id of the part drawn at the node; absent when the Entity has no usable geometry. |
| `translation`, `rotation`, `scale` | The local transform in glTF axes: a vector, a unit quaternion `[x, y, z, w]` and a vector. A negative X scale marks a mirror. A shear is not stored. |

### Parts

A part is one GLB file with the meshes of one model. Nodes that draw the same model with the same
material library and the same tint palette draw the same part, so a gun that appears eight times
is one file with eight referencing nodes: an importer links one mesh data block to eight objects.
The sharing is that of the Assembly ([ADR-0077](adr/0077-scene-meshes-are-shared-by-model-level-and-material-library.md),
[ADR-0080](adr/0080-assembly-materials-take-the-palette-of-their-part-and-share-textures.md)).

| Field | Meaning |
| --- | --- |
| `id`, `file` | The id and the file relative to the package folder (`parts/p0001.glb`). |
| `geometry` | The geometry file the model was read from, below `Data`. |
| `palette` | The name of the tint palette the part's colours were taken from; absent without one. |
| `instances` | How many nodes draw the part. |
| `bytes`, `vertices`, `triangles` | Size of the file and of its meshes. |
| `materials` | The materials of the part: `name` is the name of the glTF material in the file, and `textures` has the ids of the `diffuse`, `normal` and `specular` textures that were written for it, each absent when the material names none or it could not be written. `smoothness_in_alpha` is true when the alpha channel of the normal map holds the smoothness. |

The file is a plain GLB: one scene whose root node (named after the Entity's class name) has the
identity transform, the nodes of the model below it, plain-colour materials with the tint palette
applied and no embedded textures. The mesh of the file is in the space of the root node.

### Textures

Each texture the materials of the parts name is written once, whichever parts and materials use
it, as the pixels the Archive holds: no palette colour is baked in and the diffuse, normal and
specular maps are not merged. The importer builds the materials from them.

| Field | Meaning |
| --- | --- |
| `id`, `file` | The id and the file relative to the package folder (`textures/t0001.png`). |
| `source` | The texture's path in the Archive, below `Data`. |
| `format`, `bit_depth` | `png` and `8` in version 1. High-range textures (EXR, 16 bits) will be new `format` values in a later minor version; a reader skips textures with a format it does not know. |
| `width`, `height` | Pixels at the mip level of `options.texture_mip`. |

A normal map of a `_ddna` texture carries the smoothness in its alpha channel; the RGB channels
are the decoded normal with its Z reconstructed.

### Issues

`issues` lists what was left out or fell back: the issues of the Assembly (`origin` is
`assembly`: parts that could not be resolved, missing geometry, palettes that could not be
resolved, as for `p4kx entity glb`) and textures that could not be loaded or whose file name is
no safe path (`origin` is `texture`, `kind` is `texture` or `texture_path`). Each has `kind`, a
stable code, `path`, `name`, `skipped` (the part is left out of the scene) and `reason`, text for
people that comes partly from the Archive and is untrusted.

## Versions

`version` is `<major>.<minor>`. The major version changes when a field changes its meaning or
disappears, or a rule above changes; the minor version changes when fields or values are added
that a reader can ignore. A reader

- refuses a package with another `format`;
- refuses a package with a higher major version and says that it needs a newer reader;
- reads a package with a higher minor version and warns that parts of it may be ignored;
- reads every lower minor version of its major version.

The version of the schema file is the version of this document: a change of the Rust types that
changes the schema is a change of the format, and the checked-in schema must change with it.

## What a later version adds

Layered materials with their parameters, palettes and decals (#158), lights, skeletons and
animations, and the Interior and Character packages build on this layout. They add fields and
files and keep the rules above.
