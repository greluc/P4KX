# Cosmetic variants are grouped by structural equality

Decided for issue #325. Builds on ADR-0054 (the catalog grows additively), ADR-0050 (the vehicle
implementation is patched by the Modification) and ADR-0065 (item stat blocks).

The Basetool matches hangar ships by class name and collapses variants when it creates ships and
items from the catalog. Grouping ships by `vehicleDefinition` alone both over-merges (separately
sold ships such as the Cutlass Black and Steel share one definition) and under-merges (the same
ship under two definitions).

## Decision

- **`variantOf` on `ships` and `items`.** The GUID of the group's base, `null` for the base and for
  an entity alone in its group. Every value resolves to an entry of the same kind. Version 1 stays
  open and additive.
- **Cosmetic means structurally equal without the loadout.** Ships: equal `vehicleDefinition`,
  equal vehicle implementation after the Modification. Ship Records carry no armour component on
  LIVE, so armour is not part of the rule. The default loadout does not count, so editions that
  differ only in it are grouped; paints, livery and names never count. Items: equal `AttachDef`
  type, subtype, size, grade and manufacturer, equal stat blocks including `resourceStates`, and
  equal other components, leaving out the display, geometry, material, purchasable, interaction,
  carrying and actor-usage components and the tag, colour and palette fields. The exact lists,
  taken from the field differences between colour siblings on LIVE, are in
  `docs/basetool-catalog.md`.
- **No name enters the rule, and playability does not either.** AI and mission variants follow
  the same rule as player ships.
- **The base** is the member with `playability` `player`, then the smallest `className`, then
  the smallest GUID.
- **Equality that cannot be shown leaves an entity ungrouped:** a ship without a definition or
  with an implementation that is not read in full, an item without a type or without a component
  that is compared.
- **Equality is a 128-bit digest** of the compared data (two SipHash states); nothing but the
  digest is kept per entity.

## Consequences

- A ship family whose members differ in the implementation splits into several
  groups; the Basetool may match them by the Modification and class name.
- Matching a pledge to a ship type needs a mapping of its own; it is not part of this decision.
- `variantOf` means "structurally identical", not "cosmetic" in the shop sense. On LIVE it also
  groups per-ship functional duplicates, e.g. the 96 `Controller_Shield_<ship>` items and the 39
  `Engineering_Buff_Modifier_<ship>` items, which are equal in every compared field and differ only
  in their name. They are kept grouped by decision; a consumer that wants only items players
  see or fit combines `variantOf` with `playability`.
- The item rule is conservative about components it does not know: an unknown cosmetic component
  keeps two otherwise equal items apart. The real-Archive test prints the groups and the near
  misses so that the lists can be refined.
