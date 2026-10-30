# The skeleton of a character definition is checked, and a creature whose files are missing is a listed gap

Decided for issue #859, on top of ADR-0150 (character definitions in an Assembly), ADR-0224
(a skin attachment is drawn in the bind pose) and ADR-0229 (an item port of a skinned model is
anchored to a bone). Evidence: `crates/p4kx-assembly/tests/real_archive/real_creature_census.rs`
over LIVE.

## Census rule

A creature class is found by structure: an Entity whose main geometry is a character file
(`.cdf`, `.chr` or `.skin`) outside `Objects/Characters/Human`, the folder the skeletons of
`BodyVariant` lie in. `Geometry::is_nonhuman_character_file` is that rule. It also matches
Vanduul and Banu characters and animated props (thrusters, weapons with a rigged part); the
census groups them by file kind and geometry folder, never by class name. Real-Archive tests pick
their Entities from the census, by class order.

## Decision

- **The skeleton file of a `.cdf` is read.** The `Model File` of a definition is checked once per
  distinct definition: it must be in the Archive, at most `MAX_SKELETON_LEN` (16 MiB) and read as
  a model. A missing geometry companion of the skeleton is no problem, a skeleton is read for its
  bones. The result travels with the model as `ProvidedModel::skeleton`.
- **Two notes, not errors.** `skeleton_missing` and `skeleton_unreadable` are Issues with
  `skipped` false: the skin is still the part's model, as before. A key problem stays fatal.
- **A geometry the Archive lacks stays a skipped part.** An Entity whose own geometry file is
  missing or unreadable (`geometry_missing`, `geometry_unreadable`, which names a missing `.chrm`
  or `.skinm`) gives a valid GLB without meshes, or a Blender package without meshes, exit code
  0, and the Issue names the file. No command fails for a part that cannot be drawn; the output
  file is written only when complete.
- **A creature is drawn static.** Its skins are placed in the bind pose, ADR-0224; no joints,
  weights or animations are written for an Entity.

## What the census found in LIVE

Every Entity of the census exports to a valid GLB; no skeleton file of a definition is missing or
unreadable. Causes that stay open:

- Files the Archive does not hold, nothing to fix here: the skeleton of `Tauroch_*`, the skins of
  one Vanduul armour and one hair, the geometry of a few deployables, gadgets and landing gear,
  the library `Objects/Characters/ragdoll_material` of the larger creatures, the XML Loadout of
  `Aloprat`.
- Creatures drawn without joints (#880).
- Item ports of a creature's skin whose helper is a bone of the creature's skeleton, e.g. a
  collar, are placed at the origin (#881).
- Creature base Entities whose body port is unset draw nothing (#882).
