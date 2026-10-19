# A loaded DataForge lives on its own thread

`DataForge<'a>` borrows the bytes of the Entry it is opened over, and builds its GUID, Record
order and root indexes lazily inside itself. The DataForge of the LIVE Archive is 321 MiB; the
MCP server answers many calls against it and must read it once per open Archive, keep it open with
its indexes, and hold at most one copy per Archive.

Keeping the bytes and the `DataForge` that borrows them in one cached value needs a
self-referential struct. `self_cell`, `ouroboros` and `yoke` expand `unsafe` blocks into the
crate that uses them, which `unsafe_code = "forbid"` (ADR-0006) rejects. Opening the DataForge
again for every call is cheap, but throws away its indexes, so every lookup by record path or
every Query would sort all Records again.

`LoadedDataForge` in `p4kx-extraction` therefore reads the Entry and moves its bytes to a thread
of its own, which opens the DataForge and keeps it on its stack. `with(work)` sends the work over
a channel; the thread runs each call on a scoped thread of its own against the shared
`&DataForge`, so calls run side by side and use rayon inside, and sends the result back. Dropping
the `LoadedDataForge` closes the channel; the thread waits for the calls still running and then
frees the DataForge and its bytes. `p4kx-mcp` keeps one `LoadedDataForge` next to each open
Archive, reads it on the first DataForge call while later calls wait for that read without
holding a worker permit, and drops it with the Archive when the Archive file changes. An Archive
that falls out of the cache while a call still holds it is found again, so there is never a second
copy. A read that failed because of the Entry's content is remembered; one that failed because of
the disk, a changed Archive or a missing thread is tried again by the next call.

The window keeps its own DataForge handling for now; it can move to `LoadedDataForge` when it
needs the indexes across runs.

## Considered Options

- Leak the bytes to get `'static`: frees nothing when the Archive changes, 321 MiB per reload.
- Make `DataForge` own its bytes: every view type of `p4kx-dataforge` borrows the database, so
  this changes the whole crate for one front-end.
- Open per call: simple, but rebuilds the indexes on every call.
