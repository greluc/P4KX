# Third-party notices are a Markdown document generated from the JSON of cargo-about

Decided for issue #820.

## Decision

- The notices are `THIRD-PARTY-NOTICES.md`. The hand-written part (material that is not a Rust
  crate) sits above the line `<!-- GENERATED BELOW THIS LINE -->`; `cargo xtask notices --write`
  rewrites everything below it, and `cargo xtask notices` fails when that part is stale.
- The generated part is rendered by `xtask` from `cargo about generate --format json`, not from a
  Handlebars template, because a template cannot choose a fence longer than the backtick runs of a
  licence text. It holds a table of every crate (version, licences, link) and one section per
  licence text with its users; each text sits in a fenced block whose fence is longer than any
  backtick run at the start of a line of the text. `about.hbs` is gone.
- P4KX's own licences are also shipped as `LICENSE-MIT.md` and `LICENSE-APACHE.md`. The plain
  files stay because GitHub's licence detection, `Cargo.toml` and the SPDX headers rely on them.
  `cargo xtask licences` (part of the plain `cargo xtask`) requires each `.md` to hold the same
  words as its plain file, ignoring Markdown heading marks and layout.
- The portable ZIP and the MSI ship `THIRD-PARTY-NOTICES.md` and both `.md` licences next to the
  plain licence files.

## Consequences

- Fixed rendering and a stricter format at the price of Rust code in `xtask` instead of a template.
- Crates without a licence file (`dasp_sample`, `enum-map`, `enum-map-derive`) stay out of the
  generated part and are named by hand in the head, as before. `difflib`, a test-only dependency
  whose licence file is an unfilled template, is left out too.
