# A DataForge file on disk is read by its content, without an Archive

Decided for issue #862. Builds on ADR-0136 (versions 6 and 8 only) and the size limit of the
DataForge Entry.

## Decision

- **A `Game2.dcb` kept from an earlier patch is read like the Entry of an Archive.** Every
  `p4kx dcb` command (`info`, `export`, `query`, `fields`, `refs`, `tags`), the DataForge tools of
  the MCP server and the window accept the file where they accept an Archive.
- **The file is recognised by its content, not by its name.** The first eight bytes decide: they
  are no Archive signature (`PK`) and the version word after the reserved word is at most 65,535.
  Any such version passes the recognition, so that version 5 and older end as the typed
  `DataForgeError::UnsupportedVersion` of ADR-0136 and exit code 5, not as an unknown file. A
  file that is neither an Archive nor a DataForge is an Archive error, exit code 3.
- **The limits are those of the Entry.** Versions 6 and 8, and `MAX_DATAFORGE_SIZE` (2 GiB), which
  is checked against the file length before a byte is read. The file is opened read-only and read
  in mebibytes, polling for a stop like the Entry.
- **Texts and tags.** A DataForge holds no localisation texts. `dcb export --lang` on a file needs
  `--archive <Data.p4k>` for them and exits with code 2 without it; an Archive as input supplies
  its own and refuses `--archive`. Tag paths resolve from the DataForge alone.
- **Without Channel and build** the default export folder is
  `SC-EXPORT-UnknownChannel-UnknownBuild-<timestamp>`, and `--combined` is named after the file.
- **MCP.** The `archive` argument of the DataForge tools takes the absolute path of a `.p4k` or a
  `.dcb` file, on a local drive like before. The extension stays required there: a tool call comes
  from a model, and recognition by content alone would let it probe any file of the machine. The
  read DataForge and the selection of the last Query are kept per file version.
- **Window.** The Archive menu offers **Open DataForge file…**; dropping a file or opening one
  through **Open Archive…** works as well. The file opens as an Archive without Entries
  (`Archive::stand_in`), so the Records, Query and Tags views use the one DataForge cache they
  use for an Archive. The Entry views are empty; localisation texts are not offered.

## Reasons

- Users keep `Game2.dcb` files of old patches, and the Archive they came from is often gone.
- Recognition by content serves a renamed file; the version word, not the extension, separates a
  DataForge from an Archive.
- A stand-in Archive in the window reuses every consumer of the DataForge cache instead of a
  second code path through them.
