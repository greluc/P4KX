# A folder of extracted files is converted through the sources of an Extraction

Decided for issue #863. Builds on ADR-0003 (layers), ADR-0248 (portable file names) and ADR-0249
(a DataForge file is recognised by its content).

## Decision

- **An Extraction reads its files through an `EntrySource`.** `p4kx-extraction` works on a
  `SourceEntry` (path with `\`, size, position, where the data is stored) that a source opens.
  The Archive is one source; `FolderSource` is the other. The writers, the texture, sound,
  soundbank and DataForge conversions, the progress tracker and the written-path bookkeeping know
  only the trait, so `extract --convert` and `p4kx convert` share the converters, the output
  names (`relative_path`, `check_path_length`, the case collision rule), the Conflict policy, the
  report and the cancel. The trait lives in the service crate; `p4kx-archive` is unchanged
  (ADR-0003).
- **`p4kx convert <folder> --out <folder>`** takes `--filter`, `--textures`, `--mip`, `--audio`,
  `--conflict` (default `skip`) and `--copy-other` as `extract` does. The relative paths below
  the input folder act as Entry paths: the `p4kx search` Filter matches them, a texture is its
  header `<name>.dds` with the parts `<name>.dds.N` and the alpha tail `<name>.dds.a` next to it,
  and a header whose parts are missing is reported as not converted and written unchanged.
- **What is converted** is what `extract --convert` converts: CryXml (by content), textures,
  `.wem` sounds and `.bnk` soundbanks (by extension), and a DataForge file (by content, ADR-0249),
  which becomes one Record XML file per Record below its folder. A file with no conversion is
  counted as not converted and left out; with `--copy-other` it is written unchanged. With several
  DataForge files in a folder only the first by path is converted, and the others count as not
  converted.
- **The folder is untrusted.** The scan lists regular files below the root with `read_dir` and
  never follows a symbolic link or junction; each is reported as a failure and not read. A name
  that is not UTF-8 or holds a separator is rejected the same way. The scan is capped at
  `MAX_FOLDER_FILES` files and `MAX_FOLDER_DEPTH` levels. Opening a file checks again that it is
  no link and still has the listed size; a file that changed fails on its own. Files stream in
  chunks; only CryXml, DataForge and sounds are read whole, under the limits of an Extraction.
- **The output never touches the input.** The command refuses an output folder equal to, inside
  or above the input folder, resolved through links, and one inside a Star Citizen installation;
  nothing is written then.
- **Exit codes and report** are those of `extract`: 0 complete, 1 partial failure, 2 invalid
  input, 130 interrupted. `--json` prints the report of `extract` plus `not_converted`.
- **The window offers Convert folder…** in the Archive menu with progress and cancel, running the
  same function on a background thread.

## Reasons

- One pipeline gives the guarantee the issue asks for: a converted folder equals what
  `extract --convert` writes for the same Entries, tested by comparing both trees.
- A copy of the orchestration for folders would drift from the Archive path; a folder-backed
  Archive would put a second storage model into the format crate that reads untrusted p4k data.
- Checking links and sizes at scan and at open keeps a hostile folder from leading the run out of
  its root, short of a process that races the open itself.
