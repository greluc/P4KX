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

## When the Archive changes

The RSI Launcher may update an Archive while it is open in the window. The window compares the
Archive's size, modification time and Build with the state it was opened in before every
Extraction and preview, and about once a second while it is open.

Once it changed, a notice with **Reload** appears below the menu. A running Extraction stops as
if cancelled, and its summary says that the Archive changed; files written before the change was
noticed may be stale. No Extraction or preview reads from the changed Archive until you reload
it, which opens it again as it is on disk now.

The command line behaves the same way: `p4kx extract` stops and exits with code 4.
