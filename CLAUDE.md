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
real Archive run only when `P4KX_DATA_P4K` points to one; CI uses synthetic fixtures.

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
- `unsafe_code = "forbid"` everywhere except the `cfg(windows)` DWM module in `p4kx-gui`.
- Source files stay below about 400 lines; split by responsibility before that.
- One canonical Record tree serialised to XML and JSON; References are links, never embedded
  (ADR-0004).
- UI strings live in Fluent files for `de-DE` and `en-GB` (British spelling), never inline.

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
