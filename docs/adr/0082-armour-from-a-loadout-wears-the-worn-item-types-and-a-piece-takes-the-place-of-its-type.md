# Armour from a Loadout wears the worn item types, and a piece takes the place of its type

Decided for issue #150, which builds on ADR-0079 (armour on a body) and the Loadout tree of
ADR-0012. Evidence: `p4kx armour glb --loadout` over the XML Loadout
`Scripts/Loadouts/Character/Human/Armor/NPC/Criminal/Pyro/Criminal_Gang_Guard_Heavy_Boss.xml` of
LIVE on both bodies (`crates/p4kx-assembly/tests/real_archive/real_armour_loadout.rs`): an
undersuit, arms, helmet, legs, torso and a backpack, the GLB passing the checker of the testkit.

## What a Loadout gives

`--loadout` names one of, tried in this order:

- an XML Loadout file on disk, when the text is the path of a file; a file on disk always wins
  over an Entity or Archive Entry of the same text. Its includes are read from the Archive, and
  the text is the name of its root in the report;
- an Entity, by class name, GUID or record path, whose default Loadout is resolved as for
  `entity loadout`;
- an XML Loadout Entry of the Archive, by its path (`Scripts/Loadouts/…xml`), when no Entity
  matches and the text ends in `.xml`.

An XML Loadout is resolved into a tree whose root is no Entity (`LoadoutResolver::resolve_file`),
so that all forms go through the same `EntityNode` walk. One resolution reads at most
`MAX_XML_INCLUDES` files, each file once, failures included; further includes are left out with
an issue. A file on disk that cannot be opened is invalid input like the other forms. A text that is none of these is invalid
input (exit code 2) and writes nothing.

The pieces are the Entities anywhere in the tree whose `AttachDef.Type` starts with `Char_Armor_`
or `Char_Clothing_` (ignoring case), also below an empty port's nested Loadout and below other
worn Entities, in tree order, named `<port>/<class name>`. Weapons, bodies, heads and other items
are not pieces. Whether an Entity has geometry for the body variant is the business of
`Armour::build` (ADR-0079), so a piece the body cannot wear is skipped and listed there.

What cannot be resolved is a `Piece::failed`, so that the report lists it as `piece_not_found`: an
attachment that fails (unknown class name, dangling Reference, cycle), an Entity whose Record
cannot be read, and what the root Loadout could not read, such as an include that is missing. An
unresolved attachment is listed whatever it was meant to be, because nothing is known of an Entity
that cannot be found.

## `--piece` on top of a Loadout

A `--piece` whose Entity has an item type takes the place of the Loadout's pieces of the same item
type, ignoring case: one torso armour replaces the Loadout's torso armour, whatever port it hangs
on. A `--piece` of another type, or without an item type, is added after the Loadout's pieces. The
report lists what was replaced (`replaced` in JSON, `replaced <old> by <new>` in the text). Pieces
given by hand never replace each other.

## Not decided here

The NPC actors of the Archive name their clothing and armour in a
`SItemPortLoadoutOutfitParams` (a base Loadout and outfits selected by tags), which the Loadout
reader does not read yet, so the default Loadout of such an actor holds no pieces. Their XML
Loadouts can be given by path. Reading outfits needs a rule for choosing among them, and changes
`entity loadout` for every NPC.
