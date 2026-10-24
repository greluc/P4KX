### Changed

- Models: a `HairPBR` material whose `TexSlot1` is an opacity map (`_opac`), such as hair cards,
  is now written alpha-masked with that map as the alpha of a white base colour image, in
  `entity glb`, `model glb` and `chf glb`, so the cards are cut out instead of opaque.
