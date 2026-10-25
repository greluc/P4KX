// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::{Arc, Mutex, PoisonError};

use p4kx_assembly::{ArchiveFiles, Entities};
use p4kx_extraction::{Record, RecordTree};

use crate::CatalogFailure;
use crate::ship_base::{ShipRead, ShipReader};

/// The ship readers not in use. A reader keeps the shared Entities and containers it has read, so
/// a reader taken for one ship and given back for the next keeps its caches across the splits of
/// a parallel scan.
pub(crate) struct ShipPool<'f> {
    entities: Arc<Entities<'f>>,
    files: &'f ArchiveFiles<'f>,
    idle: Mutex<Vec<ShipReader<'f>>>,
}

impl<'f> ShipPool<'f> {
    pub(crate) const fn new(entities: Arc<Entities<'f>>, files: &'f ArchiveFiles<'f>) -> Self {
        Self {
            entities,
            files,
            idle: Mutex::new(Vec::new()),
        }
    }

    pub(crate) fn read(
        &self,
        record: &Record<'f>,
        tree: &RecordTree<'f>,
    ) -> Result<ShipRead, CatalogFailure> {
        let taken = self
            .idle
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .pop();
        let mut reader = taken
            .unwrap_or_else(|| ShipReader::with_entities(Arc::clone(&self.entities), self.files));
        let result = reader.read_with_digest(record, tree);
        self.idle
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(reader);
        result
    }
}
