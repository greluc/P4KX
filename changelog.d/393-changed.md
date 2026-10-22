### Changed

- Basetool catalog: the items carry `tagGuids` beside `tags`, the GUIDs of the Tag Records, and
  the window suggests `basetool-catalog.json.zst` as the file name and lists the compressed form
  first. Plain `.json` stays selectable but is over the 64 MB upload limit; a warning advises
  `.json.zst`.
