# User-visible changes are recorded as fragments in changelog.d and assembled at the Release

Decided for issue #360.

Every pull request used to add its entry at the same place under `## [Unreleased]` in
`CHANGELOG.md`. After each merge every other open pull request conflicted and needed another
merge, test and CI round.

Now a user-visible change adds a Markdown file `changelog.d/<issue>.md` (or
`<issue>-<suffix>.md` for several per issue) instead of editing `CHANGELOG.md`. A fragment starts
with `### <Section>`, one of Added, Changed, Deprecated, Removed, Fixed, Security, and holds list
items in the style of `CHANGELOG.md`. The first-line heading was chosen over front matter because
it is the heading the entries get in the changelog, needs no parser and reads the same in a
review.

- `cargo xtask changelog`, and so plain `cargo xtask` in CI, checks every fragment: name pattern,
  known section, at least one list item, no headings and no loose text inside the items. It checks
  `CHANGELOG.md` as before.
- `cargo xtask changelog --release <version> [--date <YYYY-MM-DD>]` writes
  `## [<version>] - <date>` below an emptied `## [Unreleased]`: per section in the order Added to
  Security, first the entries that stood under `## [Unreleased]`, then the fragments by issue
  number. It then deletes the fragments. It refuses an invalid fragment, an existing version
  section and an empty release, and changes nothing in those cases. The date is today in UTC
  unless given.
- The entries already under `## [Unreleased]` stay; nothing is migrated. A Release is prepared
  by bumping the workspace version and running `--release` with it, before tagging; the tag
  workflow of ADR-0007 still takes its notes from the section of that version.
- `changelog.d/README.md` documents the form and is ignored by the checks.
