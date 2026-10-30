# Contracts export the description as a game text

Decided for issue #903, on top of ADR-0054 (additive growth) and ADR-0068 (contracts), whose scope
it changes: ADR-0068 left descriptions out.

## Decision

- **A `description` on every `contracts` entry,** a `LocalisedText` (`key`, `en`, `de`) marked
  `"x-gameText": true`, like the `title`. The Basetool keeps game texts from third parties; the
  catalog documents that the field must not be passed on.
- **Resolved like the title,** by the same code: the `Description` string parameter of the first
  layer (sub contract, contract, handler), else the `description` of the contract's
  `MissionBrokerEntry`. `~mission(Token)` is replaced with the title's rules and limits (depth 3,
  64 replacements, about 4 KiB), and blanks around the text are removed.
- **No template source.** The `displayString` of `contractDisplayInfo` is an unlabelled list of
  slots; none is known to be the description, so the template contributes nothing.
- **Additive** (ADR-0054): a new field on an existing kind, version unchanged.

## Consequences

- The Rust type, the schema, the docs and the game text test change together.
- An entry without a description has `key`, `en` and `de` `null`.
