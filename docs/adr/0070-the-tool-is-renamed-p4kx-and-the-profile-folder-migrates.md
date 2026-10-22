# The tool is renamed P4KX and the profile folder migrates

Decided for issue #471. Builds on ADR-0004 (Record XML namespace) and ADR-0007 (MSI and update
check).

The owner renamed the tool from P4K-Extractor to P4KX and the repository to `greluc/P4KX`. v0.3.0
is not released yet, so users see one break together with the move from `krt-profit`.

## Decision

- The product name is **P4KX** wherever a user sees it. Crate names (`p4kx-*`) and binary names
  are unchanged.
- Repository links, the Record XML namespace and every published schema `$id` use
  `https://github.com/greluc/P4KX/…`. The changelog states one breaking change from the
  `krt-profit` identifiers straight to these.
- The MSI keeps its `UpgradeCode`, so an installed P4K-Extractor upgrades to P4KX. The install
  folder, the shortcut and the registry keys take the new name; the artefacts are
  `P4KX-<version>-x86_64.msi` and `P4KX-<version>-x86_64-portable.zip`.
- The profile folder is `P4KX` in the platform config directory (`%APPDATA%\P4KX` on Windows).
  On the first use per process, `p4kx_settings::migrate_profile` copies the old `P4K-Extractor`
  folder into it when the new one does not exist:
  - the old folder is never changed or deleted; it stays as a backup;
  - the copy is built in a staging folder and renamed to `P4KX` only when complete, so a file in
    an existing `P4KX` folder is never overwritten and a failed copy leaves no half-migrated
    folder (the next start retries);
  - only regular files and folders are copied; symbolic links and `.tmp` files are skipped, files
    above 16 MiB are skipped, and the copy stops at 1024 entries, 8 levels and 64 MiB;
  - the settings, the stored Archive key and any small cache file are copied as bytes; a failure
    to read the old folder never stops the start (the library has no logger, so it is not
    reported).
- The testkit's stored-key lookup reads `P4KX` first and falls back to `P4K-Extractor`.

## Consequences

- A consumer that checks the Record XML namespace or a schema `$id` must switch once.
- The old folder name stays in the code only in the migration and the testkit fallback.
