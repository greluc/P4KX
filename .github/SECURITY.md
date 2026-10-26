# Security Policy

Thank you for taking the time to look at the security of P4KX. This document explains how to
report a vulnerability, which versions receive fixes, and what you can expect in return.

## Reporting a Vulnerability

**Please do not open a public GitHub Issue or Pull Request for anything you believe is
security-sensitive.** Public disclosure before a fix is available puts every user of P4KX at risk.

The preferred channel is GitHub's **Private Vulnerability Reporting**:

> **[Report a vulnerability](https://github.com/p4kx-org/p4kx/security/advisories/new)**

That form gives us a private place to triage the finding, work on a fix with you, request a CVE
and coordinate the disclosure.

A good report includes:

- A clear description of the issue and its impact.
- The affected version (Release tag, or commit SHA for a build from source) and how P4KX was
  installed (MSI, portable ZIP, built from source).
- The affected component: the window (`p4kx-gui`), the command line (`p4kx`), the MCP server
  (`p4kx-mcp`), the Blender add-on, the MSI installer, a GitHub Actions workflow, or a library
  crate by name.
- Reproduction steps, a proof of concept or a minimal input. A crafted input file is welcome; do
  **not** attach game files, extracted game assets or the Archive key.
- Your assessment of severity (a CVSS vector is welcome but not required).
- Whether you intend to publish your own write-up, and on what timeline.

If you cannot use GitHub's form, e-mail the maintainer at
[lucas.greuloch@gmail.com](mailto:lucas.greuloch@gmail.com) with the subject
*"Security report: P4KX"*. As a last resort, open a minimal public issue that says only *"I would
like to report a security issue, please contact me"*, without any technical detail, and we will
reach out privately.

## What to Expect

P4KX is a community project without a commercial SLA, but we aim for the following on every
report:

| Step                                                   | Target                    |
|--------------------------------------------------------|---------------------------|
| Acknowledge receipt of the report                      | within 7 days             |
| Initial triage and severity assessment shared with you | within 14 days            |
| Fix in `main` for High / Critical issues               | within 90 days            |
| Coordinated public disclosure after a fix is released  | within 14 days of release |

If we cannot meet a target we will tell you why and propose a new date in the advisory thread. We
credit you in the published advisory and the [changelog](../CHANGELOG.md) unless you ask us not to.

## Supported Versions

P4KX is in the `0.x` release line and under active development. Only the latest Release receives
security fixes; a fix ships in the next Release, older Releases get no backports.

| Version                       | Supported            |
|-------------------------------|----------------------|
| Latest Release and `main`     | :white_check_mark:   |
| Every older Release           | :x: (please upgrade) |

## Scope

The following are **in scope**:

- Source code in this repository: the crates under `crates/`, the Blender add-on under
  `blender_addon/`, the build scripts and `xtask`.
- The Release artefacts published on
  [GitHub Releases](https://github.com/p4kx-org/p4kx/releases): the MSI installer, the portable ZIP
  and the Blender add-on ZIP.
- GitHub Actions workflows under `.github/workflows/` and the actions they use.

Particularly interesting classes of issue, given what P4KX does:

- **Untrusted input.** P4KX parses large binary formats (the p4k Archive, DataForge, CryXmlB,
  CryEngine chunk files, textures, audio banks). An input that makes it read or write out of
  bounds, allocate without limit, loop forever or crash in a way an attacker can trigger with a
  crafted file is in scope.
- **Path traversal.** Extraction, export and the Blender package must never write outside the
  chosen output folder, whatever paths an Archive or a package contains.
- **Writes into the game installation.** P4KX is read-only towards Star Citizen; any way to make it
  write into a game folder is in scope.
- **The MCP server.** Any way for an MCP client to read or write files beyond what the server's
  tools document.
- **The Blender add-on.** Any way for a crafted package to run code or touch files outside the
  package it imports.
- **Update check.** The update check is one anonymous request to the GitHub API; anything that
  sends more about the user, the machine or the Archives, or that leads P4KX to run what it
  downloads, is in scope.
- **Secrets.** The Archive key or any other secret appearing in the repository, a Release, a log
  or an error message.
- **Supply chain.** Anything affecting the integrity of our build and release pipeline.

The following are typically **out of scope**:

- Vulnerabilities in third-party dependencies without a viable fix path in this project. Please
  report those upstream first; you are still welcome to tell us so we can track an upgrade.
- Findings that need an already-compromised machine or administrator rights on it.
- Reports generated solely by automated scanners without a demonstrated, reproducible impact.
- Anything concerning Star Citizen itself, its launcher, or services of Cloud Imperium, GitHub or
  other third parties. P4KX is an unofficial fan project; report game issues to Cloud Imperium.
- Best-practice suggestions without a concrete vulnerability. These are very welcome as regular
  issues or pull requests instead.

## Coordinated Disclosure

We follow coordinated disclosure. The default window is **90 days from the date we acknowledge the
report**, or sooner once a fixed Release is public. If a fix is not ready after 90 days we agree on
an extension with you before any public disclosure.

When the advisory is published we:

1. Publish a fixed Release.
2. Publish the GitHub Security Advisory with details, affected versions, workarounds and credit.
3. Add a `### Security` fragment under [`changelog.d/`](../changelog.d/README.md) referring to the
   advisory; the Release folds it into the [changelog](../CHANGELOG.md).

## Safe Harbor

We will not pursue or support legal action against researchers who:

- Make a good-faith effort to comply with this policy.
- Avoid privacy violations, data destruction and service degradation against users of P4KX.
- Use only their own systems and data, and stop as soon as they have demonstrated the issue.
- Give us reasonable time to fix the issue before any public disclosure.

This policy does not authorise you to test any third-party service, and it does not override the
Star Citizen End User Licence Agreement or Terms of Service.

## Thank You

Security research is real work and we appreciate it. If you report something that turns out to be
valid, we credit you in the advisory and the changelog by name, handle or anonymously, as you
choose.
