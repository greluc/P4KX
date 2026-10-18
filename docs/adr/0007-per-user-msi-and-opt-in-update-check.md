# Releases ship as a per-user MSI and a ZIP; the update check is opt-in and anonymous

A `v*` tag builds every Release on GitHub Actions: `cargo xtask release-notes` takes its notes
from `CHANGELOG.md` and fails unless the tag matches the workspace version; then a portable ZIP
and an MSI are built and published.

The MSI is built from `crates/p4kx-gui/wix/main.wxs` by `wix build` of WiX Toolset 7, pinned to`nan exact version and installed as a .NET tool in the workflow (`.github/scripts/build-msi.ps1`,
which also runs `wix msi validate`). WiX 7 is used under the OSMF EULA, accepted per build with`n`-acceptEula wix7`; its maintenance fee applies only above US$10,000 annual revenue. The MSI`ninstalls per user into `%LOCALAPPDATA%\Programs\P4K-Extractor` without administrator rights,
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
- cargo-wix 0.3 with WiX Toolset v3: no extra licence, but v3 is unsupported, and only older
  runner images still ship it.
- cargo-wix from its unreleased main branch, which drives WiX 4 and later: keeps `cargo wix`, but
  pins the build to an unreleased tool for a single `wix build` call.
