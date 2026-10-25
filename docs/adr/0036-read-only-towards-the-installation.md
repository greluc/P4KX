# The tool never writes into a Star Citizen installation and only looks at the process list

The legal review of 2026-10-20 names the limits that keep the tool within what the publisher
tolerates: it reads and never writes the installation, because Easy Anti-Cheat blocks modified game
files, and it is used with the game closed, never against the running process. The project rules
already said so; the tool now enforces it.

- **Write guard.** `p4kx-guard` holds the protected folders: the Library folders (the default one
  and those the RSI Launcher log names, ADR-0005) and the folder of an opened Archive that holds the
  game's `Bin64` folder. A copy of an Archive somewhere else is not an installation, so its folder
  stays writable. The whole installation is protected, including `<Channel>\user\`: `p4kx chf
  from-json` cannot write into `CustomCharacters`, and the file is copied in by hand. The command
  line and the window register the Library folders once at program start, so that commands without
  an Archive are guarded too; opening an Archive adds its folder. `P4KX_PROTECT_FOLDERS`, a path
  list, adds further protected folders; it is a test aid for fake installations in a temp folder
  (no test targets the real game folders) and can only add protection, never remove it. The
  folders the user enters by hand in the settings are protected as well, as soon as they are loaded
  or added: a manual folder as a whole, because it is declared a Library or Channel folder, and a
  manual Archive by the `Bin64` rule. A Channel folder inside a Library folder, or equal to a
  manual folder, is therefore covered without a rule of its own.
- **Same folder, other route.** Besides the canonical key, the identity of the folder (volume and
  file index, through `same-file`) of the target's existing ancestors is compared with that of
  every protected folder, so a loopback share such as `\\localhost\D$\...` does not get past the
  guard.
- **Canonical comparison.** A target is made absolute, `.` and `..` are resolved, every existing
  component is canonicalised (symbolic links, junctions, `\\?\`, 8.3 names) and the rest is
  appended; for a target that does not exist yet its nearest existing ancestor decides. The
  components are compared case-folded on Windows and macOS. A run that writes below one target
  checks that target once, never per Entry; a conversion that writes single files
  (`write_replacing`) checks every file, which costs about 155 µs for a deep path. The shown
  installation path has no `\\?\` or `\\?\UNC\` prefix.
- **Limits.** The check and the write are two steps: a link that another process creates between
  them is not covered (TOCTOU). Inside a target, `create_folders` refuses a link that already
  exists on any level, and the folder cache of a run checks every level again on each use, so a
  link replaced during a run is refused too; the guard on the target and that check complement each
  other and neither replaces the other.
- **Typed refusal.** `WriteRefused` names the target and the installation. The services return it
  as `ExtractionError::Installation`, `ConversionError::Installation` and
  `QueryExportError::Installation`; the command line ends with exit code 2 before it creates
  anything, and the window shows a notice in the setup dialog and does not offer the start.
- **Read-only Archive.** The Archive file is opened for reading with shared read and write access
  and never with a write handle; a test opens a file whose read-only attribute is set.
- **Running game.** The check only lists process names: `qprocess` on Windows (it exits with 1
  when nothing matches), a PowerShell count of the matching processes where Windows Home has no
  `qprocess`, `/proc/*/comm` elsewhere. No handle to the game process is opened and no memory is
  read, which rules out libraries that open every process for its start time. The command line
  looks once per run, in the
  background; the window looks when an Archive is opened and then every 20 seconds. Reading goes on
  while the warning shows. `P4KX_GAME_CHECK=off` switches the look off for the command line; the
  test suite sets it so that a game on the developer's machine changes no output. Decided for
  issue #347.

## Considered Options

- `sysinfo` for the process list: well maintained, but on Windows it opens each process with
  `PROCESS_VM_READ` for its start time, which includes the game.
- The Toolhelp snapshot through `windows-sys` or the `tasklist` crate: names only, but it needs
  `unsafe` here (forbidden) or a crate that is not maintained.
- `tasklist.exe`: no handle either, but seconds slow on some systems.
- Protecting the folder of every opened Archive: simpler, but a copy of an Archive next to its
  output, as in every test, would be refused.
