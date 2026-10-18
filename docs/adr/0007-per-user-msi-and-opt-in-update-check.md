# Releases ship as a per-user MSI and a ZIP; the update check is opt-in and anonymous

A `v*` tag builds every Release on GitHub Actions: `cargo xtask release-notes` takes its notes
from `CHANGELOG.md` and fails unless the tag matches the workspace version; then a portable ZIP
and an MSI are built and published.

The MSI comes from cargo-wix 0.3 and its WiX Toolset v3 template in `crates/p4kx-gui/wix/`. It
installs per user into `%LOCALAPPDATA%\Programs\P4K-Extractor` without administrator rights,
with a Start menu entry and the folder on the user `PATH`. Its UpgradeCode
`090C8F25-8543-42F0-B40A-1514653149F9` and the two component GUIDs never change: a new
UpgradeCode would install side by side instead of upgrading, and every Release is a major upgrade
of the one before. Before publishing, the workflow installs the MSI, upgrades it with a copy built
one patch version higher, checks that exactly one installation of the new version is left, and
uninstalls it.

The window can ask `api.github.com` for the latest Release. That is the only network traffic of
P4K-Extractor (no telemetry, no crash reports). The check is off until the user switches it on in
the Help menu, which is stored as `update_check` in the settings file. It is one anonymous GET
whose user agent names only the program and its version, and its result is a link in the status
bar, never a download. The service crate `p4kx-update` holds it; it uses `ureq` with `rustls`,
which needs no system TLS library on either platform and no async runtime.

## Considered Options

- An update check that is on by default: finds more users a new Release, but contacts GitHub
  before anyone agreed to it.
- Downloading and installing the new MSI from the window: convenient, but turns the check into an
  updater with its own signing and failure handling.
- WiX Toolset v4 or later built directly with `wix build`: newer toolset, but cargo-wix 0.3 only
  drives v3, and newer WiX releases require accepting an extra licence.
