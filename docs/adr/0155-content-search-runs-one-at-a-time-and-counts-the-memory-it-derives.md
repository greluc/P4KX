# Content search runs one at a time and counts the memory it derives

Decided for issue #387, the follow-ups of the review of PR #382. Amends ADR-0044.

## Decision

- **One search at a time per tab.** Cancelling a search drops its `Run`, which only sets a flag; the
  thread ends when it next polls. A search started at once used to run beside it, and the two
  threads together held two memory budgets. The tab now owns a lane: every search thread enters it
  before it reads anything and holds its turn until it ends, so the next search starts when the
  cancelled one has ended. A search that is cancelled while it waits for its turn ends without
  having read anything.
- **The memory budget counts what is derived from an Entry.** ADR-0044 said "a budget of 256 MiB
  for all tasks together" but the gate reserved only the Entry's size. A `CryXml` Entry also holds
  what the parser derives from it: a node of 28 bytes becomes 28 bytes of node and 8 of links, an
  attribute of 8 bytes becomes 16, and a byte of string data becomes at most 5 (one if the strings
  are copied to repair invalid UTF-8, and four for the offset of a terminator, if every byte is a
  terminator). That bound holds only when no byte belongs to two tables, so the `CryXml` parser
  refuses a file whose node, attribute, child and string tables overlap
  (`CryXmlError::TablesOverlap`); a crafted file can no longer have its strings counted again as
  nodes. With the file itself the gate reserves six times its size. A UTF-16 Entry reserves its size and the 64 KiB buffer
  that decodes it. Streamed UTF-8 reserves nothing: its scratch space is one read buffer and one
  line of at most 1 MiB for each thread. The budget stays 256 MiB, and one Entry larger than the
  budget still runs alone.
- **The DataForge read can be cancelled.** `DataForgeEntry::read_until` polls a stop function
  before each mebibyte and returns `DataForgeLoadError::Cancelled`; `ForgeCache::read_until`
  also stops waiting for the read of another consumer. A read that was stopped stores nothing, so
  the next consumer reads it again, except that a read is not stopped while another consumer, such
  as the Record tree, already waits for it: it goes on and is kept, so it is not started twice.
  `read` behaves as before.
- **Lines end at a lone carriage return too.** The line sink and the Text preview split at a line
  feed, a carriage return and line feed, or a lone carriage return, and a pair split between two
  reads is one break, so line numbers and byte offsets of a hit are those of the preview. The XML
  view of converted `CryXml` splits only at line feeds, as before, and the sink of that form does
  the same.
- **Case folding.** A needle may end inside the expansion of a character: `i` finds `İ`, which
  lower-cases to `i` and a combining dot. The final sigma `ς` is folded to `σ` on both sides. `ß`
  is still not folded to `ss`. The reverse does not hold: `istanbul` does not find `İstanbul`,
  because the combining dot (U+0307) stays between the `i` and the `s`.
- **Excerpts are safer.** Line and paragraph separators (U+2028, U+2029), zero-width characters
  (U+200B to U+200D, U+2060 to U+2064), the soft hyphen (U+00AD), U+180E and the byte order mark
  (U+FEFF) become U+FFFD like the other controls.
- **A hit opens at its line.** Opening an Entry hit scrolls the Text or XML preview to the line of
  the hit once, with one line above it. The browser counts the hits it opens and remembers the one
  a view scrolled to last, so two hits in one Entry both scroll, and a view that is shown again
  does not scroll again. A Text preview that holds only the start of a large Entry says so in its
  note of bytes shown; a hit line beyond it scrolls to the end of what is shown.
- **Lookup by path is an array.** `EntryIndex::id_of_path` reads the id from the table that maps
  the position of an Entry in the Archive to its id, which the index built anyway, instead of
  scanning every Entry.
- **Left as it is.** A `,P` hit selects the row of the base key: the table has one row for a key
  and its suffixed lines, and shows the plain text. The minimum width of a window is left to
  #358: the work surface is at least 720 px wide and scrolls below it, so at 640 px the Search
  button and "Text kopieren" are reached by scrolling, as the desktop spec says. A test checks that
  both fit the surface at its minimum.

## Measured

On LIVE 4.10, release mode, all text Entries without the Archive key, peak heap above the opened
Archive by a counting allocator: 392 MiB before and 142 MiB after, in 2.4 s before and 2.5 s after
on a warm file cache. Every one of the real `CryXml` Entries still converts with the tables of
each kept apart.

## Considered Options

- **Share one memory gate between runs** instead of a lane. A cancelled run would still compete
  for the disk and the threads while it winds down, and the gate would have to live with the
  Archive. A lane costs the new search the time the old one needs to see its flag.
- **Count the real size of the parsed tables** after the parse. The reservation must be taken
  before the Entry is read, so a bound from the Entry size is the only figure available then.
