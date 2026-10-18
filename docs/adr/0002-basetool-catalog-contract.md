# The Basetool catalog is a new, versioned contract owned by this repo

The KRT P4K Reader's `basetool-catalog.json` had no schema version, no provenance and no notion of
completeness, so the basetool import could neither validate it nor detect removed Records. The
P4K-Extractor defines a new catalog format: Rust types are the source, a JSON Schema (2020-12) is
generated from them and checked in, every catalog names its Channel and Build and states per kind
whether it is a complete snapshot. The basetool switches to the new format outright and drops the
old one; its import is adapted in its own repository.

## Considered Options

- Accept both formats in the basetool during a transition: rejected, the old format is produced by
  a tool whose source is lost and which will not be maintained.
- Let the P4K-Extractor also write the old format: rejected for the same reason.
