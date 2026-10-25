# Performance baseline

Timings of the Archive and Extraction on the LIVE Channel (Data.p4k of 151 GB, 1,379,308
Entries), measured on 2026-10-25 on an AMD Ryzen 9 9950X3D (32 logical processors) with the game
on an SSD, release build. The decisions they led to are in ADR-0137.

## How to run

The timing tests are `#[ignore]`d like every real-Archive test and print `BENCH` lines to stderr.
They need a real `Data.p4k` and its Archive key, so CI does not run them, and a few of them write
tens of gigabytes (the Extraction and Record export tests), so they run only when asked: set
`P4KX_BENCH=1`, or name a filter that starts with `bench_`.

```powershell
$env:P4KX_DATA_P4K = 'C:\Program Files\Roberts Space Industries\StarCitizen\LIVE\Data.p4k'
$env:P4KX_BENCH = '1'
cargo test --release -p p4kx-extraction --test real_archive --locked -- --ignored --nocapture --test-threads 1 bench_
```

Use `-p p4kx-archive` or `-p p4kx-search` (`--test real_archive`) for the other crates, and point
`TMP` and `TEMP` at a drive with room for the extracted files. The key comes from the usual
sources of the real-Archive suite (`P4KX_ARCHIVE_KEY`, `P4KX_KEY_FILE`, the stored key). They are
timing tests rather than Criterion benchmarks for the same reason: they need the real Archive and
its key, which CI does not have.

Times are the median of several runs after a warm-up; the Extraction, Record export and thread
runs are single runs. Extraction speeds vary by about 20 percent between runs on a busy disk, so
differences below that are not claimed.

## Baseline

"Before" is main at the time of the measurement; each row was run once or as the median of the
runs stated.

| Measurement | Before | After |
|---|---|---|
| Open the Archive and read the directory (median of 5) | 359 ms | unchanged |
| Read 2,000 random Entries of up to 1 MiB, 137 MiB (median of 5) | 294 ms (146 us each) | unchanged |
| Build the `PathIndex` (median of 5) | 36 ms | unchanged |
| `PathIndex::find`, per path (median of 20) | 1.4 us | unchanged |
| Scan for a path with `eq_ignore_ascii_case`, per path (median of 5) | 135 us | not used |
| `select_entries`, 1 / 100 / 1000 paths (median of 5) | 252 / 252 / 229 ms | 7.8 / 8.5 / 9.7 ms |
| `Filter::select`, substring `objects\spaceships\ships\aegs\` (median of 5) | 299 ms | 10.6 ms |
| `Filter::select`, `*.cgf` (median of 5) | 451 ms | 17.4 ms |
| `Filter::select_with_parts`, same substring | 15.9 ms | 10.9 ms (already parallel; within noise) |
| Content search, all 49,149 text Entries | 1.43 s | unchanged |
| Content search, `*objects/*.xml` (352 Entries) | 19 ms | unchanged |
| Read the DataForge Entry (`Game2.dcb`, median of 3) | 537 ms | unchanged |
| Open the DataForge over the bytes (median of 5; the open is lazy, the indexes are built on use) | 0.12 ms | unchanged |
| Export all 117,709 Records as XML (single run) | 53.2 s | 47.0 s |
| Extract 76,310 Entries, 8,225 MiB (three runs each) | 267 to 276 MiB/s | 325 to 362 MiB/s |
| Extract them again, every file exists (single run each) | 13.2 s | 4.3 s (every folder level checked) |
| Extract 26,083 `.mtl` Entries with 1 / 2 / 4 / 8 / all threads (single runs) | not applicable | 11 / 17 / 22 / 21 / 18 MiB/s |

The "before" rows of the Extraction come from the same build with the folder cache switched off;
every other "before" row comes from main before the change.

## Reading the numbers

- The 30-fold gains of `select_entries` and `Filter::select` come from running the scan on 32
  logical processors, plus, for `select_entries`, the end of two string allocations per Entry.
  With fewer cores the gain shrinks roughly in proportion; on 4 cores expect about 4 times for
  `Filter::select` (it was a serial scan) and somewhat more for `select_entries`. This was not
  measured on a smaller machine.
- The Record export gain (12 percent) and the first-run Extraction gain (about 20 percent) are
  single runs close to the run-to-run noise of the disk. The repeat Extraction, where only folder
  handling remains, is the clear result: the 13.2 s were file system calls for folders that
  already existed.
- The `symlink_metadata` of every folder level on each use (which keeps a folder or parent
  swapped for a link from being followed) is what remains of the folder work: 0.7 s for the
  repeat Extraction when only the leaf was checked, 4.3 s with every level, against 13.2 s
  before the cache. The first Extraction is unaffected within the noise (326 and 328 MiB/s).
