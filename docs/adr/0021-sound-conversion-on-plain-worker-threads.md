# Sound Conversion runs on plain worker threads within a byte budget

An Extraction with Conversion on converts WEMs and Soundbanks on a few plain worker threads of
its own (at most eight, scoped to the run), not on the rayon pool every other Entry uses; the
two run at the same time. Each worker holds one sound or Soundbank in memory at a time and first
takes its uncompressed size from a shared budget of 1 GiB, waiting while the budget is spent; a
larger one takes the whole budget and runs alone. This is the one exception to "libraries
parallelise with rayon" in `CLAUDE.md`. Decided for issue #97.

A rayon worker that waits for budget or decodes a sound can steal another job while it joins: the
IMA ADPCM and PTADPCM decoders split their blocks with rayon. A stolen sound would then hold its
bytes on a thread that already holds one, so neither a budget nor a pool size bounds memory, and
a blocking budget can deadlock once every lease holder waits inside such a join. Plain threads
never steal; the decoders' own rayon work still runs on the rayon pool, whose jobs take no
budget.

## Considered Options

- Sounds on the rayon pool with the `Budget` the UI movie Extraction uses for pixels: can
  deadlock and nest leases as above.
- A dedicated rayon pool of a few threads: no deadlock without a budget, but nested steals still
  let one thread hold several sounds, so memory is not bounded.
- Sounds one after another on one thread: bounded, but an Extraction of every sound of an
  Archive would decode tens of thousands of WEMs serially.
