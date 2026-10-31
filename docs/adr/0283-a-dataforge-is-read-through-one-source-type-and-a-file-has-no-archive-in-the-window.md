# A DataForge is read through one source type, and a file has no Archive in the window

Decided for issue #878. Replaces the stand-in Archive of ADR-0249. Builds on ADR-0003 (layers)
and ADR-0249 (a DataForge file is recognised by its content).

## Decision

- **One source type.** `DataForgeSource` in `p4kx-extraction` is either the DataForge Entry of an
  Archive or a DataForge file path. `p4kx dcb` (`ForgeInput`), the DataForge tools of the MCP
  server and the window (`ForgeCache`) read, load, watch and name a DataForge through it. The
  MCP server keeps its per-file caches around it and the window keeps its cache of indexes; only
  the question "where do the bytes come from" is shared.
- **`DataForgeSource` is not `EntrySource`.** `DataForgeSource` says where the bytes of one
  DataForge come from; ADR-0272's `EntrySource` and `FolderSource` list and open many files for
  an Extraction. They stay apart: an Extraction picks its DataForge file from what its source
  lists, while the window and the CLI read one DataForge without listing anything.
- **No Archive without Entries.** `Archive::stand_in` is removed; no constructor makes an Archive
  that has no Entries. `ArchiveWatch::of_file` watches a plain file by its `FileStamp` (size and
  modification time), without a Build; the MCP file cache validates against the same stamp.
- **`EntryIndex` of a file has no Archive.** `EntryIndex::archive()` returns `Option<&Archive>`.
  A job that reads Entries takes `EntryIndex::entries_archive()`, which fails with `NoEntries`
  for a file, so no such job runs against an empty Archive; the failure types of the jobs convert
  from `NoEntries`. What only needs a path or a Build (`path()`, `build_of_archive()`) answers
  for both.
- **The window shows a file as its DataForge only.** No folder tree and no Entry table are
  shown; the DataForge node is selected, its DataForge starts to read and the DataForge tab is
  shown. The status bar shows the path and nothing that counts Entries. The tabs that read
  Entries are hidden, now Search too, whose Record search is the one of the Browse tab.
- **The size limit is testable without a 2 GiB file.** `DataForgeEntry::read_file_within` takes
  the limit; the unit tests use a few bytes, and the `p4kx dcb` test uses an Archive whose Entry
  claims more than the limit.

## Reasons

- A public fake Archive let extraction, comparison, verify or export be handed something that
  fails misleadingly; the absence of an Archive is now a type the compiler makes callers handle.
- Three notions of "an Archive or a DataForge file" would drift; one type keeps the limits, the
  change detection and the naming the same.
