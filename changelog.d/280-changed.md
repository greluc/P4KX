### Changed

- Textures: floating-point textures are written as OpenEXR `<name>.exr` instead of `<name>.png`,
  so their file names change; the `format` of `p4kx texture png --json` is `exr` instead of `png`
  for them, and four-channel ones keep their alpha in the file. The 8-bit preview and
  `--tone-map` output of these textures is tone-mapped instead of clamped to 0..1.
