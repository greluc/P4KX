# A creature without a body port takes the first alternative of its generated Loadout

Decided for issue #882, on top of ADR-0255 (a creature without files is a listed gap) and
ADR-0275 (the skins of a creature are written bound to the skeleton of its root). Evidence: the
LIVE Archive, `Quasigrazer_Tundra` and `Aloprat`, and the census of
`real_creature_census.rs`.

## What the data says

Creature base Entities (`libs/foundry/records/actor/actors/*`) whose root geometry is a skeleton
(`.chr`) draw nothing when their body port is unset. The census of #859 listed `Aloprat` and the
`Quasigrazer*` variants.

- **The Quasigrazers have a choice, and the Assembly did not read it.** Their default Loadout is
  `SItemPortLoadoutRandomGeneratedParams`, not manual entries: it names `SLoadoutAssortment`
  Records (`libs/foundry/records/loadoutkits/pu_loadoutbundles/creature_loadoutbundle/*`). An
  assortment has groups (`GroupedLoadouts`, e.g. `Body` and `TorsoAttachments`), each with
  alternative Loadouts, here XML Loadouts of `Scripts/Loadouts/Character/Creatures/Qausi_graser`.
  The game picks one alternative per group at random. Those XML files are in the Archive and put
  the body skin (`m_body_quasigrazer_*`) on `Body_ItemPort` and the eggs on the egg ports.
- **`Aloprat` has no choice in the Archive.** Its base Loadout is the XML file
  `Scripts/Loadouts/Character/Creatures/aloprat.xml`, which the Archive lacks, and no Entity
  Record holds an Aloprat body skin (`aloprat.skin` is a file nothing names). The statue
  `AnimatedCharacterStatue_Aloprat` names its body by a Reference to a Record the Archive lacks.
  The data is not there: a missing file, not a missing reader.

## Decision

- **A default Loadout of type `SItemPortLoadoutRandomGeneratedParams` is read.** It is the
  Loadout `LoadoutSpec::Assortments`: the GUIDs of its `LoadoutAssortments`, read when the
  Loadout is resolved.
- **The first alternative of every group is taken**, the groups in order and the assortments in
  order, laid over each other like the layers of an outfit (`overlay`). The game's pick is
  random; the first alternative is the deterministic default, so an export is reproducible. The
  `Requirement` of an assortment or group is not evaluated.
- **An assortment that is missing or cannot be read is a Loadout issue** of the Entity, and the
  other assortments still apply.
- **A creature whose Loadout could not be read lists why.** The Assembly of an Entity whose root
  geometry is a creature file (ADR-0275, `is_creature_path`) reports the issues of its own
  Loadout as `loadout_unreadable` on the root, with `skipped` false: `Aloprat` says that its XML
  Loadout is not in the Archive. Other Entities are unchanged. Exit codes are those of ADR-0255:
  the root geometry is present, so the export succeeds with an empty body.
- **The census counts them.** It prints the number of skeleton-rooted Entities that draw nothing
  with the codes of the issues that explain each, and the real-Archive test fails for one without
  a reason. The Entities are found by the census rule, not by name.

## Consequences

In LIVE all six Quasigrazers draw their body. Left without a body, each with its reason: `Aloprat`
(the XML Loadout and the statue's Record are not in the Archive) and the five `Tauroch_*`
(their skeleton file is not in the Archive, ADR-0255). Which alternative the game picks stays
unknown to an export; a Loadout of its own, per port, overrides it.
