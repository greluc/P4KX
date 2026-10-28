# The RSI ship matrix is a development check, saved by hand

Decided for issue #809. Applies ADR-0002 (the Archive is the source of the catalog).

## Context

The `playability` of ships (ADR-0054, "Playability" in `docs/basetool-catalog.md`) is derived from
Record data and has never been compared with an outside reference. Another extractor takes "flight
ready" from the public RSI ship matrix, `https://robertsspaceindustries.com/ship-matrix/index`
(`production_status == "flight-ready"`). The same data can serve as a development check of our
rule.

## Terms of use

Checked on 2026-10-28 in the RSI Terms of Service (`https://robertsspaceindustries.com/en/tos`)
and in `https://robertsspaceindustries.com/robots.txt`:

- Section IV (Rules of Conduct) forbids "automated measures like bots, spiders, scrapers or
  similar means or technologies" and names data scraping or harvesting.
- Section XIII.B (Intellectual Property Rights) forbids copying or downloading RSI Content from the
  RSI Services without permission, and selling, distributing, publishing or modifying it.
- Section XIII.D (Personal and Fansite Use) forbids creating derivative works of the RSI Fansite
  Content.
- `robots.txt` disallows only `/media/` and has no rule for `/ship-matrix`.
- No clause names crawlers, robots.txt or data mining.

An automated fetch, by P4KX or by CI, is an automated measure under section IV. P4KX therefore
never fetches the ship matrix.

## Decision

- The ship matrix JSON is saved **by hand** in a browser by the owner, as `ship-matrix.json`, to
  `local-data/ship-matrix/ship-matrix.json` in the repository folder. `/local-data/` is excluded
  locally in `.git/info/exclude`, not in `.gitignore`, so the file never reaches git and never
  leaves the PC. `tools\real-archive.ps1` points `P4KX_REF_SHIP_MATRIX` at it when it exists.
- No code of P4KX requests anything from RSI. CI has no ship matrix: the check is `#[ignore]`d,
  skips without `P4KX_REF_SHIP_MATRIX`, and its comparison logic is tested with a small synthetic
  matrix written for the test.
- Nothing from the ship matrix is committed, copied into a fixture or doc, or exported: no
  descriptions, images or stats, and no value is read from it (its stats are often out of date).
  Docs hold only counts and Archive class names derived by the check.
- The check reads the matrix as `{"data": [...]}` or a bare list. A matrix entry maps to a catalog
  ship through the curated `shipStoreSlugs` (ADR-0117): by the RSI ship id (`id`), then by the last
  segment of `url` against the slug. Matrix names the slugs do not map are mapped by an explicit
  name table in the test (`NAME_SUPPLEMENT`), read off the Archive's class names and used only
  there; no name enters the `playability` rule.
- The `playability` rule is refined only on a structural signal, never by name. The first
  comparison (see "Comparison" in `docs/basetool-catalog.md`) showed none.

## Consequences

- The comparison is repeatable by anyone who saves the matrix the same way, without any automated
  access to RSI.
- The ship matrix changes over time; the stored counts date from the file of 2026-10-28.
- If RSI permits automated access to the ship matrix, a later ADR may add a fetch.
