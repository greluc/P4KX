# Contract entries state all their places, a giver faction and their blueprint rewards

Decided for issue #915 (VerseKit, an external consumer). Evidence: LIVE 4.10 (2026-10-31), the 109
`ContractGenerator` Records, 5,175 contract entries. Builds on ADR-0054 (additive growth), ADR-0065
(raw facts), ADR-0068 (contracts), ADR-0263 (layers) and ADR-0266 (locations).

VerseKit lists where a blueprint comes from and found four gaps in the `contracts` kind.

## Decision

- **`canBeShared` needs no change.** Each entry has had `canBeShared` as `{value, layer}` since
  ADR-0263 (parameter `CanBeShared` of sub contract, contract and handler, else the contract class
  of the template).
- **`offeredAt` lists every place.** `pickupPlaces` keeps its meaning (at most five starmap
  objects, ordered by rank; 3,936 of the 5,175 entries reach the cap). The new `offeredAt` has
  `localityGuids`, the `MissionLocality` Record of each `ContractPrerequisite_Locality`, and
  `locations`, the object of each `ContractPrerequisite_Location`, each with `starmapObjectGuid`,
  `objectType` and `place`. It reads the same prerequisites as the pickup places (a sub contract's
  own replace those of the contract and its handler), in stored order and none twice, without the
  cap, without dropping stars and without dropping an object that is missing from
  `starmapObjects` (its `objectType` and `place` are `null`). A locality is linked, not expanded:
  the consumer joins `missionLocalities[].starmapObjectGuids` to `starmapObjects`, which keeps
  the 5,175 entries from repeating the same list. A contract with no place at all takes the
  location of its broker entry. A Reference to a missing Record keeps its GUID (ADR-0269). On LIVE
  4,801 entries state a place and every link resolves.
- **`giverFaction` states the faction and where it came from.** `faction` stays the handler's
  `factionReputation` as stored; changing it would change its meaning, which ADR-0054 forbids.
  `giverFaction` is `{source, faction}`, `faction` having the shape of `faction`, and `source` one
  of `handler` (the same faction as `faction`), `reputationResult` or `reputationPrerequisite`:
  the handler's faction first; else the faction of the first success amount of the contract's
  `reputationGains` whose `amount` is positive; else the faction of the first reputation
  prerequisite (the order of `standingRequirements`) that is not an exclusion. A negative amount
  names a rival that loses standing (the `CFP_OOS` and `HH_OOS` handlers, ADR-0068) and never
  counts. `null` when none names a faction. On LIVE: 4,010 entries `handler`, 232
  `reputationResult`, 57 `reputationPrerequisite`, 876 `null`.
- **Why this does not break ADR-0065.** ADR-0065 forbids values computed with the game's rules
  (damage per second, acceleration). `giverFaction` computes nothing: it names one of the factions
  the Archive states and the field it came from, using a fixed order the documentation states.
  The raw facts stay in `faction`, `reputationGains` and `standingRequirements`, so a consumer
  with another preference reads them. The catalog does not weigh several stated factions against
  each other: the first wins.
- **`blueprintRewards` lists the pools of the entry.** Each element is `{poolGuid, chance}`: a
  `BlueprintRewards` result whose first `missionResults` flag (success) is set, `poolGuid` the
  `guid` of an entry of `blueprintPools` and `chance` as stored. The results are the entry's own
  resolved results: those of the sub contract when it states results, else those of the
  contract. `blueprintPools[].rewardSources` stays, and names only the `id` that a contract and
  its sub contracts share.

## Census (LIVE 4.10)

- `BlueprintRewards` results: 1,037, all in the `contractResults` of a contract (1,032 of
  `contracts`, 5 of `introContracts`). No sub contract and no handler has a `contractResults`
  block, so every sub contract takes the results of its contract; the reading of the sub
  contract's own results is the rule for a build that adds some.
- Outcome flags: 1,034 fire on success, 3 on the second outcome. Those three are left out of
  `blueprintRewards`, as a failure amount is left out of `reputationGains` (ADR-0068). All 1,037
  have a `chance`.
- The entries list 2,282 rewards on 783 contracts, each one checked against the reward sources of
  its pool.

## Consequences

- The three fields are additive within schema version 2 (ADR-0054). A reader that does not know
  them ignores them.
- `offeredAt.locations[].place` is a game text like `pickupPlaces[].place`.
- A consumer that wants names of every place resolves `localityGuids` through `missionLocalities`
  and `starmapObjects`, and reads `locations` directly.
