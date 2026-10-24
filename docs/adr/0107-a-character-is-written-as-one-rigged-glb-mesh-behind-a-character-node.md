# A Character is written as one rigged GLB mesh behind a character node

Decided for issue #162 (M13.6, Blender package, #13). Builds on ADR-0083 (the package), ADR-0096
(format 1.2), ADR-0093 and ADR-0100 (the add-on), ADR-0098 (animations of a Character) and ADR-0099
(the Character builder).

## Decision

**Format 1.3, additive.** A Character needs skinned parts, morph targets and animations, and none
of them changes the meaning of a field of 1.2. They are a new node kind and a new optional field,
so they are a minor version: a reader of 1.2 skips a node of an unknown kind with its children and
warns about the minor version, as ADR-0096 set out. `FORMAT_VERSION` is `1.3` and the add-on's
`SUPPORTED_MINOR` is 3.

**The GLB carries the rig, the manifest lists it.** A Character is one mesh, `meshes/m0001.glb`,
the file `Armour::write_glb_animated_with` writes for `chf glb` and `armour glb`: the bones of the
body as nodes, one glTF skin over them, a skinned mesh per part, the sparse morph targets of the
head (ADR-0091) and the animations, body and facial (ADR-0098). The package does not rebuild any
of that: the library function takes the `Armour` and its `CharacterAnimations` that the exports
already take and calls the same writer, so the file and the GLB export cannot drift apart. The
animation budget `MAX_ANIMATION_VALUES` applies as before, because the animations are fitted to
the Character by `Armour::animate` before the package sees them.

The manifest does not copy the glTF. The mesh gets an optional `rig` (`body`, `skeleton`, `bones`,
`skinned_meshes`, `morph_targets`, `embedded_textures`, `parts`, `animations`) that says what the
file holds, so a reader can show, check or choose without parsing glTF, and a test compares it with
the file. The manifest-level `skeletons` array and the lazy sidecar clips that ADR-0083 reserved
are not used: Blender's glTF importer builds the armature, the shape keys and the actions from the
GLB itself, and a second description of the same bones would be a second source of truth. A
different consumer that wants clips as separate files would add them as a later minor version.

**A new node kind `character`.** The package has one node of that kind below no parent, named after
the Character, at the identity transform, which draws the mesh. Its `source.entity` carries the
Character's name as `class_name` and an empty `guid`, because a Character is not an Entity of the
Archive. The mesh is never shared: one Character is one file, and a second Character in the same
package would be a second mesh.

**Materials stay in the GLB.** The materials of the rigged writer are plain colours or, with
`--materials textures` (the default of `character package`), textures embedded at the chosen mip.
The package therefore has no texture files, `textures`, `materials` and `palettes` are empty and
the mesh lists no `materials`. Building the layered node trees of ADR-0100 for skin, hair and eyes
is a follow-up: the rigged writer merges equal materials across parts and the manifest would have
to reproduce that order, and the head and skin shaders are not among the families ADR-0100 maps.

**The add-on imports a Character once, not as an instance.** An armature in an instanced
collection cannot be posed or animated, so the add-on imports a rigged mesh into the node's own
collection, makes the top-level objects of the import children of the node's empty and leaves the
materials the glTF importer made. The import reports the armatures, the shape keys (the morph
targets, without the basis) and the actions it created. Static meshes of the same package are
instanced as before.

**One library entry point.** `p4kx_blender::write_character_package` takes a `CharacterScene` (name,
`Armour`, `CharacterAnimations`), a texture loader, the GLB options, the `PackageInfo` and the
output folder, and has the staging, guard, replace rule and cancellation of `write_package`. A
front-end that has built the Armour, as the GUI's Character builder does (ADR-0099), calls it next
to `write_package`.

**The command.** `p4kx character package <archive> --out <folder>` takes what the Character
export takes: `--chf` (the CHF decides the body, `--body` is refused beside it), `--body`,
`--piece`, `--loadout`, `--lod`, `--materials`, `--texture-mip`, `--no-vertex-colours`,
`--expression`, `--no-expressions`, `--anim` and `--anim-from`. Without a CHF, a piece or a
Loadout it writes the bare body. Pieces that cannot be fitted and animations that fit nothing are
issues of the manifest. A head that could not be shaped or expressions that could not be made end
the command with the exit code of `chf glb` after the package is written.

## Consequences

- A package of format 1.3 with a Character is read by the add-on of this release; the 1.2 add-on
  warns and skips the Character node. Packages without a Character are unchanged.
- The Character's materials are not editable through the package description of ADR-0100 yet.
- The GUI's "Export as Blender package" (#163) can offer the Character builder's Armour through
  `write_character_package` without a new code path.
