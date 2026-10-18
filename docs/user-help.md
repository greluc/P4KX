# P4K-Extractor help

How to use the P4K-Extractor window. Terms follow [CONTEXT.md](../CONTEXT.md).

## Preview

Click an Entry's name in the table to preview it below the table. The Entry is read straight
from the Archive in the background, whatever its compression and whether it is encrypted, so
the window stays responsive while it loads.

- **Text** shows the Entry as text, line by line. UTF-8 is assumed; a UTF-8 or UTF-16 byte order
  mark is honoured. Bytes that are not valid text and control characters other than tabs and
  line breaks show as a middle dot `·`.
- **Hex** shows 16 bytes per row: the offset, the bytes in hexadecimal and the printable ASCII
  characters, with a dot for every other byte.
- **Copy** puts the open tab on the clipboard: the text, or the hex rows as shown.

### Size limits

| Limit | Value | What happens |
|---|---|---|
| Largest Entry previewed | 256 MiB | A larger Entry is not read; the preview says so. Extract it to look inside. |
| Bytes shown | 1 MiB | Only the start of a longer Entry is read and shown, with a note giving the bytes shown and the Entry's full size. Copy takes only what is shown. |

The limits are fixed.

## Comparing two Archives

The **Comparison** tab next to **Browse** compares two Archives, for example LIVE against PTU.

1. Pick the **Old Archive** and the **New Archive**. Each picker lists the detected Channels with
   their Build, the Recent Archives, and **Choose file…** for any other `Data.p4k`.
2. Switch on **Entries**, **Records** or both. The Record level reads the DataForge of each
   Archive and fails with the reason when one has none. The list shows the Entry changes; the
   Record changes go into the saved report.
3. Press **Compare**. Both Archives are opened and their central directories joined in the
   background, without decompressing anything; a progress bar shows the Entries joined, and
   **Cancel** stops at once.

The result shows both sides with Channel and Build, old first, so the direction of every change
is clear. An Entry is added when only the new Archive has it, removed when only the old one has
it, and changed when its size or CRC-32C differs; paths match ignoring case and separator kind.
The tiles count each kind, and **By directory** counts them per top-level directory.

The list below can be filtered with the rules of the search (a glob with `*` or `?`, otherwise
any part of the path) and sorted by any column; the counts follow the Filter. **Save report…**
writes the JSON report of what is shown, the same file `p4kx compare --report` writes with that
Filter (see [cli-json.md](cli-json.md)).

## DataForge

When the Archive holds DataForge (`Data\Game2.dcb`, or `Data\Game.dcb` in older Archives), a
**DataForge** node follows the folders of the path tree. Expanding or selecting it loads DataForge
in the background; a spinner and *Loading…* show on the node meanwhile, and the window stays usable.
The Records then appear as folders by record path: a Record alone at its record path is named after
the last part of its record path; several Records sharing one record path sit in a record folder
named after that part.
Hovering a Record shows its name, type and GUID.

DataForge is loaded once per open Archive and kept until you reload the Archive or it changes on
disk. A DataForge version this P4K-Extractor does not read, or a database it cannot read, is named
on the node; the rest of the window keeps working.

The **DataForge** tab of the preview shows the version, the numbers of Records, structs and enums,
and the first 10 Records in the order DataForge stores them.

## When the Archive changes

The RSI Launcher may update an Archive while it is open in the window. The window compares the
Archive's size, modification time and Build with the state it was opened in before every
Extraction and preview, and about once a second while it is open.

Once it changed, a notice with **Reload** appears below the menu. A running Extraction stops as
if cancelled, and its summary says that the Archive changed; files written before the change was
noticed may be stale. No Extraction or preview reads from the changed Archive until you reload
it, which opens it again as it is on disk now.

The command line behaves the same way: `p4kx extract` stops and exits with code 4.

## Installing and updating

Each Release on GitHub offers two downloads:

- **`P4K-Extractor-<version>-x86_64.msi`** installs for the current user only, without
  administrator rights, into `%LOCALAPPDATA%\Programs\P4K-Extractor`. It adds P4K-Extractor to the
  Start menu and that folder to your `PATH`, so `p4kx` works in a new terminal. A newer MSI
  replaces the installed version in place; uninstall through **Settings › Apps**.
- **`P4K-Extractor-<version>-x86_64-portable.zip`** holds `p4kx-gui.exe` and `p4kx.exe` to run
  from any folder, without installing anything.

Both keep their settings in `%APPDATA%\P4K-Extractor\settings.toml`, so switching between them or
uninstalling keeps them.

### Checking for a new version

**Help › Check for a new version at start** is off until you switch it on. While it is on, the
window asks the GitHub API once at start for the latest Release of P4K-Extractor; switching it on
also asks at once. When a newer Release exists, the status bar shows **Version … is available**,
which opens its page in your browser.

The request is a plain anonymous GET of
`https://api.github.com/repos/krt-profit/p4k-extractor/releases/latest`; it carries only the
program name and version as its user agent and nothing about you, your machine or your Archives.
It is the only network traffic of P4K-Extractor. A failed request shows nothing.
