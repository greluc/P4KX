# Changelog fragments

A user-visible change adds one file here instead of editing `CHANGELOG.md`, so parallel pull
requests do not conflict. `cargo xtask changelog` checks the fragments; `cargo xtask changelog
--release <version>` folds them into the section of the version and deletes them (ADR-0035).

- Name: `<issue>.md`, or `<issue>-<suffix>.md` (lowercase letters, digits and hyphens) for several
  fragments of one issue, e.g. `345.md` and `345-fixed.md`.
- First line: `### <Section>` with the section `Added`, `Changed`, `Deprecated`, `Removed`,
  `Fixed` or `Security`. One section per file.
- Then one or more list items in the style of `CHANGELOG.md`: `- ` at the start of the line,
  continuation lines indented by two spaces, no blank lines between items, no headings.

```markdown
### Fixed

- Search: an empty filter no longer hides every Entry. The filter field keeps its text when the
  Archive changes.
```
