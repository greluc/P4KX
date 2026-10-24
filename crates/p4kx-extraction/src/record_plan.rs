// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::fmt;
use std::marker::PhantomData;

use p4kx_dataforge::DataForge;

use crate::RecordFileFormat;
use crate::record_files::RecordFiles;

/// The file below the target folder of every Record of one DataForge in the format `F`
/// (ADR-0008), planned once and reused by every Record export from that DataForge.
pub struct RecordFilePlan<F> {
    files: RecordFiles,
    records: usize,
    bytes: usize,
    format: PhantomData<fn() -> F>,
}

impl<F: RecordFileFormat> RecordFilePlan<F> {
    /// Plans the file of every Record of `forge`; this reads every Record and belongs on a
    /// background thread.
    #[must_use]
    pub fn new(forge: &DataForge<'_>) -> Self {
        Self {
            files: RecordFiles::plan(forge, F::EXTENSION),
            records: forge.record_count(),
            bytes: forge.byte_len(),
            format: PhantomData,
        }
    }

    /// Returns whether the plan can belong to `forge`: both have the same number of Records and
    /// the same size in bytes.
    pub(crate) const fn fits(&self, forge: &DataForge<'_>) -> bool {
        self.records == forge.record_count() && self.bytes == forge.byte_len()
    }

    pub(crate) const fn files(&self) -> &RecordFiles {
        &self.files
    }
}

impl<F> fmt::Debug for RecordFilePlan<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RecordFilePlan")
            .field("records", &self.records)
            .field("bytes", &self.bytes)
            .finish_non_exhaustive()
    }
}
