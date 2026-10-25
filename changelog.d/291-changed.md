### Changed

- Extraction remembers the target folders it has made, still checking each is a real folder
  before it writes there, which makes a repeated Extraction of 76,000 Entries about 19 times
  faster on LIVE. Selecting Entries by path or by Filter now runs on all processors (about 30
  times faster on 32 of them).
