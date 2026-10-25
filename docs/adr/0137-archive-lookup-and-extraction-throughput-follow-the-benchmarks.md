# Archive lookup and extraction throughput follow the benchmarks

Decided for issue #291, from the performance ideas of StarBreaker (MIT).

## Decision

**The benchmarks are ignored timing tests, not Criterion benchmarks.** They need a real Archive
and its key, which CI does not have, and Criterion's repeated runs would write the extracted
files again and again. The tests are named `bench_*`, are `#[ignore]`d like every real-Archive
test and print `BENCH` lines to stderr through `p4kx_testkit::bench` and `bench_once` (a warm-up,
then the median and minimum of several runs; one run for what is too slow to repeat). Because
some write tens of gigabytes, they run only with `P4KX_BENCH=1` or a test filter starting with
`bench_`, so the command that runs every real-Archive test skips them. How to run them and the
baseline numbers are in `docs/performance.md`; they live next to the code they time:
`p4kx-archive` (open, random Entry reads), `p4kx-extraction` (path lookup, selection, Filter,
DataForge, full Record export, Extraction throughput and threads) and `p4kx-search` (content
search).

**Each idea is adopted only where a measurement showed a gain:**

- *Path index over the Entries, case-insensitive without allocation.* Already there:
  `PathIndex` holds the sorted case- and separator-folded 64-bit hashes of the paths with the
  Entry position (no copy of the keys) and verifies a hit against the stored path. A lookup costs
  about 1.4 us against 135 us for a scan. No change, and the `Archive` API is unchanged.
  `select_entries` did not use it and allocated two strings per Entry (252 ms); it now hashes
  each path without allocating and scans in parallel (8 ms on 32 logical processors).
- *Parallel name search.* `Filter::select` was the only serial scan; it is parallel now (299 ms
  to 10.6 ms on 32 logical processors; the gain scales with the cores). The unordered AND of
  several tokens is not added: a glob such as `*a*b*` already ANDs in order, the scan is 10 ms,
  and a second matching syntax would differ between the CLI, the GUI and the MCP server.
- *Pre-created directories.* Implemented as a per-run cache of the folders already created
  (`FolderCache`), not as a separate pre-pass. All Entry writers use it (plain, DataForge,
  sounds, soundbanks) and so does Record export. A cached folder is trusted only after a
  `symlink_metadata` of it and of every folder above it below the target says each still is a
  real folder, so a folder or parent swapped for a link or junction during the run is refused and
  a deleted one is made again; the checks cost nothing measurable. A race between the check and
  the write remains possible, as it was before the change. A
  repeated Extraction of 76,310 Entries fell from 13.2 s to 4.3 s; the first Extraction (about
  20 percent) and the Record export (12 percent) gained within the noise of single runs.
- *Sort Entries by data offset.* Rejected: 92 percent of the neighbours of a Filter selection
  were already in offset order, and a sorted run was not faster than a repeat of the unsorted one
  (330 against 331 MiB/s). The accessor used to test it is not kept.
- *`--max-threads`.* Implemented as `p4kx extract --threads <n>` and `ExtractOptions::threads`:
  a rayon pool of its own for the run, capped at the logical processors. The sound workers stay
  threads of their own, so that waiting for the memory budget never blocks a pool thread (a
  worker that holds a lease while its nested work steals another worker would deadlock the
  pool); only their conversion runs on the pool, so all compute stays within the limit. The wait
  for the budget ends when the run is cancelled. A
  pool that cannot be started fails every Entry with `ExtractionError::ThreadPool`. Small files
  stop gaining at about four threads (22 MiB/s at 4, 21 at 8, 18 with all), so the limit helps on
  a slow disk and when other programs need the processors. The Filter and selection scans before
  the run use the current pool.
- *Hint when a Filter matches nothing.* Implemented: the error names the rule the Filter broke
  (whole-path glob, surrounding whitespace, or the substring rules).

## Consequences

`ExtractOptions` gains the public field `threads`, so a struct literal without `..Default` stops
compiling; the one in the GUI sets `None`. `ExtractionError` gains the variant `ThreadPool`.
`PathIndex`, `select_entries` and `Filter` keep their signatures and results. `--threads`
bounds the threads that compute; the sound workers remain up to eight extra threads that mostly
wait for the budget or for the pool.
