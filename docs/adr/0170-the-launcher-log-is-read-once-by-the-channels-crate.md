# The launcher log is read once, by the Channels crate, within bounds

ADR-0053 takes the public patch from the RSI Launcher log; ADR-0005 limits the launcher files read
to `log.log` and `log.old.log`. Two crates had each their own copy of that rule, and every open of
an Archive, every Channel and the detection read the logs again.

`p4kx-channels` is the only crate that reads the launcher logs. It holds the file list, the folder
lookup and one scan that collects the library folders and the public patches in a single pass.
`p4kx-guard` takes the file list and the folder from there.

- `Build` is pure: `Build::with_public_patch(Option<String>)` sets the value, nothing is read, and
  `Archive::open` reads no launcher file. A test opens an Archive in a child process whose
  `P4KX_LAUNCHER_LOGS` names a log with the Build's change number and finds no public patch.
- `PublicPatches` is the change number to version map of one scan. `detect` and
  `detect_with_patches` read the logs once for all Channels; a front-end that needs the patch of
  an Archive's Build (`p4kx info`, the catalog and ship base `source`, the comparison and schema
  drift report, the Blender package, the MCP `archive_info`) gets the map and passes it on:
  `PublicPatches::standard()` in the CLI, the detection of the Channel list in the GUI, the launcher
  folder of the server in MCP. Library code takes the map in its options and never the environment.
- `P4KX_LAUNCHER_LOGS` names another log folder only for `PublicPatches::standard()`. Detection reads
  the logs of `SearchPaths::launcher`, so an in-process test that passes explicit paths never reads
  the developer's log.
- A line is read as bytes with its break stripped and never held beyond its limit. The tail and the
  64 KiB bound apply to the public patches only: only the last 16 MiB of a file count (the launcher
  rotates at 5 MiB) and a line over 64 KiB is skipped, so the newest line wins in an oversized log.
  The first line of a tail is dropped only when the tail starts mid-line.
- Library folders, on which the guard depends, are read from the whole file, with lines up to 1 MiB;
  only a longer line is skipped.
- A file that is not a regular file is not opened. A read error that is no interruption ends the
  file.
- A label counts when its version is non-empty numeric parts separated by dots and its change is
  1 to 20 digits. The map keeps the newest 4096 changes; the old log is folded in before the
  current one, so the current log wins.

The comparison report and the schema drift report state `public_patch` in `source.<side>.build`, the
MCP `BuildShape` carries it; both are additive.
