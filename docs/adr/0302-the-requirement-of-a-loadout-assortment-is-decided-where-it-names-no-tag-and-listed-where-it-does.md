# The Requirement of a loadout assortment is decided where it names no tag and listed where it does

Decided for issue #955, on top of ADR-0289 (a generated Loadout takes the first alternative of
each group of its assortments) and in the manner of ADR-0279 (conditions stay undecided unless they
restrict nothing). Evidence: the LIVE Archive and the census
`crates/p4kx-assembly/tests/real_archive/real_assortment_requirement.rs`.

## What the data says

`SLoadoutAssortment` has a `Requirement` field and so has each of its `GroupedLoadouts`. It is a
strong pointer to one of three Requirement structs:

- `SLoadoutRequirementTags` with `RequiredTags` and `BannedTags`, arrays of References to Tags;
- `SLoadoutRequirementOR` with `Requirements`, an array of Requirements;
- `SLoadoutRequirementNOT` with `RequirementToInvert`, one Requirement.

No other Requirement type exists in LIVE, and a Requirement nests three deep at most (a NOT around
an OR of tag sets).

LIVE holds 321 assortments and 2316 groups. No assortment has a Requirement, as #882 found for the
creature assortments. 1055 groups have one: 1034 `SLoadoutRequirementTags`, 14 `OR`, 7 `NOT`. Of
the 1034 tag Requirements, 248 name no tag at all, neither required nor banned.

A tag Requirement matches Tags against the Tags of the spawn that asks for the Loadout. The Archive
holds neither those Tags nor the rule of the match (every required Tag or any, and against which
Tags); the game decides at spawn time. The Tags themselves are in the Archive, the spawn is not.

## Decision

- **A Requirement is `Met`, `Unmet` or `Undecided`.**
  - No Requirement, a null pointer, and a tag Requirement with neither required nor banned Tags
    restrict nothing: `Met`.
  - A tag Requirement that names a Tag is `Undecided`, as is a Requirement of a type not listed
    above, an `OR` without parts, a `NOT` without a part, and one nested deeper than 16 levels.
  - `OR` is `Met` when a part is `Met`, `Unmet` when all parts are `Unmet`, otherwise `Undecided`;
    `NOT` swaps `Met` and `Unmet` and keeps `Undecided`. Nothing in LIVE reaches `Unmet`; the rule
    follows from the two above and is there for a patch that adds such a Requirement.
- **An `Unmet` assortment or group gives nothing.**
- **An `Undecided` assortment or group is used and listed.** It contributes its first alternative as
  ADR-0289 states, so that no export changes, and the Loadout of the Entity lists the issue
  `assortment_requirement` with the group (or the assortment) and the GUID of the assortment. A
  script that wants the exact picture of a spawn knows which groups the game might have left out.
  The group is not dropped: the Archive does not show that the game would drop it, and dropping the
  groups of a human NPC would leave the body bare more often than the game does.
- **The picks stay those of ADR-0289.** The alternative within a group is still the first; the
  option to choose another is #954 and builds on the same group reading.
- **The creature note is unchanged.** A creature lists the issues of its Loadout as
  `loadout_unreadable` (ADR-0289); the creature assortments have no Requirement in LIVE, so no
  creature gains a note.

## Consequences

- The Entities of LIVE reach 371 `Undecided` groups; 538 Entities (every Entity with a generated
  Loadout) list at least one `assortment_requirement`. The ports they fill are those of ADR-0289.
- The new issue code is `assortment_requirement`, listed in `docs/cli-json.md` and among the
  `issues` codes of the catalog's loadouts.
- The census counts the assortments, the groups with a Requirement by type, the tag Requirements
  that restrict nothing, and the undecided ones reached by an Entity.
