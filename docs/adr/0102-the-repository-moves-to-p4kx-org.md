# The repository moves to p4kx-org/p4kx

Decided for issue #557. Supersedes the repository part of ADR-0070.

The owner moved the repository from `greluc/P4KX` to the organisation `p4kx-org/p4kx`. GitHub
redirects the old name, but nothing should depend on that redirect.

## Decision

- Links to the repository use `https://github.com/p4kx-org/p4kx`. This covers the Cargo
  metadata, the update check's release API, the About text, the MSI, the Blender add-on manifest,
  the release workflow and the docs.
- The Record XML namespace becomes `https://github.com/p4kx-org/p4kx/record-xml`, and the Record
  JSON schema `$id` becomes `https://github.com/p4kx-org/p4kx/schemas/record-json.schema.json`.
  The owner chose to move the identifiers with the repository, so one name is used everywhere.
- The changelog states the identifier change as breaking against 0.3.0. Released changelog
  sections and ADR-0070 stay as written.

## Consequences

- Readers that match the 0.3.0 namespace or `$id` exactly must accept the new ones.
- The update check of 0.3.0 still finds new releases through GitHub's redirect.
