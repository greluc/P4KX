### Fixed

- Vehicle implementations: a file nested exactly one level deeper than the 128 allowed was read when
  it was CryXml but refused when it was text; both now read 128 levels and refuse 129.
