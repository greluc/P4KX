# Ship base data is summed from the vehicle implementation and the Loadout's cargo grids

Decided for issue #316.

The Basetool wants mass, hull points, cargo capacity, size and the default Loadout of ships from the
Archive instead of UEX and the SC Wiki. ScDataDumper, NovaSCDataExtractor and scunpacked agree on
the facts (not on every detail), and none is a licence we can copy from, so the rules below are
the facts reduced to what the Archive states, and each export value names its source.

## Decision

- **Where it lives.** `p4kx-export` reads one ship with `ShipReader` and all ships with
  `build_ship_base`; the output types are public so that the catalog kinds of #395 and #393 reuse
  them. `p4kx-assembly` supplies the parts: `VehicleImplementation::totals`, `ValueOrigin` per part
  attribute and `EntityNode::loadout`, the default Loadout walk that #393 shares. The catalog shape
  is not changed here.
- **Mass** is the sum of `mass` over all parts of the patched vehicle implementation. Landing gear
  and Loadout items are not added. A ship whose parts state no mass takes the mass of its rigid
  physics controller (ScDataDumper does the same). The sum is in kilograms, rounded to four
  decimals.
- **Hull points** are the sum of positive `damageMax` over the parts, leaving out parts whose item
  port is flagged `uneditable` or `invisible`. The destruction threshold of a part is
  `damageRatioMin × damageMax` of its first damage behaviour in the group `Destroy`; the detach
  threshold is `detachRatio × damageMax` for a part without such a behaviour.
- **Cargo** is the sum of the capacity of the cargo grids (`AttachDef.Type` `CargoGrid`) reached
  through the default Loadout. An open container gives `x·y·z / 1.953125` and a cell count of
  `floor(side / 1.25)` per side; both are exported because the three reference tools use the
  volume and the game counts cells. A closed container gives its capacity in cargo units.
  Templates and the subtrees of docked vehicles are not counted.
- **Docked vehicles** are not expanded (`stopping_at_docked_vehicles`, #305): they appear in the
  Loadout marked, and their mass, grids and ports do not enter any sum.
- **Number noise** (#304): invisible Unicode format characters (category Cf) are dropped before a
  number is read, the existing prefix rule of ADR-0050 handles suffixes such as `d`, and a number
  attribute that holds text but no number is reported with its raw text in `unreadable` and
  left out of the sum, never counted as 0. The count of cleaned numbers is exported so a real
  Archive shows how often it happens.
- **Provenance** (#411): the vehicle implementation records per attribute whether the base file, the
  patch file or an `Elem` of the Modification set the value that is read (`ValueOrigin`). Sums
  export one source per origin with the number of values it supplied: the file is the base file,
  the patch file, or the base file for an `Elem`, with the Modification named in the field.
  The value of one part is not exported separately; a consumer that needs it reads the part with
  `VehicleImplementation`.
- **Format.** `p4kx ships` writes JSON (one document with `schemaVersion`, the catalog's `source`
  and the ships) or NDJSON (the ships). The data stays additive within version 1 under the rule of
  ADR-0054. It has no game texts, so the legal limits for third parties hold unchanged.

## Considered Options

- Adding the Loadout items' masses to the hull mass (ScDataDumper's `MassLoadout`): left out, since
  the issue asks for the hull and the items are the Loadout's data (a consumer can sum them from the
  class names).
- Reading cargo from every `SCItemInventoryContainerComponentParams` in the tree (ScDataDumper's
  fallback strategies by class-name prefix and base class): not done, since those guess a ship's
  grids from its siblings; this export counts what the Loadout reaches.
- A per-value source list on every part: too large for a ship with hundreds of parts; the tally per
  origin names every file that supplied a value.
