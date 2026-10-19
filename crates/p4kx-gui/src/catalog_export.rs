mod job;
mod write;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU8, AtomicUsize, Ordering};

use p4kx_export::CatalogSummary;

pub use job::{CatalogExport, CatalogExportStatus};
pub(crate) use job::{refused, start};

use crate::forge_cache::ReadGate;

/// The file name the save dialog of the Basetool catalog suggests.
pub const CATALOG_FILE: &str = "basetool-catalog.json";

/// The step a running Basetool catalog is at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatalogExportStage {
    /// DataForge is read from the Archive.
    Reading,
    /// The Records are checked and the catalog is built.
    Building,
    /// The catalog file is written.
    Writing,
}

/// How far a running Basetool catalog has come.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CatalogExportProgress {
    /// The step it is at.
    pub stage: CatalogExportStage,
    /// Records of the DataForge checked so far.
    pub done: usize,
    /// Records of the DataForge, known once building starts.
    pub total: usize,
}

/// The outcome of a Basetool catalog that was written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogExportSummary {
    file: PathBuf,
    summary: CatalogSummary,
}

impl CatalogExportSummary {
    /// Returns the catalog file written.
    #[must_use]
    pub fn file(&self) -> &Path {
        &self.file
    }

    /// Returns what the catalog holds: the counts per kind, the texts and the size.
    #[must_use]
    pub const fn summary(&self) -> &CatalogSummary {
        &self.summary
    }
}

#[derive(Debug, Default)]
struct Shared {
    stage: AtomicU8,
    done: AtomicUsize,
    total: AtomicUsize,
    gate: ReadGate,
}

impl Shared {
    fn enter(&self, stage: CatalogExportStage) {
        self.stage.store(stage as u8, Ordering::Relaxed);
    }

    fn progress(&self) -> CatalogExportProgress {
        let stage = match self.stage.load(Ordering::Relaxed) {
            0 => CatalogExportStage::Reading,
            1 => CatalogExportStage::Building,
            _ => CatalogExportStage::Writing,
        };
        CatalogExportProgress {
            stage,
            done: self.done.load(Ordering::Relaxed),
            total: self.total.load(Ordering::Relaxed),
        }
    }
}
