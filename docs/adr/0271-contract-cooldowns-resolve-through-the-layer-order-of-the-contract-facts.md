# Contract cooldowns resolve through the layer order of the contract facts

Decided for issue #909. Evidence: LIVE 4.10, the flat values of the `ContractGenerator` Records.
Amends ADR-0244, which read the cooldown fields from the `defaultAvailability` of the handler only.

## Census

The five cooldown parameters are `HasPersonalCooldown`, `PersonalCooldownTime`,
`PersonalCooldownTimeVariation`, `AbandonedCooldownTime` and `AbandonedCooldownTimeVariation`. The
`param` of an override is an enum value of the game; the value types and counts on LIVE are:

| Level (field) | Override array | Parameter (type) | Overrides |
|---------------|----------------|------------------|-----------|
| sub contract | `intParamOverrides` | `PersonalCooldownTime`, `PersonalCooldownTimeVariation`, `AbandonedCooldownTime`, `AbandonedCooldownTimeVariation` (integer) | 6 each |
| contract (`paramOverrides`) | `boolParamOverrides` | `HasPersonalCooldown` (bool) | 13 |
| contract (`paramOverrides`) | `intParamOverrides` | `PersonalCooldownTime` (integer) | 110 |
| contract (`paramOverrides`) | `intParamOverrides` | `PersonalCooldownTimeVariation` (integer) | 9 |
| contract (`paramOverrides`) | `intParamOverrides` | `AbandonedCooldownTime` (integer) | 23 |
| contract (`paramOverrides`) | `intParamOverrides` | `AbandonedCooldownTimeVariation` (integer) | 2 |
| handler (`contractParams`) | `intParamOverrides` | `PersonalCooldownTime` 4, `PersonalCooldownTimeVariation` 1, `AbandonedCooldownTime` 2 (integer) | 7 |

- The handler's `defaultAvailability` states the five fields as `Single` (floating point) values;
  the overrides state the times as integers. No override array of a float type exists on LIVE, so
  no cooldown is overridden by a float there.
- The overrides are partial: a contract that overrides `PersonalCooldownTime` takes the other four
  fields from the handler. A contract can also override only `HasPersonalCooldown`.
- Of the 5,175 entries, 215 take at least one field from the contract's overrides, 6 from the
  overrides of the sub contract, and 4,954 from the handler alone. Intro contracts and the
  legacy and bounty containers state none.

## Decision

- Each cooldown field resolves through the layer order of the other contract facts (ADR-0263): the
  nearest of sub contract, contract and handler that overrides the parameter of the same name
  (case ignored), else the `defaultAvailability` of the handler. The resolver is the existing
  `mission_layers::Layer` with `stated()`; `Layer` reads a new `floatParamOverrides` array next to
  the bool and integer ones, and `Layer::number_param` returns the float override of a name, else
  its integer override widened to `f32`. The field name `floatParamOverrides` follows the sibling
  arrays; LIVE has no such array, so it is read in readiness, not observed.
- The values stay as stored, as numbers, so the flat fields keep their names, types and meaning.
  Integer overrides appear as floating point numbers, like the handler's values.
- `cooldownLayer` is the nearest level that states any of the five fields: `subContract`,
  `contract` or `handler`; `null` when none does. Because the overrides are partial, a new object
  `cooldownLayers` names the level of each of the five fields (`null` for a field no level
  states). The layer type is `FactLayer` of the contract facts; the cooldown never reports
  `template`, which has no cooldown.
- The catalog stays additive within schema version 2 (ADR-0054): `cooldownLayers` is new, and
  `cooldownLayer` takes the values `subContract` and `contract` besides `handler`. A consumer that
  reads the handler only is unaffected until it meets those values.
- The unit statement of ADR-0244 stands: the Archive states none.

## Consequences

- A contract that overrides a cooldown is exported with its own value and the layer that states
  it, in place of the handler's value with `handler`.
- The real-Archive test reads the overrides straight from the `ContractGenerator` Records and
  checks that every entry that overrides a cooldown reports `contract` or `subContract`.
