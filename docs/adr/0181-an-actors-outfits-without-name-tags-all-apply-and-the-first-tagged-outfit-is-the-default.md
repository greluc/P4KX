# An actor's outfits without name tags all apply, and the first tagged outfit is the default

Decided for issue #513. Builds on ADR-0109, which reads an NPC actor's default Loadout as an
outfit (`SItemPortLoadoutOutfitParams`). Evidence: the 2,312 actor Records of the LIVE Archive
(`libs/foundry/records/actor/actors/*`) and `crates/p4kx-assembly/tests/real_archive/real_npc_outfits.rs`.

## What the Records hold

An outfit is a `BaseLoadout` (body, head, hair) and `Outfits`, each a
`SItemPortLoadoutOutfitTagParams` with `OutfitNameTags` (References to Tag Records) and `Loadouts`
(the clothing, armour and weapons). Of the 2,312 actor Records, 613 have no outfit, 1,695 have one
and 4 have two. Five Records have name tags at all:

- Two story characters, Luca Brunt and Tessa Banister, have two outfits that both have one name tag
  (`Outfits.S42.Work` and `Outfits.S42.OffDuty`): the game picks one by the tag a mission or scene
  asks for.
- Three Vanduul pilots have one outfit with one name tag.
- The other two-outfit actors (a Navy gunner, a Lorville guard) have outfits without name tags, and
  the guard's second one is empty.

No Record or file the tool reads says which tag applies by default; the choice is made by game
logic that sits outside the Archive.

## The rule

- Every outfit without name tags applies. Their Loadouts are laid over the base Loadout in the
  order of the Record.
- When every outfit has name tags, the first outfit in the Record's order applies, because the
  Archive names no tag to prefer. For Luca Brunt this is `Outfits.S42.Work`.
- An outfit with name tags beside an outfit without any does not apply.
- The Loadouts of the outfits that apply are limited to 16 layers, the base Loadout included.

The rule was already the behaviour of ADR-0109; this decision states it, names its reason and
makes it visible.

## Shown in the output

`LoadoutResolver::outfits` returns an `OutfitReport` for an Entity whose default Loadout is an
outfit: the XML Loadouts of the base Loadout and, per outfit, its name tags as tag paths (the
GUID when the tag database lacks the tag), its XML Loadouts and whether it applies. `p4kx entity
loadout` prints it as `outfit base:` and `outfit N worn|not worn` lines and in `--json` as
`outfit`; `p4kx armour glb --loadout` and `p4kx character package --loadout` repeat it in the
`loadout` object of their report. `entity glb` wears what the resolved tree holds, so it follows
the same rule.

## Consequences

- An actor with several tagged outfits is dressed in the first; a tag chosen by the caller (an
  `--outfit` option) is not offered. A caller who wants another outfit names its XML Loadout with
  `armour glb --loadout`.
- An outfit is parsed once, into the layers and, for the Entity's own default Loadout only, the
  report. Only then are the files of outfits that are not worn read; an outfit nested in a
  Loadout entry skips them and is not reported. The tag database is built once per resolver, on
  the first report that names a tag by Reference.
- A name tag that reads as nothing (a null Reference) is no tag: an outfit whose tags are all such
  has none and counts as untagged.
