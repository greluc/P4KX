//! Comparison service of P4K-Extractor: the Entries added, removed and changed between two
//! Archives, as a queryable result and as a JSON report with a generated schema.

mod compare;
mod comparison;
mod entry_change;
mod error;
mod options;
mod path_key;
mod report;

pub use compare::compare;
pub use comparison::{Comparison, Side};
pub use entry_change::{ChangeKind, DirectoryCounts, EntryChange, EntryCounts};
pub use error::ComparisonError;
pub use options::{CompareOptions, ComparisonProgress, Level, Levels};
pub use p4kx_archive::{Archive, ArchiveError, Build, Entry};
pub use p4kx_extraction::{CancelToken, Filter};
pub use report::{REPORT_FORMAT_VERSION, Report, report_schema};
