# Performance baseline

Timings of the Archive and Extraction on the LIVE Channel (Data.p4k of 151 GB, 1,379,308
Entries), measured on 2026-10-25 on an AMD Ryzen 9 9950X3D (32 logical processors) with the game
on an SSD, release build. The decisions they led to are in ADR-0137.

## How to run

The tests are `#[ignore]`d and print `BENCH` lines to stderr. Run them through the wrapper that
supplies the Archive and its key:

```powershell
& D:\Coding\claude-worktrees\p4k-extractor\tools\real-archive.ps1 -Worktree <worktree> test --release -j 4 -p <crate> --test real_archive --locked '--' --ignored --nocapture --test-threads 1 bench_
```

`<crate>` is `p4kx-archive`, `p4kx-extraction` or `p4kx-search` (its test file is `real_archive`
too). Times are the median of several runs after a warm-up; the Extraction and Record export are
single runs. Extraction numbers vary by about 20 percent between runs on a busy disk.

## Baseline

| Measurement | Before | After |
|---|---|---|
| Open the Archive and read the directory | 359 ms | unchanged |
| Read 2,000 random Entries of up to 1 MiB (137 MiB) | 294 ms (146 us each) | unchanged |
| Build the `PathIndex` | 36 ms | unchanged |
| `PathIndex::find`, per path | 1.4 us | unchanged |
| Scan for a path with `eq_ignore_ascii_case`, per path | 135 us | not used |
| `select_entries`, 1 / 100 / 1000 paths | 252 / 252 / 229 ms | 7.8 / 8.5 / 9.7 ms |
| `Filter::select`, substring `objects\spaceships\ships\aegs\` | 299 ms | 10.6 ms |
| `Filter::select`, `*.cgf` | 451 ms | 17.4 ms |
| `Filter::select_with_parts`, same substring | 15.9 ms | 10.9 ms |
| Content search, all 49,149 text Entries | 1.43 s | unchanged |
| Content search, `*objects/*.xml` (352 Entries) | 19 ms | unchanged |
| Read the DataForge Entry (`Game2.dcb`) | 537 ms | unchanged |
| Open the DataForge | 0.12 ms | unchanged |
| Export all 117,709 Records as XML | 53.2 s (2,214 Records/s) | 47.0 s (2,502 Records/s) |
| Extract 76,310 Entries (8,225 MiB) | 276, 267, 270 MiB/s | 325, 330, 331 MiB/s |
| Extract them again, every file exists | 13.2 s | 0.76 s |
| Extract 26,083 `.mtl` Entries with 1 / 2 / 4 / 8 / all threads | not measured | 11 / 17 / 22 / 21 / 18 MiB/s |

"Before" is the code of main at the time; the folder cache was measured by disabling it in
the same build. `select_with_parts` was already parallel; its change is the noise of a rerun.
