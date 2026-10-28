<!--
Thanks for the pull request. Fill in the sections below and work through the checklist as far as
it applies. Sections marked "(if affected)" are optional; the reviewer checks them against the code.
A pull request that is not ready yet is opened as a Draft.
-->

## What's this about?

<!--
One to three sentences: what this changes and why (not what: the diff shows that).
Link the issue it resolves (`Closes #123`) and related ones (`Refs #456`).
-->

## Type of Change

- [ ] Bug fix
- [ ] Feature
- [ ] Breaking change (a command, flag, JSON field, file layout or MCP tool changes incompatibly)
- [ ] Refactor (no user-visible change)
- [ ] Documentation
- [ ] Build / CI / dependencies
- [ ] Tests

## Affected Parts

- [ ] Window (`p4kx-gui`)
- [ ] Command line (`p4kx`)
- [ ] MCP server (`p4kx-mcp`)
- [ ] Blender add-on (`blender_addon/`)
- [ ] Format crates (Archive, DataForge, CryXmlB, chunks, textures, audio, ...)
- [ ] Service crates (extraction, export, comparison, assembly, ...)
- [ ] Installer / Release (`.github/`, WiX)

## How was this tested?

<!--
Be concrete: which tests were added or run, against synthetic fixtures or a real Archive (which
channel), and for window changes which render was checked against the desktop spec. If something
could not be tested, say so explicitly.
-->
-

## Checklist

### General

- [ ] The issue is linked (`Closes #...`).
- [ ] The branch is up to date with `main`.
- [ ] A user-visible change adds a fragment `changelog.d/<issue>.md` ([form](../changelog.d/README.md)); `CHANGELOG.md` itself is not edited.
- [ ] I have signed the [Contributor License Agreement](../CLA.md) (once; see [`docs/cla-signatures.md`](../docs/cla-signatures.md)).
- [ ] Every commit carries a `Signed-off-by:` trailer (`git commit -s`, see [CONTRIBUTING.md](../CONTRIBUTING.md#developer-certificate-of-origin-dco)).
- [ ] No game files, extracted game assets, the Archive key or other secrets are in the diff.

### Code

- [ ] `cargo fmt --all`, `cargo clippy --workspace --all-targets --locked -- -D warnings` and `cargo xtask` pass.
- [ ] `cargo nextest run --workspace --locked` (or `cargo test --workspace --locked`) passes.
- [ ] Comments are doc comments or short technical comments of the current state, with no history; lint suppressions use `#[expect(lint, reason = "...")]`.
- [ ] No `unwrap` / `expect` outside tests; `thiserror` in libraries, `anyhow` only in binaries.
- [ ] Crate dependencies follow the layer direction (ADR-0003); source files stay below about 400 lines.
- [ ] Every new `.rs` file starts with the SPDX header.
- [ ] Terms follow [CONTEXT.md](../CONTEXT.md).

### Tests (if affected)

- [ ] New behaviour and fixes have tests, as modules of `tests/it/` where the crate is converted.
- [ ] Real-Archive tests are `#[ignore]`d and read the Archive through `p4kx_testkit`.

### Window (if affected)

- [ ] Colours, sizes, spacing, radii and type come from `p4kx_gui::theme` and match `docs/design/`.
- [ ] Every new string is in the Fluent files for `de-DE` and `en-GB`.
- [ ] Screenshots of the change are attached below.

### Documentation (if affected)

- [ ] `README.md`, `docs/user-help.md`, the format documents under `docs/` and, for a design decision, an ADR under `docs/adr/` move with the change.
- [ ] A dependency change regenerates `THIRD-PARTY-NOTICES.md` (`cargo xtask notices --write`).

## Screenshots (optional)

<!-- For window changes: before and after. -->

## Reviewer Notes

<!--
What should the reviewer look at closely? Where were trade-offs made deliberately? What is
explicitly not part of this pull request (with a link to the follow-up issue)?
-->
