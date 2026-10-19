# Entry folders fold ASCII case and separator kind

The window and `p4kx-mcp` each built their own folder view of an Archive, and the two disagreed:
the window kept folders whose names differ only by case apart (`Data\Objects` and
`Data\objects` were two folders), while `list_entries` folded them into one, and the two split
paths differently (the window only at `\`).

Both front-ends now list folders from one `EntryTree` in `p4kx-extraction`, built once per open
Archive, with one rule:

- Folder names compare ignoring ASCII case, and `/` and `\` are both separators, so spellings that
  differ only so are one folder. The game reads its files ignoring case, Windows extracts them
  into one folder anyway, and `find_entry`, the Filter and the Extraction's collision check already
  ignore ASCII case.
- A folder is named, and its path spelled, as its first Entry in path order spells it.
- Path order sorts Entries by path ignoring ASCII case with `\` as `/`, the rule of the Record
  order, and then byte by byte, so the order is total. The Entries of a folder and everything
  below it stay one contiguous range of Entry ids.
- Subfolders are ordered by name ignoring ASCII case; the Entries directly in a folder are kept
  per folder in path order, so a page of `list_entries` costs time in proportion to the page.

Non-ASCII case is not folded, as nowhere else in the Archive API; Unicode case folding would make
the order depend on a case table.

Split textures (D.26): a texture whose parts are spelled with a folder of another case than its
header (`Data\tex\wall.dds` and `data\tex\WALL.DDS.2a`) now has header and parts in one folder, so
the folder shows one row and counts the group once. Only parts in a genuinely different folder
still belong to the view of the header.

## Considered Options

- Keep case-different folders apart in both views: faithful to the bytes, but a folder lookup by
  path stays ambiguous and Extraction merges them on disk regardless.
- Fold only in `list_entries`: keeps the two views apart, which is what this decision ends.
