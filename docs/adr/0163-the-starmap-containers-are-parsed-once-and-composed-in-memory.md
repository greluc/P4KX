# The starmap containers are parsed once and composed in memory

Decided for issue #455. Follows up ADR-0002 and the starmap kind of #450.

The first reading of the system containers walked each star system depth first, opening,
decompressing and parsing every external container where a Child named it. A container instanced
several times (the Nyx ring) was parsed again for each reference, and the raw XML of every
container on the chain (up to 128 MiB each) was held at once. Only the systems ran in parallel.

## Decision

- **Parse once, then compose.** `starmap_parse` reads the reachable containers level by level
  from the system containers: the containers of one level are read and parsed in parallel
  (rayon), each exactly once for the whole Archive, into a tree of Children (pose, record,
  entity name of a placed Child, external path, nested Children). The raw XML is dropped after the
  parse. `starmap_walk` then composes the positions per system, in parallel, over those trees,
  in the same document order, with the same limits and counts as before. The levels are bounded
  by the external chain limit (32); a cycle ends because a container is read once.
- **A container whose XML is not named like it is reported.** When no `<stem>.xml` exists and the
  first other XML entry is taken, the report gets an issue line naming it. It is read, not counted
  as unreadable, because the position data is intact.
- **A second `ChildObjectContainers` is reported.** Only the first list of an element is read;
  when its direct siblings hold another one, an issue line says so. The positions are unchanged.
- **Type names are read once.** `TypeNames` caches the name of each type Record by GUID for the
  parallel scan.
- **`StarmapReport` stays public, read-only.** Its lines are diagnostics for the real-Archive
  verification and for a person checking the placements; the fields are crate-private and
  `CatalogSummary::starmap_report()` hands out accessor methods. The wording of the lines is
  documented as not being a contract. A feature flag was not worth the second build
  configuration.

## Consequences

- The catalog JSON and its schema do not change.
- An out-of-range position is reported with the composed number and the entity name of a placed
  Child, no longer with the raw attribute text, which is not kept.
