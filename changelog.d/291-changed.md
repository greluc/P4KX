### Changed

- Extraction creates and checks each target folder once per run instead of once per file, which
  makes a repeated Extraction of 76,000 Entries about 17 times faster and a first one about 20
  percent faster on LIVE. Selecting Entries by path or by Filter is about 30 times faster.
