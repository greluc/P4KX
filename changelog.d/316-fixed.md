### Fixed

- Vehicle implementations: a number with an invisible Unicode format character inside it, such as
  the U+202C in the Carrack's `mass`, is read in full instead of up to the character.
