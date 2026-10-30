# The catalog verification measures texts per key and kind, not by a global share

Decided for issue #916. Evidence: LIVE 4.10. Amends ADR-0266, which exported only the `nameKey` of
location templates to stay above the German floor.

## Decision

- The real-Archive verification no longer asserts a global share of German or English texts. A
  share over all kinds measures how complete the game's German `global.ini` is, so every new kind
  with untranslated keys can break it (the contract descriptions added 5,143 texts the German file
  lacks and took the share to 0.469 against 0.5).
- It checks the exporter per kind and key, against the Archive's own `global.ini` files read
  independently of the exporter:
  - a key the language's file holds with a text must have that text in the catalog, and a key it
    lacks or holds empty must be `null` there (the Basetool falls back to English); a contradiction
    fails and names the kind and keys;
  - the keys a file lacks are counted per kind and language and must stay within the baseline of
    that kind, measured on LIVE and rounded up to the hundredth. A kind that is not listed has a
    baseline of zero, so a new kind names itself and its baseline is added on purpose.
- The keyed `name` and `description` of every kind and the `displayName` of each `names` entry of a
  location template are measured; the numbers of every kind are printed.
- The English check keeps its meaning: an unresolved English key is counted against a per-kind
  baseline, so mis-built keys show up in the kind that builds them.
- `missionLocationTemplates[].names[]` exports `displayName` as a game text (`x-gameText`) again,
  like every other named kind, in place of `nameKey`. The catalog stays additive within schema
  version 2 (ADR-0054).

## Consequences

- Translation gaps in the game never fail the verification; a regression of the exporter, or a
  kind whose untranslated share grows past its baseline, does and names the kind.
- A change of the game that moves a share past its baseline is a deliberate edit of the baseline,
  made after looking at the kind.
