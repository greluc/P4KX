# Contributing to P4KX

Thanks for taking the time to help improve P4KX, the tool that opens, browses, compares and
extracts the `Data.p4k` Archives of Star Citizen. This document explains how to ask questions,
report bugs, propose features and send pull requests, and the rules every change is held to.

The short version:

1. **Security issues never go into public issues**; use the confidential channel in
   [`.github/SECURITY.md`](.github/SECURITY.md).
2. **Bugs, features and tasks go through the
   [issue templates](https://github.com/p4kx-org/p4kx/issues/new/choose).**
3. **Never commit or attach game files, extracted game assets or the Archive key.**
4. **Pull requests branch from `main` and target `main`**, and pass `cargo fmt`, `cargo clippy`,
   `cargo xtask` and the tests locally before they are pushed.
5. **Every user-visible change adds a changelog fragment** under
   [`changelog.d/`](changelog.d/README.md) instead of editing [`CHANGELOG.md`](CHANGELOG.md).
6. **Sign the [Contributor License Agreement](CLA.md) once** before your first pull request, and
   **sign off every commit** ([DCO](#developer-certificate-of-origin-dco)): `git commit -s`.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Project documentation map](#project-documentation-map)
- [Asking questions](#asking-questions)
- [Reporting a security vulnerability](#reporting-a-security-vulnerability)
- [Reporting bugs](#reporting-bugs)
- [Suggesting features](#suggesting-features)
- [Game data and the Archive key](#game-data-and-the-archive-key)
- [Local development setup](#local-development-setup)
- [Pull requests](#pull-requests)
- [Contributor License Agreement (CLA)](#contributor-license-agreement-cla)
- [Developer Certificate of Origin (DCO)](#developer-certificate-of-origin-dco)
- [Commit messages](#commit-messages)
- [Style guides](#style-guides)
- [Before-you-push checklist](#before-you-push-checklist)
- [Licence](#licence)

## Code of Conduct

This project and everyone who takes part in it are governed by the
[Contributor Covenant 3.0 Code of Conduct](CODE_OF_CONDUCT.md). By taking part, you agree to uphold
it. Report unacceptable behaviour confidentially to
[lucas.greuloch@gmail.com](mailto:lucas.greuloch@gmail.com).

## Project documentation map

| Topic                                                              | Where                                                                  |
|:-------------------------------------------------------------------|:-----------------------------------------------------------------------|
| What P4KX does, building it, the commands                          | [`README.md`](README.md)                                               |
| Using the window                                                   | [`docs/user-help.md`](docs/user-help.md)                               |
| Product decisions and milestones                                   | [`docs/ROADMAP.md`](docs/ROADMAP.md)                                   |
| The glossary: the terms to use in code, issues and commits         | [`CONTEXT.md`](CONTEXT.md)                                             |
| Architecture decisions                                             | [`docs/adr/`](docs/adr/)                                               |
| Architecture rules, test layout, real-Archive suite, working rules | [`CLAUDE.md`](CLAUDE.md)                                               |
| The look of the window: theme, desktop spec, logo                  | [`docs/design/`](docs/design/)                                         |
| The formats P4KX reads and writes                                  | the documents under [`docs/`](docs/)                                   |
| Release notes                                                      | [`CHANGELOG.md`](CHANGELOG.md), unreleased ones in [`changelog.d/`](changelog.d/README.md) |
| Security reporting, supported versions, scope                      | [`.github/SECURITY.md`](.github/SECURITY.md)                           |

`CLAUDE.md` holds the working rules for human and AI contributors alike; read it before your
first pull request.

## Asking questions

Start with the [README](README.md) and the [user help](docs/user-help.md). If they do not answer
your question, open an issue and say what you tried.

## Reporting a security vulnerability

Do not open a public issue. Follow [`.github/SECURITY.md`](.github/SECURITY.md).

## Reporting bugs

Use the [bug report template](https://github.com/p4kx-org/p4kx/issues/new?template=bug_report.yml).
Before you open one:

- Use the latest Release; the bug may already be fixed.
- Search open and closed issues for duplicates.
- Note the P4KX version, how it was installed, the Star Citizen channel and Build, and the exact
  steps. Name Entry paths instead of attaching files.

## Suggesting features

Use the
[feature request template](https://github.com/p4kx-org/p4kx/issues/new?template=feature_request.yml).
Describe the problem first, then the solution you have in mind. Check
[`docs/ROADMAP.md`](docs/ROADMAP.md) to see whether it is planned already.

## Game data and the Archive key

P4KX reads your own Star Citizen installation and ships no game data.

- Never commit, attach or upload game files or anything extracted from them. Tests use synthetic
  Archives from `p4kx-testkit`.
- Never write into the game folders; P4KX itself refuses to.
- The Archive key is never part of the repository, in any spelling (ADR-0027). `cargo xtask keys`
  checks this.
- Never read or share the launcher's credential files.

## Local development setup

You need the stable Rust toolchain (see the [README](README.md#build)) on Windows or Linux, and
for the Blender add-on Python and Blender 5.2 LTS.

```powershell
cargo build --release
cargo fmt --all
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo nextest run --workspace --locked
cargo test --workspace --locked --doc
cargo xtask
```

`cargo test --workspace --locked` works as well when cargo-nextest is not installed. `cargo xtask`
checks the layer direction of the crates, the 400-line limit, the SPDX headers, the changelog
fragments, the lint set and the Archive key. After a dependency change, run
`cargo xtask notices --write` (needs `cargo install cargo-about --version 0.9.2 --locked`).

The tests against a real Archive are ignored by default; `CLAUDE.md` and the
[README](README.md#real-archive-suite) explain how to run them against your own installation.

## Pull requests

1. Open or pick an issue first, so the change is agreed before you spend time on it.
2. Branch from `main`.
3. Keep a pull request to one concern. Refactors that are not needed for the change go into a pull
   request of their own.
4. Fill in the [pull request template](.github/PULL_REQUEST_TEMPLATE.md) and link the issue with
   `Closes #<issue>`.
5. CI runs the lints, `cargo xtask`, the tests on Windows and Linux, the schema and concurrency
   tests, the Blender add-on tests, CodeQL and the DCO check. All must pass.
6. A maintainer checks your [CLA](#contributor-license-agreement-cla), reviews and merges with a
   merge commit.

## Contributor License Agreement (CLA)

Before your first contribution is merged, you sign the
[P4KX Individual Contributor License Agreement](CLA.md) once; it covers all your present and
future contributions. You keep the copyright in your contributions: the CLA grants the maintainers
a perpetual copyright and patent licence, in addition to the MIT and Apache-2.0 licences under
which your contribution is distributed.

To sign, open a pull request titled `CLA: sign — <your-github-handle>` that adds your row to
[`docs/cla-signatures.md`](docs/cla-signatures.md), with the acceptance sentence from
[§ 11 of the CLA](CLA.md#11-how-to-sign) in its description. If you contribute on behalf of an
employer, an Entity CLA is needed as well; ask
[lucas.greuloch@gmail.com](mailto:lucas.greuloch@gmail.com).

The CLA applies to every contribution that ends up in a commit on `main` under your authorship:
code, documentation, translations, workflows, typo fixes alike. A pull request from an author
without a signed CLA waits until it is signed.

## Developer Certificate of Origin (DCO)

In addition to the one-time CLA, every commit carries a `Signed-off-by` trailer that certifies the
[Developer Certificate of Origin, version 1.1](https://developercertificate.org/): that you wrote
the change or otherwise have the right to submit it under the project's licence. The CLA is the
lasting licence grant; the sign-off is the per-commit statement of origin.

```powershell
git commit -s
```

That appends `Signed-off-by: Your Name <you@example.org>`, taken from `git config user.name` and
`user.email`. To fix commits you already made:

- the last commit: `git commit --amend --signoff --no-edit`
- every commit since `main`: `git rebase --signoff main`

then update the pull request with `git push --force-with-lease`. Never force-push `main`.

The *DCO sign-off* job of the [Commit Gates](.github/workflows/commit-gates.yml) workflow checks
every commit of a pull request: its `Signed-off-by` trailer must match the commit's author name and
e-mail (the e-mail case-insensitively). Merge commits and Dependabot and GitHub Actions commits are
skipped.

## Commit messages

- The subject is one imperative sentence, starting with a capital letter, without a trailing full
  stop, at most about 72 characters. A prefix naming the area is welcome:
  `Assembly: choose the sub-geometry on replaced ports too`.
- The body explains *why*. The reasoning belongs here and in the pull request, not in code
  comments.
- Use the terms of [`CONTEXT.md`](CONTEXT.md) and British spelling.
- The footer carries the `Signed-off-by:` trailer.

## Style guides

### Rust

- `thiserror` in libraries, `anyhow` only in binaries; no `unwrap` / `expect` outside tests.
- `unsafe_code` is forbidden everywhere except the one module ADR-0006 allows.
- Libraries are synchronous and parallelise with rayon; tokio only in `p4kx-mcp`.
- Crates follow the three layers of ADR-0003: format crates, service crates, front-ends. Never add
  a dependency against this direction.
- Source files stay below about 400 lines; split by responsibility before that.
- Every `.rs` file starts with the two-line SPDX header.

### Comments

No comments in code besides doc comments (`///` and `//!`), and those are short and without
history. Lint suppressions use `#[expect(lint, reason = "...")]`.

### Window

The window follows P4KX's own theme strictly: dark only, Lato only. Every colour, size, spacing and
radius comes from `p4kx_gui::theme` and [`docs/design/`](docs/design/); a new value goes into both
together. Verify a visual change with a render against the desktop spec and attach screenshots.

### Strings

User-visible strings live in the Fluent files for `de-DE` and `en-GB` (British spelling), never
inline.

### Tests

A crate's integration tests are modules of one test binary, `tests/it/`; tests against a real
Archive are modules of `tests/real_archive/`. See "Test layout" in [`CLAUDE.md`](CLAUDE.md).

## Before-you-push checklist

- [ ] `cargo fmt --all` leaves nothing to change.
- [ ] `cargo clippy --workspace --all-targets --locked -- -D warnings` passes.
- [ ] `cargo xtask` passes.
- [ ] The tests pass.
- [ ] A user-visible change has its `changelog.d/` fragment.
- [ ] The CLA is signed.
- [ ] Every commit is signed off.
- [ ] No game data, extracted assets, Archive key or other secrets in the diff.

## Licence

P4KX is licensed under either of the [Apache License, Version 2.0](LICENSE-APACHE) or the
[MIT licence](LICENSE-MIT) at your option. Unless you state otherwise, any contribution you
intentionally submit for inclusion in P4KX is dual-licensed as above, without any additional terms
or conditions.
