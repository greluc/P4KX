# An Assembly reads the lowest level when none is low enough, lists the ports only the vehicle declares and draws the skin of a character definition

Decided for issue #375. Amends ADR-0028, which rejected a fallback from a missing level of detail,
and ADR-0040. Evidence: `crates/p4kx-assembly/tests/real_archive/real_followups.rs` over ten ships of
LIVE.

## Level of detail

ADR-0028 holds that `--lod 3` of a model without LOD3 is invalid input. ADR-0040 already made an
exception for Assemblies: a model without the requested level is read at the nearest lower one,
and the export counts those models. The exception now covers every case, so that
`ModelProvider::model` and the glTF writer (`nearest_lod`) agree: when the model has no level at
or below the request, as `freelancer_pfx.cgfm` with its LOD4 alone, the provider reads it at its
lowest level instead of reporting it unreadable. A model with no level at all is still
unreadable. The strict selections of the model reader and the `--lod` option of `model glb` keep
ADR-0028; only the Assembly falls back.

## Ports only the vehicle declares

A hardpoint that the vehicle implementation declares and neither a Loadout entry nor the Record
names was no port of the tree, so `--replace` could not attach an Entity to it and a front-end
could not offer it. The tree now lists it after the defined ports, in file order, with the source
`empty`, no definition and no origin. Its constraints are those of the vehicle's item port, its
visibility that of the part. `skipPart` parts stay out. The ports count in `ports` and `empty` of
`entity loadout`.

## Character definitions

A geometry that is a `.cdf` is a `CharacterDefinition`: a skeleton (`Model File`, a `.chr`
without meshes) and an `AttachmentList` of skins (`CA_SKIN`) and bone attachments (`CA_BONE`,
usually a `.cga`). The provider reads the definition (at most 1 MiB), takes the binding of its
first skin attachment, else of its first attachment, and reads that file as the part's model; the
attachment's `Material`, if any, replaces the library the model names. The files of the other
attachments are not drawn and are listed in the issue `attachments_left_out`, once per
definition. A bone attachment needs a second model per part, placed at a bone of the skeleton;
that is not decided here. A definition that names nothing, another definition or a missing file
is `geometry_unreadable`.

## Smaller changes

- **Helper issue.** A port whose helper is searched in a parent without a model no longer
  reports `helper_not_found`; the parent's own `geometry_missing` or `geometry_unreadable` says
  why.
- **Vertex budget.** The distinct files are read in the order of their first part, a wave of
  twice the number of threads (at most 256) at a time in parallel, and no further wave is started
  once the distinct models read hold the budget. The decision is taken between waves from the
  models read, never from the timing inside a wave, so the same Archive gives the same Assembly
  on any number of threads, and hostile data overshoots the budget by one wave of files. A file
  left unread belongs to a part that the part-by-part check, which stays, would refuse too.
- **Placement.** `entity glb --json` lists `placements`: per part its path, how it was placed
  (`root`, `helper`, `port_node`, `origin`), the helper and whether it has a model. A port with no
  helper and no node of its name lands at the parent's origin; it is listed there, not as an issue.
- **Materials.** A Loadout entry names no material: manual entries hold `itemPortName`,
  `entityClassName`, `entityClassReference`, `loadout` and `inventoryContainer`, the items of an
  XML Loadout `portName`, `itemName`, `tag`, `ownership`, `gameFlags` and `entitlementUrnOverride`.
  The library of a part comes from its Entity's geometry, as ADR-0040 says; there is nothing more
  to read.
- **Equal materials.** The glTF writer writes a material once when its JSON is equal, name
  included, so two libraries with the same colours cost one material only when the materials
  carry the same name.
- **Shear.** A transform with a shear is written without it; `docs/cli-json.md` says so.
- **Armour.** Armour and clothing read their pieces through the same provider, so they get the
  level fallback and the reading of character definitions too.
- **Library issue.** `material_library` is reported for the first part that draws with a library.
