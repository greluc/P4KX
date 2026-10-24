# Blender package

A Blender package is a folder that describes an Assembly in files any importer can read: a scene
description, shared GLB meshes and PNG textures. It does not depend on Blender's file format, so
new Blender versions do not break it; the [Blender add-on](../blender_addon/README.md) imports it
([ADR-0093](adr/0093-the-blender-add-on-is-an-extension-that-reads-and-validates-a-package-without-blender.md)),
and so can any other tool. `p4kx entity package` writes it ([ADR-0083](adr/0083-a-blender-package-is-a-versioned-manifest-with-shared-glb-meshes-and-png-textures.md)),
and `p4kx character package` writes a Character in the same format, see
[Characters](#characters).

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
`--interior` adds the Entity's Interior as for `entity glb --interior` (ADR-0086), see
[Interiors and lights](#interiors-and-lights).

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
| `version` | `<major>.<minor>` of the package format, matching `^[0-9]+\.[0-9]+$`; `1.3` today. |
| `generator` | `name` (`p4kx`) and `version` of the program that wrote the package. |
| `source` | `channel` (the Channel folder, e.g. `LIVE`), `build` (`branch`, `version`, `public_patch`, `change_number`, `build_id` of the Archive's build manifest) and `entity` (`class_name`, `guid`). `channel` and `build` are absent when unknown. |
| `coordinates` | `basis` is `gltf_y_up` and `unit` is `metre`. A reader refuses a `basis` or `unit` it does not know. |
| `options` | What the package was made with: `lod`, `texture_mip`, `vertex_colours`, `exclude` (ports) and `replace` (`port`, `entity`). |
| `nodes` | The scene hierarchy, see below. |
| `meshes` | The GLB meshes, see below. |
| `textures` | The PNG textures, see below. |
| `materials` | The material descriptions, see "Materials". Added in 1.1; absent in 1.0 packages. |
| `palettes` | The tint palettes, see "Palettes". Added in 1.1; absent in 1.0 packages. |
| `issues` | What could not be resolved or written. |

`format`, `basis`, `unit`, `Texture.format`, `colour_space`, `alpha`, `Node.kind`,
`Issue.origin`, `usage`, `alpha_mode` and the `palette` of a material or layer are enumerations in
the schema.

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
| `kind` | What the node places: `entity`, or in an Interior (version 1.2) `container`, `object` or `light`, or (version 1.3) `character`. Later minor versions add kinds; a reader skips nodes of a kind it does not know, with their children. |
| `name` | `<port>/<entity>` (the class name for the root) for an `entity`, the name from the Interior's files for the other kinds; use it as the object name. |
| `path` | The port names from the root joined by `/`, below `Interior` the names of the nodes joined by `/`; empty for the root. |
| `parent` | Index of the parent in `nodes`; absent for the root. |
| `entity` | `class_name` and `guid` of the Entity; present for the kind `entity`, absent for the others. |
| `light` | The light, see below; present for the kind `light`. |
| `mesh` | Id of the mesh drawn at the node; absent when the node draws nothing. |
| `translation`, `rotation`, `scale` | The local transform in glTF axes: a vector, a unit quaternion `[x, y, z, w]` and a vector. |

### Interiors and lights

With `--interior` the root Entity gets one more child, the node `Interior` (kind `container`,
path `Interior`). Below it, in the order of the Entity's Record, hang the containers the Entity
names, placed at their bones as in ADR-0086, and below each container the nodes ADR-0081 and
ADR-0087 describe:

| Kind | What it is | `mesh` | `entity` |
| --- | --- | --- | --- |
| `container` | The group `Interior`, a container, or a `.soc` file a container streams. It draws nothing and only gives its children a place. | absent | absent |
| `object` | A model a `.soc` file places, or a prop. The mesh is absent when the model could not be read; the issue says why. | the mesh drawn | absent |
| `light` | A punctual light. | absent | absent |

Objects share meshes with each other and with the Entity's parts as everything else does (same
model, material library and palette), and an Interior model has no tint palette: it is drawn with
plain colours or its textures. Paths and names come from the Archive and are
untrusted text.

A `light` node has a `light` object:

| Field | Meaning |
| --- | --- |
| `kind` | `point`, `spot` or `directional`. Other engine types (`Ambient`, `Planar`) are no nodes; the issues list them as `light_unsupported`. |
| `colour` | Linear red, green and blue, each in 0 to 1. The engine's colour is read as sRGB; with a colour temperature it is multiplied by the colour of a black body (ADR-0087). |
| `intensity` | The engine's value, not converted: its unit is not documented, so it only compares lights of one package. Never negative. |
| `range` | Metres beyond which the light has no effect; absent when the engine gives none. |
| `inner_cone`, `outer_cone` | Radians from the axis, for the kind `spot` only: where the falloff begins (0 today) and where the light ends (half of the projector's opening angle, at most a quarter turn). |

**Direction.** The node's transform is already turned so that the light shines along the node's
local -Z axis, as glTF and Blender lights do (the engine's lights shine along +X; the writer
applies the same rotation as `entity glb`). An importer needs no correction beyond its axis
conversion.

**Mapping in the Blender add-on** (approximate, because the intensity has no unit): `point`,
`spot` and `directional` become a point, spot and sun light; the colour is carried over; a point
or spot light gets 10 W per unit of intensity and a sun 1 per unit, both times the import
option *Light strength*; a spot's size is `2 * outer_cone` between 1 and 180 degrees and its
blend `1 - inner_cone / outer_cone`; a `range` becomes the light's custom distance. The mapping is
`blender_addon/p4kx_importer/lights.py`.

### Characters

`p4kx character package <archive> --out <folder>` writes a Character: the head of a CHF file
(`--chf`), pieces of armour and clothing (`--piece`, `--loadout`) on the default body of the CHF's
or the chosen variant (`--body`), with expressions (`--expression`, `--no-expressions`) and
animations (`--anim`, `--anim-from`), all as `chf glb` and `armour glb` take them
([ADR-0107](adr/0107-a-character-is-written-as-one-rigged-glb-mesh-behind-a-character-node.md)).
`--materials textures` (the default) embeds the textures at `--texture-mip` in the GLB,
`--materials colours` writes plain colours. The output rules are those of `entity package`.

A Character package (version 1.3) is the folder above with one mesh and no texture files:

- `nodes` has one node of the kind `character`, named after the Character (`--name`, else the
  CHF's file name, else the body variant), at the identity transform, with `mesh` `m0001`. Its
  `source.entity.class_name` is the same name and `guid` is empty.
- `meshes/m0001.glb` is the Character as `armour glb` writes it: the bones as nodes, one glTF skin
  over them, a skinned mesh per part named after it, the sparse morph targets of the head named by
  `extras.targetNames` and the animations as glTF animations. Its materials are in the file; the
  mesh lists no `materials` and `textures`, `materials` and `palettes` are empty.
- The mesh has a `rig` that says what the file holds:

| Field | Meaning |
| --- | --- |
| `body` | The body variant, `male` or `female`. |
| `skeleton` | The skeleton file the bones come from, below `Data`. |
| `bones`, `skinned_meshes`, `morph_targets` | How many bones, skinned meshes and morph targets the file has. |
| `embedded_textures` | How many PNG images the GLB embeds. |
| `parts` | The skins worn on the skeleton, the body first: `name` (the node's name in the file), `origin` (`body` or `piece`), `file` (the skin in the Archive) and `morph_targets`. |
| `animations` | The animations in the file: `name`, `duration_seconds`, `additive` (its keys are offsets to a pose, which glTF cannot say), `channels` and `keys`. |

Pieces that cannot be fitted to the body and animations that fit nothing are `issues` with the
kinds the `armour glb` report uses (`origin` `assembly`).

The add-on imports a `character` node's mesh once for that node, not as an instance, so that the
armature can be posed: the armature, the skinned meshes below it, the shape keys of the head and
the actions are objects and data of the scene, with the armature below the node's empty. The
library entry point is `p4kx_blender::write_character_package`.

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
| `palette_id` | The id of that palette in `palettes` (1.1); absent without one. |
| `instances` | How many nodes draw the mesh. |
| `bytes`, `vertices`, `triangles` | Size of the file and of its geometry. |
| `rig` | The skeleton, parts, morph targets and animations of a Character's mesh (1.3), see "Characters"; absent for a static mesh. |
| `materials` | The materials of the mesh in the order of the glTF materials in its file: `index` is the index in the file's `materials` array, `name` the glTF material's name, and `textures` the ids of the `diffuse`, `normal` and `specular` textures that were written for it, each absent when the material names none or it could not be written, and, from 1.1, `material`, the id of the material's description in `materials`, absent for the engine's default material. Match by `index`; names can repeat. |

The file is a plain GLB: one scene whose root node (named after the Entity's class name) has the
identity transform, the nodes of the model below it, plain-colour materials with the tint palette
applied and no embedded textures.

From 1.1 every glTF material in the file has `extras` `{"p4kx_index": <index>}`, its index in the
file's `materials` array, the `index` of the matching entry in the manifest's `materials` list of
the mesh. Blender's glTF importer creates materials in first-use order and renames duplicates
(`.001`), so an importer matches materials by this custom property, not by name. `p4kx entity
glb` files do not carry it.

### Textures

Each texture the materials of the meshes name is written once, whichever meshes and materials use
it, as the pixels the Archive holds: no palette colour is baked in and the maps are not merged.
From 1.1 these are the files of every texture slot of every described material, layer materials
included (diffuse, normal, specular, blend, mask, detail, wear and the rest); 1.0 packages hold
only the diffuse, normal and specular maps of the meshes' materials. The importer builds the
materials from them; the `textures` of a material description say which slot uses which file.

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

### Materials

`materials` (1.1) describes every material the meshes are drawn with, and every material their
layers name, as the game's `.mtl` files define it: the inputs of a renderer's material, not a
renderer's nodes. A description is written once, however many meshes use it; a mesh's
`materials[].material` and a layer's `material` reference it by id (`x0001`). Colours are linear
and exactly those of the file: **no palette is applied**; the description says which palette
entry the material takes and the mesh says which palette it has. A reader that cannot interpret a
shader still has every value the file holds.

| Field | Meaning |
| --- | --- |
| `id`, `name` | The id, and the name in the file (empty when it has none). |
| `library`, `index` | The material library (path below `Data`, no extension, `/` separators) and the position in its sub-material list, which is the material ID of the submeshes. `index` is absent when the file is one material that serves every ID (most layer files are). A library the model names with a bare name is looked for beside the model first, so two models in different folders do not share a description for it. |
| `shader` | The shader name, e.g. `Illum`, `HardSurface`, `Layer`, `MeshDecal`, `GlassPBR`. Unknown shaders are written as they are. |
| `gen_mask` | The shader generation mask split at `%` (`NORMAL_MAP`, `USE_TINT_PALETTE`, `SECOND_UVS`, ...): which features the shader uses. |
| `surface_type` | The physical surface, e.g. `metal_dense`. |
| `flags`, `two_sided`, `no_draw` | `MtlFlags` and the two flags the writer decodes. |
| `diffuse`, `specular`, `emissive` | Linear colours. `emissive` counts times `glow`. |
| `shininess`, `opacity`, `alpha_test`, `glow` | Scalars of the file (`shininess` from 0 to 255). |
| `alpha_mode` | `opaque`, `mask` (`alpha_test` above 0) or `blend` (`opacity` below 1, or a glass or decal shader): derived from the fields above. |
| `palette` | `primary`, `secondary`, `tertiary` or `glass`: the palette entry the material takes (the first layer with a `PaletteTint` from 1 to 3, or the tint-palette glass colour). Absent when it takes none. |
| `default_palette` | The Record path of the library's default palette. |
| `textures` | The texture slots in file order, see below. |
| `layers` | The layers in file order, see below. |
| `parameters` | The shader parameters (`PublicParams`) as `name` and `value`, in file order. Decal stencil colours and opacities, wear and dirt blends, damage tiling, parallax and every parameter this writer has never heard of are here, as text. |
| `attributes` | The attributes of the material element that no field above holds, as `name` and `value`. |
| `elements` | The child elements of the material that are none of textures, layers, parameters or sub-materials, as `name`, `attributes` and `children`, to a depth of 8. |

A **texture slot** has `slot` (`TexSlot1`, ...) and `file` (the name in the file, or an engine
placeholder such as `$TintPaletteDecal` or `$RenderToTexture`, which has no file), `usage`, and
`texture`, the id in `textures`, absent for a placeholder and for a texture that could not be
written. `usage` is derived from the suffix of the file name (`_ddna` and `_ddn` normal, `_spec`
specular, `_mask`/`_opac` mask, `_blend` blend, `_detail` detail, `_wear` wear, `_ao` occlusion,
`_displ`/`_disp` displacement, `_gloss` gloss; `_id`, `_scalp`, `_dir`, `_dirm` and `_hal` are
`other`, even in `TexSlot1`) and, for other files, the slot: `TexSlot1` is `diffuse`, anything
else `other`. `attributes` (`TexType`, `Used`, ...) and `elements` (the `TexMod`
child with the tiling `TileU`/`TileV`, the offset `OffsetU`/`OffsetV` and the rotation of the
slot's UVs) are as in the file. Which UV set a slot samples is decided by the shader and its
`gen_mask`; the package does not guess it.

A **layer** is another material blended into the surface: `name`, `path` (the layer file, with its
extension), `sub_material` (`Submtl`, the name of the sub-material in a layer file that has a
list; absent for none), `tint_colour` (`TintColor`), `wear_tint`, `uv_tiling`, `gloss_mult` (each
absent when the file gives none or it is malformed), `palette` (the entry of its `PaletteTint`),
`attributes` (all attributes of the layer in file order, the ones above included, with
`WearGloss`, `HeightBias`, `HeightScale` and the rest), and `material`, the id of the layer file's
description in `materials`. A layer file is a material like any other, so it has its own
textures, shader and parameters, and its own layers. Files that name each other, or themselves,
refer to each other's descriptions. A layer whose file is missing, unreadable (a path that leaves
`Data` included) or lacks the named sub-material has no `material` and is listed in `issues`.
Layers are followed through 8 files; the layers of the eighth stay without a `material` and one
issue says so.

The reader keeps at most 64 layers and 4096 raw elements per material (the attribute values are
bounded by the 16 MiB cap on a material file). What is cut off is listed in `issues` (`origin`
`material`, `kind` `material`), and the rest of the file is read.

Decals and wear are not separate objects in the game's files; they are materials: a decal is a
material with the shader `MeshDecal` (its stencil colours in `parameters`, the placeholder
`$TintPaletteDecal` in a slot), wear is `wear` textures, `WearTint`/`WearGloss` on layers and
`Wear*` parameters.

### Palettes

`palettes` (1.1) lists each tint palette once. A mesh's `palette_id` references it. A palette has
`id` (`p0001`), `name`, and the entries `primary`, `secondary` and `tertiary`, each with `tint`,
`specular` (linear colours) and `glossiness` (0 to 255), and `glass`, the glass colour. An importer
applies a palette by taking the entry a material's or layer's `palette` names: the `tint` replaces
the diffuse colour (or multiplies a layer's, with its `tint_colour`) and so on, as the game does;
the package leaves the decision to the importer.

### Issues

`issues` lists what was left out or fell back: the issues of the Assembly (`origin` is
`assembly`: parts that could not be resolved, missing geometry, palettes that could not be
resolved, as for `p4kx entity glb`), textures that could not be loaded or whose file name is
no safe path (`origin` is `texture`, `kind` is `texture` or `texture_path`) and, from 1.1, layers
whose material file is missing, unreadable or lacks the named sub-material (`origin` is
`material`, `kind` is `layer_material`, `path` the layer's `Path`, `name` the material that has
the layer). Each has `kind`, a
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

Version history: **1.0** is the first format; **1.1** adds the material descriptions, the palettes
and the glTF material indices in the meshes; **1.2** adds the node kinds `container`, `object` and
`light` and the node field `light` (`entity package --interior`). A reader of 1.1 skips such nodes
with their children. **1.3** adds the node kind `character` and the mesh field `rig`
(`character package`); a reader of 1.2 skips the node and warns about the minor version.

The schema file is the schema of the version in `FORMAT_VERSION`: a change of the Rust types that
changes the schema is a change of the format, and the checked-in schema changes with it. A schema
of a later minor version can contain values (a texture `format`, a node `kind`) that an older
schema rejects; readers follow the rules above, not the older schema.

## Reserved for later versions

These are the directions later issues take; they add fields and files in minor versions and keep
the rules above.

- **Skeletons and animations.** Version 1.3 carries them in the GLB of a Character and lists them
  in the mesh's `rig` ([ADR-0107](adr/0107-a-character-is-written-as-one-rigged-glb-mesh-behind-a-character-node.md)).
  A manifest-level `skeletons` array, clips as sidecar files and rigs on the meshes of an Entity
  (a ship's moving parts) are not decided.
- **Node trees of the add-on (#160).** Nothing is reserved: the descriptions of 1.1 are the
  inputs it builds from ([ADR-0100](adr/0100-the-add-on-builds-material-node-trees-from-the-package-descriptions-by-a-pure-mapping.md)).

A reader of a 1.0 package finds no `materials` and `palettes` (read them as empty) and builds
from the glTF materials of the meshes and the `diffuse`, `normal` and `specular` textures.
