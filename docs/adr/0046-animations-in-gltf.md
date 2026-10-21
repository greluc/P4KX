# Animations: what the model keeps and how glTF gets it

Decided for issue #107. Evidence: docs/model-animations.md.

## The model

- `Animation::read_caf` reads a `.caf` file, `Dba::read` a `.dba` database into `Animation`s: a
  name (the file stem), the stored path, the frame rate, the end frame, the additive flag and
  `Track`s. A track has the controller ID of a bone and, per channel, `Keys`: strictly increasing
  frames (`u16`) and one value per frame. Rotations are unit quaternions `x, y, z, w` in one
  hemisphere, translations are in metres, both absolute and relative to the parent bone, in
  `CryEngine` axes like `Bone::relative`. Frames stay frames; the writer divides by the frame rate.
- Formats the reader does not know are errors, not guesses: a key of the format `0x81` or `0xC3` is never read
  as a neighbouring format (StarBreaker reads unknown rotations as small tree 48 and unknown
  positions as zeros). An animation of a database that fails is skipped and reported (`Dba::skipped`),
  the others are read; a bad `.caf` fails the file.
- Untrusted input: every offset is checked against the data, counts are checked before anything
  is allocated (4,194,304 keys per animation, 16,777,216 per database, against 425,242 and 6,751,431
  in the Archives), key times must increase, floats must be finite, a rotation needs a length.
  Databases are decoded in parallel with rayon, one animation per task.
- `Animation::bind(bones)` matches tracks to bones by `Bone::controller_id`, the CRC-32 of the
  bone's name as spelled. A bone is moved by the first track that names it; the controller IDs of the
  other tracks come back as `unbound`.
- `ChunkFile::chunk_data_to_end` returns the bytes from a chunk's start to the end of the file
  (#273), and the animation reader uses it for `CafData`, `DbaData`, `AnimInfo` and `DbaMetadata`;
  `chunk_data` stays bound to the computed span. No database of the Archives needs it, but
  the layout does not forbid a pool behind the span and the reader checks every offset anyway.

## The GLB

- `write_glb_animated(model, animations, options, out)` adds glTF animations to the file
  `write_glb` writes; `write_glb` is the call without animations. The summary lists every animation
  asked for with its channels, keys, duration and the controller IDs left out.
- A track becomes at most two channels, `rotation` and `translation`, of the node of its bone, each with
  its own sampler: input in seconds (frame over frame rate, as `f32` with `min` and `max`), output
  `VEC4` or `VEC3` of floats. Values are converted to Y up exactly as the skeleton is (ADR-0039):
  `(x, z, −y)` and `(x, z, −y, w)`. Scale is never animated.
- The node of a bone is the bone's node when the skeleton is written as nodes (a skinned model, a
  skeleton file), otherwise the node `Bone::node` names: the static models with animated parts
  (doors, turrets, landing gear) are moved through the nodes their helper bones drive. A track
  whose bone has no node, or whose node another bone of the animation already moves, is left out
  and reported like a track without a bone.
- Interpolation is `LINEAR`, which glTF defines as a spherical interpolation for rotations, for a channel with
  two keys or more and `STEP` for a channel with one key (a held pose). The files carry no interpolation
  flag; every format stores keys over frames.
- An animation with no channel (no track found its bone) is not written. An additive animation
  (flag bit 0) is skipped by default and listed as skipped with the reason "additive animation
  — needs a base animation": its keys are offsets, glTF cannot say that, and written as absolute
  local poses they collapse the skeleton in a viewer (0.4 to 3.1 rad from the bind pose). It is
  written only when the user names it (a name of `--anim`, or a `.caf` Entry or file of
  `--anim-from`), with a note that the pose needs a base animation. The keys are not composed onto
  the bind pose. 1,475 of the 54,148 `.caf` files and 1,905 of the 15,910 database animations are
  additive.
- The sources `--anim-from` names are explicit: one that is no `.caf` or `.dba` file exits with
  2, one that cannot be read with 3. The animations found in the model's folders that cannot be
  read are listed as unread.
- Animation accessors have no `target` on their buffer views, and times and values of all
  animations are written after the meshes in the one binary chunk. The testkit's checker
  validates animations: distinct channel targets, strictly increasing times with exact `min` and
  `max`, one output per key of the property's type and unit rotations.

## The command line

- `p4kx model glb … --anim <name>` (repeatable, `all` for every animation found) and
  `--anim-from <entry>` (repeatable). Animations are looked for in the `.caf` files and `.dba`
  databases in the model's folder, in the folder at the same place below `Data\Animations`
  (`Data\Objects\x\door.cga` and `Data\Animations\Objects\x\door_open.caf`) and in the entries
  `--anim-from` names. A name is the stem of a `.caf` file or an animation of a database, its file name
  or the end of its path, ignoring case; the path of a `.caf` or `.dba` entry takes all of its animations.
  A `.caf` file is only read when its stem matches a name. A name nothing matches exits with 2.
  `all` leaves out the additive animations and lists them as skipped.
- `p4kx model info` lists the animations found with name, duration, frames, frame rate, tracks,
  tracks that find a bone of the model's skeleton, keys, the additive flag and source, and
  `--json` gives the same (`animations`, `unread_animations`). Names are untrusted file content
  and are printed with their control characters replaced (the sanitiser of the query tables).
- The animations of a character live under `Data\Animations\Characters` and are assigned to the
  skeleton by `.chrparams` files, which are not read: for a character the databases are named
  with `--anim` or `--anim-from`.

## Considered Options

- Interpret the keys as deltas and add them to the bind pose: the data says otherwise
  (the first keys of 547 animations lie near the bind pose, not at the identity).
- Match controllers by the hash of the lower-case name, as Cryengine-Converter describes: 2 of 220
  bones of a skeleton match; the hash of the name as written matches all.
- Bake animations to a fixed sample rate: loses the key times the files hold and multiplies the
  size; viewers interpolate.
- Write `CUBICSPLINE`: nothing in the data supports tangents.
- Write a single-key channel as `LINEAR`: also valid, but `STEP` says that the pose is held.
- Fail a database when one of its animations fails: one animation of unusual format would hide
  hundreds of good ones.
- Search the whole Archive for animations by name: names are not unique (the same file name occurs under `male_v7` and
  `female_v2`) and the search scans tens of thousands of entries.
