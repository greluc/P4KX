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
