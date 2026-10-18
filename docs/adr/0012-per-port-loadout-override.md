# A Loadout entry's Loadout overrides the attached Entity's own per port

A Loadout entry that attaches an Entity may carry a Loadout of its own, and the attached Entity
may have a default Loadout as well. In the live data both are partial: a ship's entry gives a
gimbal mount, which has no Loadout, its gun, and a dashboard's own Loadout of screens is partly
restated by the ship's entry. The Loadout tree therefore starts from the attached Entity's own
Loadout, replaces the entry of each port the entry's Loadout names (port names ignoring ASCII
case) in place, and appends the entry's other ports in their order; ports the Entity defines but
no Loadout names follow with their default item. Every port records whether it comes from the
Entity's own Loadout, the override, the port's default item or nothing, so a front-end can show
which ports a parent changed. The same rule merges XML Loadouts with the Loadouts they include.
Decided by the owner.

## Considered Options

- The entry's Loadout replaces the attached Entity's own as a whole: simpler, but ports the
  entry does not restate, such as the dashboard's other screens, would vanish.
- The Entity's own Loadout wins over the entry's: contradicts the ship entries that change what
  sits in a component they attach.
