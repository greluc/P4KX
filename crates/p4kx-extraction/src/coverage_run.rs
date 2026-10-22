use p4kx_archive::Archive;
use p4kx_dataforge::DataForge;

use crate::coverage::{CoverageReport, ReferenceList};
use crate::coverage_entries;
use crate::coverage_forge::{self, CoverageError};

impl CoverageReport {
    /// Checks what `forge` and `archive` hold of `list`: for every Record type whether it is
    /// defined, how many Records it has, whether they read without error and whether a sample
    /// exports; for every component how many `EntityClassDefinition` Records and instances have
    /// it and whether a Component selector Query reaches it; for every Entry folder whether its
    /// Entries convert from CryXml. `forge` is the DataForge of `archive`.
    ///
    /// # Errors
    ///
    /// Returns a [`CoverageError`] when the components cannot be counted.
    pub fn of(
        archive: &Archive,
        forge: &DataForge<'_>,
        list: &ReferenceList<'_>,
    ) -> Result<Self, CoverageError> {
        let (types, components) = coverage_forge::check(forge, list.items())?;
        let entries = coverage_entries::check(archive, list.items());
        Ok(Self {
            types,
            components,
            entries,
        })
    }
}
