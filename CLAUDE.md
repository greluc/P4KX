# CLAUDE.md

Working notes for Claude Code (and human contributors) in this repo. Keep this file current when
the build, architecture, or the rules below change.

## What this is

**P4K-Extractor** — a Rust desktop tool with a native Rust GUI that opens, browses and extracts the
`Data.p4k` archive of Star Citizen. It reimplements:

- the functionality of [dolkensp/unp4k](https://github.com/dolkensp/unp4k) (reading the p4k
  archive, decompressing and decrypting its entries, and converting the CryEngine formats it
  contains, e.g. binary XML / DataForge), and
- the functionality of the **KRT P4K Reader**, the owner's earlier tool whose source code was lost.
  It is still installed locally and serves as the behavioural reference; its behaviour is
  reconstructed by observing the installed program, not from source.

## Real test data

Real p4k files live under `C:\Program Files\Roberts Space Industries\StarCitizen\<channel>\Data.p4k`,
one folder per channel (`LIVE`, `PTU`, `EPTU`, `HOTFIX`, `TECH-PREVIEW`). They are many gigabytes,
read-only and game assets: never copy them or anything extracted from them into the repository, and
never write into the game folders. Extraction output during development goes under
`D:\Coding\claude-worktrees\p4k-extractor\`.

Never read `loginData.json` or other launcher credential files in the game folders. Tests against a
real Archive are `#[ignore]`d and need `P4KX_DATA_P4K`; CI uses synthetic fixtures. Run them with
`TMP` and `TEMP` under `D:\Coding\claude-worktrees\p4k-extractor\` so the extracted subset lands
there:

```powershell
$env:TMP = 'D:\Coding\claude-worktrees\p4k-extractor\tmp'; $env:TEMP = $env:TMP
$env:P4KX_DATA_P4K = 'C:\Program Files\Roberts Space Industries\StarCitizen\LIVE\Data.p4k'
cargo test --release -p p4kx-archive -p p4kx-extraction -p p4kx-gui --test real_archive -- --ignored
```

A new real-Archive test gets `#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]`
and takes the path from `p4kx_testkit::real_archive_path`.

## Plan and decisions

- [docs/ROADMAP.md](docs/ROADMAP.md): product decisions and milestones M1–M14.
- [CONTEXT.md](CONTEXT.md): the glossary; use its terms in code, issues and commits.
- [docs/adr/](docs/adr/): architecture decisions.

## Architecture

- `p4kx-*` crates in three layers (ADR-0003): format crates depend only on `p4kx-foundation`,
  service crates on format crates, front-ends (`p4kx`, `p4kx-gui`, `p4kx-mcp`) only on services.
  Never add a dependency against this direction.
- Libraries are synchronous and parallelise with rayon; tokio only in `p4kx-mcp`.
- `thiserror` in libraries, `anyhow` only in binaries; no `unwrap` / `expect` outside tests.
- `unsafe_code = "forbid"` everywhere except the `cfg(windows)` DWM module in `p4kx-gui`, whose
  manifest repeats the workspace lints with `unsafe_code = "deny"`; `cargo xtask lints` checks
  that (ADR-0006).
- The look follows the KRT design system strictly: dark only, Lato only (ADR-0006). See
  "GUI design" below.
- Source files stay below about 400 lines; split by responsibility before that.
- One canonical Record tree serialised to XML and JSON; References are links, never embedded
  (ADR-0004).
- UI strings live in Fluent files for `de-DE` and `en-GB` (British spelling), never inline.

## GUI design

- **Binding source:** the current `main` of
  [krt-profit/design-system](https://github.com/krt-profit/design-system) is the leading and
  binding KRT design system for every GUI change. Read it fresh before GUI work, never from
  memory: `README.md` and `SKILL.md` for the rules, `tokens.json` for every colour, type size,
  weight, spacing, radius and glow, `krt-components.css` and `components/<Name>/preview.html`
  for the component contracts.
- `p4kx_gui::theme` mirrors those tokens 1:1 and draws the components after those contracts.
  Never invent a colour, size, spacing or radius; when the design system changes, the theme
  follows.
- Deviate only where egui or the platform forces it, never in colour, form or type, and record
  the deviation in an ADR.

## Rules

- **No comments in code** besides the language's documentation comments (Rust: `///` and `//!`),
  short and without history. Lint suppressions use `#[expect(lint, reason = "…")]`. The reasoning
  goes into the commit message and the PR body.
- Shell work uses PowerShell 7 (`pwsh`). Commit messages go through `git commit -s -F <file>`,
  PR and issue bodies through `--body-file <file>`.
- Worktrees and temporary files only under `D:\Coding\claude-worktrees\p4k-extractor\`.

## Agent skills

### Issue tracker

Issues live in GitHub Issues of `krt-profit/p4k-extractor`, accessed with the `gh` CLI. See
`docs/agents/issue-tracker.md`.

### Domain docs

Single-context: one `CONTEXT.md` and `docs/adr/` at the repo root. See `docs/agents/domain.md`.
