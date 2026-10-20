### Fixed

- Models: a skin with a texture coordinate that is NaN or infinite (14 `.skinm` of LIVE, helmets
  among them) no longer fails to open; the coordinate reads as 0.
