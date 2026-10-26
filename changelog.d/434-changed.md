### Changed

- Basetool catalog: `schemaVersion` is now 2, a breaking change for the Basetool import. A paint
  lists only the ships whose port and Record tags offer the tags the paint itself requires, so
  `paints[].shipGuids` has 40 fewer entries on LIVE 4.7 (ADR-0161); the Basetool must accept
  version 2 before it reads the new catalogs.
- Library: `p4kx_export::Kind::rule()` returns `Option<KindRule>`, `None` for the derived kinds
  `Paints`, `Loadouts` and `Shops`; `CatalogSummary::skipped()` no longer adds the leftovers of the
  derived kinds twice.
