# The tag tree resolves tag GUIDs in output and for the services

Decided for issue #311, needed by #399 (paints) and #393 (loadouts).

Fields such as `Tags`, `RequiredTags` and `PortTags` refer to a Tag Record by GUID. A Reference
names the record path of its target, and that is the same for every Tag
(`libs/foundry/records/tagdatabase/tagdatabase.tagdatabase`), so a Record file or a preview told
nothing about the Tag, and a service that wants "which Tags are below `Ship`" had to read the Tag
Records itself.

`TagTree` of `p4kx-dataforge` reads the `TagDatabase` once into the tree of Tags. Every Tag Record
(`tagName`, `legacyGUID`, `children`) becomes a node; the roots are those the `TagDatabase` lists in
its `tags`. The tree is stored in pre-order, owns its text and borrows nothing from the DataForge, so
it can be shared between threads and kept next to the DataForge (the window keeps one per Archive in
its DataForge cache). It answers tag GUID to tag path, parent, children, ancestors, descendants and
"is within", and it is reachable from `p4kx-extraction` and `p4kx-export` for the catalog kinds that
match tags.

- **Tag path** is the names from the root down joined by `.`. It is not unique, because sibling Tags
  may share a name and a name may hold a dot; the GUID stays the identity and no lookup by path
  promises one result.
- **A tree that is not what the database lists is still built.** A Tag Record no `TagDatabase`
  reaches becomes a root of its own, unlisted, after those that no other Tag lists as a child and
  before a cycle's members; a Tag listed twice stays where it was listed first, which also cuts a
  cycle; children below 64 levels, and GUIDs that are no Tag Record, are counted and left out. The
  counts are `TagTreeIssues`; the command and the window say so, nothing fails. On LIVE (4.3) the
  tree holds 18,891 Tags in 66 roots, at most 12 levels deep, and every Tag is listed once.
- **Output only, like localisation text.** The Record tree stays what ADR-0004 says: links, never
  content. The writers of Record XML and Record JSON take `Annotations`, which hold the text
  resolver of `--lang` and the new `TagResolver`, and add `tag="…"` or `"tag": "…"` to a Reference of
  type `Tag` when asked (`dcb export --tags`, `RecordExportOptions::tags`). Without it the bytes are
  those of before; the Comparison and the Record JSON Schema see the same tree, and the schema
  allows the one new optional key. The older `write_with` of the writers stays.
- **Legacy IDs** are kept as stored; 4294967295 means none.

## Considered Options

- Resolving the path into the Record tree as a field of the Reference: every consumer of the tree
  (Comparison, Query, Record preview) would see a value that is not in the DataForge and that
  changes with another Tag.
- Joining names with `/`: it reads as a record path, which the Records of a Tag all share. Four of
  the 18,891 Tags on LIVE hold a dot in their name, so `.` is not unique either, but it reads as the
  tag paths people write (`Ship.Series.Avenger`) and the GUID settles the rest.
- Failing on an irregular `TagDatabase`: a build with one dangling child would show no tags at all.
- A Query clause or a catalog-only API: the tree is useful to the CLI, the window and every catalog
  kind alike, so it lives in the format crate that knows the Records, below the services.
