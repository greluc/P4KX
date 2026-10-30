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
  64 replacements, about 4 KiB). Blanks around the text are removed and line breaks decoded by
  `display_text` of `p4kx-localization`, the one place that trims game texts.
- **No template source.** The `displayString` of `contractDisplayInfo` is an unlabelled list of
  slots; none is known to be the description, so the template contributes nothing.
- **Additive** (ADR-0054): a new field on an existing kind, version unchanged.

## Consequences

- The Rust type, the schema, the docs and the game text test change together.
- An entry without a description has `key`, `en` and `de` `null`.
- On LIVE 4.10 (2026-10-30) 5,130 of the 5,175 entries have a description (5,108 with an English
  text); 1,718 keep a run-time token the catalog cannot state (`[SCRIPAMOUNT]` 628,
  `[CONTRACTOR]` 442, `[FUELRATE]` 212). The descriptions add about 4.1 MB; the plain catalog is
  166.9 MB, so the upload stays `.json.zst`.
