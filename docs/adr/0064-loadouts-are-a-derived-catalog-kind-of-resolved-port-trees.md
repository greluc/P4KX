# Loadouts are a derived catalog kind of resolved port trees

Decided for issue #393. Builds on ADR-0054 (the catalog grows additively) and ADR-0012 (the
Default Loadout).

The Basetool wants the ports of every ship and port-carrying item and the default fitting from the
Archive. The owner decided that loadouts are a kind of the one catalog, not a second Export; the
size is handled by `.json.zst`.

## Decision

- **A derived kind `loadouts`.** Like `paints` it selects no Records of its own: it takes the
  entries of `ships` and the entries of `items` that may have ports (their Record has a
  `SItemPortContainerComponentParams` with ports or a `SEntityComponentDefaultLoadoutParams`).
  Every ship gets an entry; an item gets one only when its tree holds a port. With a `--filter` the
  kind is partial like every other. It keeps `schemaVersion` 1.
- **One resolved tree per entity.** The entry of an entity is its Default Loadout tree
  (`LoadoutResolver`, the walk of `EntityNode::loadout`): each port with the Entity attached to it
  and, below it, the ports of that Entity under the Loadout that attached it. A ship's Loadout
  overrides the default Loadout of a turret per port, so the ship's tree cannot be rebuilt from the
  turret's own entry; the tree is therefore kept per entity, and an item with ports has its own
  entry as well. The `source` of each port (`loadout`, `override`, `port_default`, `unset`) says
  which of them supplied it.
- **References stay links.** An attached Entity is its `itemGuid` and `itemClassName`; GUID
  references close within the file for entries of `items` and `ships`. An Entity outside both (a
  development leftover, an Entity of another folder) keeps its GUID and is counted in the summary
  (`outside`); an attachment that cannot be resolved keeps the GUID or class name the Loadout
  names, with the code of the error in `unresolved`.
- **Docked vehicles** (`NOITEM_Vehicle…`) are the `itemGuid` of a ship, marked `dockedVehicle` and
  not expanded (#305). The docked ship has its own entry.
- **Ports of the vehicle implementation.** A port the Record does not define takes its sizes,
  types, flags and tags from the `ItemPort` of the vehicle implementation, with the Modification
  applied (ADR-0050); an `ItemPort` that no port of the tree names is listed last with the source
  `vehicle` and holds nothing. Where both define a port, the Record's definition wins; the
  visibility always comes from the vehicle implementation (#289).
- **Tags are names and GUIDs.** `portTags` and `requiredTags` are the words of the stored value,
  split at white space and otherwise unchanged, like `tags` of an item. Beside them the owner asked
  for the tag GUIDs, so `portTagGuids` and `requiredTagGuids` (and `tagGuids` on every item) hold,
  per name and in the same order, the `guid` of the tag the tag tree has under that name. A name
  resolves only when exactly one tag has it (ignoring ASCII case); an unknown or ambiguous name
  gives `null`, so the lists stay aligned and no GUID is guessed. The Basetool can match by name
  or by GUID. The cost is one 36-character string or `null` per tag word, which compresses well
  because the tag set is small. Flags are words without a leading `$`.
- **No texts.** The kind carries identifiers and fact values only: names, sizes, types, flags,
  tags, GUIDs, class names and the localisation key of a port's display name (`null` for a
  placeholder key). No description, no geometry, no media (the owner's legal review of 2026-10-20).
- **Closed sets are strings** (ADR-0054): `entityKind`, `source` and `visibility` document their
  values in the schema and keep the type string.

## Consequences

- The ship entries repeat the trees of the items they carry. The repetition compresses well
  (`.json.zst`) and keeps every entry self-contained for the import.
- The catalog build resolves every ship and every port-carrying item with one `LoadoutResolver`
  per worker; the measurements are in `docs/basetool-catalog.md`.
- A port is not identified across entries: a consumer that wants the ports of a turret under a
  specific ship reads the tree of that ship.
