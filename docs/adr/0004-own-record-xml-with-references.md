# Own Record XML: references are links, never embedded

unp4k and the KRT P4K Reader expand referenced Records inline up to configurable depth limits, which
duplicates data, makes output depend on settings and turns small changes into large diffs. The
P4K-Extractor serialises one canonical Record tree to XML and JSON alike: owned children (strong
pointers) are nested, every reference to another Record is a link carrying its GUID, record path and
type. The XML is therefore not unp4k-compatible and needs no depth limits; the Comparison works on
the same tree.

## Considered Options

- unp4k-compatible XML: keeps existing scripts working, but inherits the duplication and the limits.
- Embedding with a depth limit as an option: two output shapes for the same Record.
