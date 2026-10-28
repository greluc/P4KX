# The Release workflow attests, hashes and scans every asset

Decided for issue #822.

## Decision

- The Release workflow runs in separate jobs with the least permission each: `package` and `sbom`
  build, `scan` uploads to VirusTotal, `attest` holds `id-token: write` and `attestations: write`,
  `assemble` computes hashes and notes, and only `publish` holds `contents: write`.
- Every action is pinned to the full commit SHA of its newest stable release, with the version in a
  trailing comment. Checkout does not persist credentials. The tag must match
  `^v\d+\.\d+\.\d+(-[0-9A-Za-z.]+)?$`. actionlint and zizmor run in `workflow-lint.yml`.
- The CycloneDX SBOMs are generated per shipped binary with `cargo-cyclonedx` 0.5.9 (installed
  with `--locked`), because the tool writes one document per binary and no merge step is
  trusted. The Blender add-on ZIP has no SBOM: it bundles no third-party packages.
- `actions/attest` attests the ZIPs, the MSI and the SBOMs in one statement; `actions/attest-sbom`
  attests each SBOM against the portable ZIP and the MSI. The Sigstore bundles are Release assets,
  named `P4KX-<version>-provenance.sigstore.json` and `<sbom>.cdx.json.sigstore.json`, so a
  download verifies offline with `gh attestation verify <file> --bundle <bundle>`.
- `SHA256SUMS` and `SHA3-512SUMS` list every other asset, bundles and SBOMs included. SHA3-512 is
  computed with .NET on an Ubuntu runner, whose OpenSSL supports it.
- The VirusTotal scan covers the MSI and the portable ZIP, never runs on a pull request, and never
  blocks a Release; the notes state when it did not run.
- Release notes: the `CHANGELOG.md` section, the scan result, the verification commands, then
  GitHub's generated "What's Changed" list.

## Consequences

- A pull request that touches the workflow or its scripts builds and assembles everything but
  scans, attests and publishes nothing, so the tag-only steps are the only untested ones.
- The other workflows still use version tags; pinning them is a follow-up.
