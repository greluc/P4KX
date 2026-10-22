# The tool is renamed P4KX and the profile folder migrates

Decided for issue #471. Builds on ADR-0004 (Record XML namespace) and ADR-0007 (MSI and update
check).

The owner renamed the tool from P4K-Extractor to P4KX and the repository to `greluc/P4KX`. v0.3.0
is not released yet, so users see one break together with the move from `krt-profit`.

## Decision

- The product name is **P4KX** wherever a user sees it. Crate names (`p4kx-*`) and binary names
  are unchanged.
- Links to the repository (Cargo metadata, the update check, the About text, the installer, the
  docs) use `https://github.com/greluc/P4KX`. The Record XML namespace and every schema `$id` are
  identifiers, not links, and use the lower-case form `https://github.com/greluc/p4kx/…`. The
  changelog states one breaking change from the `krt-profit` identifiers straight to these.
- The MSI keeps its `UpgradeCode`, so an installed P4K-Extractor upgrades to P4KX. The install
  folder, the shortcut and the registry keys take the new name, and the two components get new
  GUIDs because their directories and key paths changed. The release workflow installs the v0.2.0
  MSI, upgrades it with the new one and checks that the old folder, `PATH` entry and registry key
  are gone. The artefacts are `P4KX-<version>-x86_64.msi` and
  `P4KX-<version>-x86_64-portable.zip`.
- The profile folder is `P4KX` in the platform config directory (`%APPDATA%\P4KX` on Windows).
  On the first use per process, `p4kx_settings::migrate_profile` copies the old `P4K-Extractor`
  folder into it when the new one does not exist:
  - the old folder is never changed or deleted; it stays as a backup. A link to it is followed,
    links inside it are skipped;
  - `settings.toml` and `archive.key` are copied first, as bytes;
  - the copy is built in a staging folder and renamed to `P4KX` only when complete, so a file in
    an existing `P4KX` folder is never overwritten and a failed copy leaves no `P4KX` folder;
    leftover staging folders are removed once `P4KX` exists;
  - a known file that cannot be copied, a file above 16 MiB, or more than 1024 entries, 8 levels
    or 64 MiB fail the migration; `.tmp` files are skipped;
  - when the migration fails, the old folder is the profile folder of that process, so nothing
    is written into a fresh `P4KX` that would hide the old folder from the next start; the next
    start copies again. The library has no logger, so the failure is not reported.
- The testkit's stored-key lookup reads `P4KX` first and falls back to `P4K-Extractor`.

## Consequences

- A consumer that checks the Record XML namespace or a schema `$id` must switch once.
- The old folder name stays in the code only in the migration and the testkit fallback.
