# SCMDB data dumps are a development check, saved by hand

Follows ADR-0245 (the RSI ship matrix) and ADR-0002 (the Archive is the source).

## Context

SCMDB (`https://scmdb.net`) publishes JSON dumps per game Build at
`https://scmdb.net/data/<name>-<version>.json`, listed in `data/versions.json`, with the version
`<game version>-<channel>.<change number>`, e.g. `4.10.2-live.12913470`. SCMDB has no documented
API. Wherever these dumps take a value straight from the DataForge, they are an independent
reading of the same Build and can check ours.

## Terms of use

The licence of the dumps is unclear: the SCMDB impressum reserves the operator's content, and
`https://scmdb.net/robots.txt` disallows `/data/`. Until the operator (info@scmdb.net) permits
otherwise, the dumps are treated like the ship matrix of ADR-0245.

## Decision

- The owner saves the dumps **by hand** in a browser to `local-data/scmdb/` in the repository
  folder: `merged-<version>.json`, `crafting_blueprints-<version>.json` and
  `mining_data-<version>.json`, one set per Build. `/local-data/` is excluded locally in
  `.git/info/exclude`, so they never reach git. The local wrapper `tools\real-archive.ps1` (not
  tracked) sets `P4KX_REF_SCMDB` to the folder when it exists.
- No code of P4KX requests anything from SCMDB, and CI has no dump. The real-Archive test
  `dataforge_values_agree_with_scmdb_dumps_of_the_same_build` of `p4kx-extraction` skips without
  `P4KX_REF_SCMDB`, and without a dump whose version names the change number and game version
  of the Archive's build manifest. Its comparison logic is tested with small synthetic dumps.
- Nothing from the dumps is committed, copied into a fixture or doc, or exported, and no value is
  read from them into P4KX.
- Compared are only values SCMDB takes from the DataForge unchanged: of
  `CraftingBlueprintRecord` the product, the tiers, their craft time, the slots of the mandatory
  cost with quantity and minimum quality of every option, and the modifier ranges; of
  `MineableElement` the `element*` fields and the resource type; of `MineableComposition` its
  parts; of `HarvestableClusterPreset` the clustering probability and parameters; the
  `minReputation` of every standing a contract names; and of every contract, by its `id`,
  `timeToComplete`, `contractBuyInAmount` and the minimum standing. Numbers agree within the
  precision of an `f32`.
- Not compared, because SCMDB rewrites them: display names and texts (token rewriting, collapsed
  blank lines, trimmed and curated names), densities (they appear nowhere in the DataForge) and
  the maximum standing of a contract (replaced by the top rank of its scope).
- A blueprint product that is a dangling Reference in the game data agrees when the error value
  names the GUID SCMDB states. A sub-contract counts as a contract of its `id` without compared
  fields.

## Consequences

- Each Build SCMDB publishes can be cross-checked locally without automated access to SCMDB.
- If the operator permits automated access, a later ADR may add a fetch.
