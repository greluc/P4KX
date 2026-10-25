### Changed

- GLB export: the inverse bind matrices of a skeleton are composed from the relative bone poses, so
  they agree with the node transforms even for a file whose stored world poses are inconsistent.
