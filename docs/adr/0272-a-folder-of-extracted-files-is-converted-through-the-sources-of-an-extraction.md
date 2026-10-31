# A folder of extracted files is converted through the sources of an Extraction

Decided for issue #863. Builds on ADR-0003 (layers), ADR-0248 (portable file names) and ADR-0249
(a DataForge file is recognised by its content).

## Decision

- **An Extraction reads its files through an `EntrySource`.** `p4kx-extraction` works on a
  `SourceEntry` (path with `\`, size, position, and the source that listed it, which opens it by
  that position, so a file cannot be handed to another source). The Archive is one source;
  `FolderSource` is the other. The writers, the texture, sound, soundbank and DataForge
  conversions, the progress tracker and the written-path bookkeeping know only the trait, so
  `extract --convert` and `p4kx convert` share the converters, the output names
  (`relative_path`, `check_path_length`, the case collision rule), the Conflict policy, the
  report and the cancel. The trait lives in the service crate; `p4kx-archive` is unchanged
  (ADR-0003). Which file is the DataForge is a rule of the run, not of the source.
- **`p4kx convert <folder> --out <folder>`** takes `--filter`, `--textures`, `--mip`, `--audio`,
  `--conflict` (default `skip`) and `--copy-other` as `extract` does. The relative paths below
  the input folder act as Entry paths: the `p4kx search` Filter matches them, a texture is its
  header `<name>.dds` with the parts `<name>.dds.N` and the alpha tail `<name>.dds.a` next to it,
  and a header whose parts are missing is reported as not converted and written unchanged.
- **What is converted** is what `extract --convert` converts: CryXml (by content), textures,
  `.wem` sounds and `.bnk` soundbanks (by extension), and one DataForge file, which becomes one
  Record XML file per Record below its folder. A file with no conversion is counted as not
  converted and left out; with `--copy-other` it is written unchanged.
- **Which DataForge file.** A folder has no fixed layout, so the rule goes from the cheap and
  certain to the expensive: a file at the path of an Entry the game reads, `Data\Game2.dcb`
  before `Data\Game.dcb` as `DataForgeEntry::locate` prefers them, also when the folder is the
  `Data` folder itself and the path is `Game2.dcb`; without one, the first file by path with the
  extension `.dcb` that starts like a DataForge (ADR-0249); without one, the first file of any
  name that does. The paths come first because they name what the game uses and need no read,
  and a stale `Game.dcb` kept next to the current one must not win by sorting order. Every other
  DataForge file the rule found is left out and named in the report (`dataforge_left_out`, a
  line on stderr per file); looking at the content of files with other names happens only when
  no path says which file it is. The choice runs on the thread limit of the run and stops with
  its cancel.
- **The folder is untrusted.** The scan lists regular files below the root with `read_dir` and
  never follows a symbolic link or junction; each is reported as a failure and not read. A name
  that is not UTF-8 or holds a separator is rejected the same way. Files, folders and rejected
  names together count against `MAX_FOLDER_ENTRIES`, which also bounds the list of folders still
  to read, and nesting against `MAX_FOLDER_DEPTH`. Opening a file, also to look at it, checks
  again that it is no link, still has the listed size and still resolves below the root (a race
  between that check and the read remains possible for a process that writes into the input
  folder), and reads no more than the listed size. Files stream in chunks; only CryXml,
  DataForge and sounds are read whole, under the limits of an Extraction.
- **The output never touches the input.** The command refuses an output folder equal to, inside
  or above the input folder, resolved through links, and one inside a Star Citizen installation;
  nothing is written then.
- **Exit codes and report** are those of `extract`: 0 complete, 1 partial failure, 2 invalid
  input, 130 for a second Ctrl+C. A run cancelled by the first Ctrl+C, also while the folder is
  read, ends with 1 as `extract` does. `--json` prints the report of `extract` plus
  `not_converted` and `dataforge_left_out`.
- **The window offers Convert folder…** in the Archive menu with progress and cancel, running the
  same function on a background thread; a cancel while the folder is read shows as cancelled.

## Reasons

- One pipeline gives the guarantee the issue asks for: a converted folder equals what
  `extract --convert` writes for the same Entries, tested by comparing both trees.
- A copy of the orchestration for folders would drift from the Archive path; a folder-backed
  Archive would put a second storage model into the format crate that reads untrusted p4k data.
- Checking links and sizes at scan and at open keeps a hostile folder from leading the run out of
  its root, short of a process that races the open itself.
