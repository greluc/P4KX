use p4kx_archive::Archive;
use p4kx_dataforge::DataForge;

use crate::coverage::{CoverageReport, ReferenceList};
use crate::coverage_entries;
use crate::coverage_forge;

impl CoverageReport {
    /// Checks what `forge` and `archive` hold of `list`: for every Record type whether it is
    /// defined, how many Records it has, whether they read without error and whether a sample
    /// exports; for every component how many `EntityClassDefinition` Records and instances have
    /// it, derived structs included, and whether a Component selector Query reaches it; for every
    /// Entry folder whether its Entries convert from CryXml. `forge` is the DataForge of
    /// `archive`.
    #[must_use]
    pub fn of(archive: &Archive, forge: &DataForge<'_>, list: &ReferenceList<'_>) -> Self {
        let found = coverage_forge::check(forge, list.items());
        Self {
            types: found.types,
            components: found.components,
            objects: found.objects,
            entries: coverage_entries::check(archive, list.items()),
            unreadable_entities: found.unreadable_entities,
        }
    }
}
