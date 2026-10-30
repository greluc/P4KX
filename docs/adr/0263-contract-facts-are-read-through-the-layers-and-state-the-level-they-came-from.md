# Contract facts are read through the layers and state the level they came from

Decided for issue #898. Evidence: LIVE 4.10 (2026-10-30), the `ContractGenerator` and
`ContractTemplate` Records.

## Decision

- Each contract entry of the catalog carries `timeToComplete`, `buyInAmount`, `canBeShared`,
  `illegal`, `onceOnly`, `maxPlayersPerInstance`, `availableInPrison`, `canReacceptAfterAbandoning`,
  `canReacceptAfterFailing`, `hideInMobiGlas` and `requiredIntros`, all additive (ADR-0054); schema
  version 2 keeps its number.
- `timeToComplete` (a `Single`) and `buyInAmount` come from `contractResults` of the contract,
  the only level that has the block; a sub contract shares them. They are plain numbers, as stored.
- The other eight values can be stated at several levels, so each is `{value, layer}` with `layer`
  one of `subContract`, `contract`, `handler`, `template`, or `null` when no level states it. The
  nearest level wins: a `ContractBoolParam` / `ContractIntParam` of the sub contract (stated
  directly on it), of the contract (`paramOverrides`) or of the handler (`contractParams`), matched
  by parameter name without regard to case; then the base value of the Record that owns the
  default: `onceOnly`, `maxPlayersPerInstance`, `availableInPrison`, `canReacceptAfterAbandoning`,
  `canReacceptAfterFailing` and `hideInMobiGlas` from the `defaultAvailability` of the handler
  (layer `handler`), `illegal` from the `contractDisplayInfo` and `canBeShared` from the
  `ActiveContractSettings` of the contract class of the template (layer `template`). A parameter
  override of the handler does not outrank its `defaultAvailability` in the layer name: both are
  `handler`, the override wins.
- `requiredIntros` lists the `ContractPrerequisite_CompletedContractTags` tags that the handler
  (`defaultAvailability`), the contract and the sub contract require, in that order and as stored,
  each with `level`, `tagGuid` and `introContracts`: the `id` of every contract in an
  `introContracts` array whose success result (`ContractResult_CompletionTags`, first
  `missionResults` entry set) grants the tag. A tag no intro contract grants has an empty list.
  The catalog does not decide how the levels combine.
- The excluded tags (`excludedCompletedContractTags`) are not exported: a contract offered only
  until its intro is completed is the opposite of a requirement, and the catalog states
  requirements only. `ContractPrerequisite_CompletedContractTags` has no `exclude` flag (its
  fields are `includePrerequisiteWhenSharing`, `requiredCountValue` and `excludedCountValue`).

## Consequences

- A consumer sees which level states each fact and can tell a game default (`handler`, `template`)
  from a deliberate override (`contract`, `subContract`).
- On LIVE 4.10 the 5,175 entries state `onceOnly`, `maxPlayersPerInstance`, `availableInPrison`
  and the re-accept flags at 5,162 to 5,175 entries through the handler, and `illegal` and
  `canBeShared` at all but about 420 (the legacy contracts without a template). 598 entries have a
  required tag that an intro contract grants.
