# Archive lookup and extraction throughput follow the benchmarks

Decided for issue #291, from the performance ideas of StarBreaker (MIT).

## Decision

**The benchmarks are ignored timing tests, not Criterion benchmarks.** A real Archive and its key
reach the test process only through `tools/real-archive.ps1`, which runs `cargo test` and
`cargo run` and nothing else, so `cargo bench` cannot be used. The tests are named `bench_*`, are
`#[ignore]`d like every real-Archive test and print `BENCH` lines to stderr through
`p4kx_testkit::bench` and `bench_once` (a warm-up, then the median and minimum of several runs;
one run for what is too slow to repeat). Run them with `--ignored --nocapture --test-threads 1
bench_`. They live next to the code they time: `p4kx-archive` (open, random Entry reads),
`p4kx-extraction` (path lookup, selection, Filter, DataForge open, full Record export,
Extraction throughput and threads) and `p4kx-search` (content search). The baseline numbers are
in `docs/performance.md`.

**Each idea is adopted only where a measurement showed a gain:**

- *Path index over the Entries, case-insensitive without allocation.* Already there:
  `PathIndex` holds the sorted case- and separator-folded 64-bit hashes of the paths with the
  Entry position (no copy of the keys) and verifies a hit against the stored path. A lookup costs
  about 1.4 us against 135 us for a scan; building it for 1.38 million Entries takes 36 ms. No
  change, and the `Archive` API is unchanged. `select_entries` did not use it and allocated two
  strings per Entry (250 ms); it now hashes each path without allocating, in parallel (8 ms).
- *Parallel name search.* `Filter::select` was the only serial scan; it is parallel now (300 ms to
  10 ms for a substring). The unordered AND of several tokens is not added: a glob such as
  `*a*b*` already ANDs in order, the scan is 10 ms, and a second matching syntax would differ
  between the CLI, the GUI and the MCP server.
- *Pre-created directories.* Implemented as a per-run cache of the folders already created and
  checked (`FolderCache`), not as a separate pre-pass, because the cache also serves Record
  export and keeps the check for links on the first use of each folder. A repeated Extraction of
  76,310 Entries fell from 13.2 s to 0.8 s, a first one from about 270 to about 330 MiB/s, the
  export of 117,709 Records from 53 s to 47 s.
- *Sort Entries by data offset.* Rejected: 70,162 of 76,309 neighbours of a Filter selection are
  already in offset order, and the sorted run was not faster than a repeat of the unsorted one
  (330 against 331 MiB/s).
- *`--max-threads`.* Implemented as `p4kx extract --threads <n>` and
  `ExtractOptions::threads`, a rayon pool of its own for the run. Small files stop gaining at
  about four threads (22 MiB/s at 4, 21 at 8, 18 with all), so the limit helps on a slow disk
  and when other programs need the processors. The sound workers follow the pool size.
- *Hint when a Filter matches nothing.* Implemented: the error names the rule the Filter broke
  (whole-path glob, surrounding whitespace, or the substring rules).

## Consequences

`ExtractOptions` gains the field `threads`, and `Entry::data_offset` is public. `PathIndex`,
`select_entries` and `Filter` keep their signatures and results.
