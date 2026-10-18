//! Comparison service of P4K-Extractor: the Entries added, removed and changed between two
//! Archives, as a queryable result and as a JSON report with a generated schema, and the line diff of
//! one Entry pair.

mod compare;
mod comparison;
mod decode;
mod entry_change;
mod error;
mod line_diff;
mod line_diff_report;
mod line_match;
mod options;
mod path_key;
mod report;
mod text_diff;

pub use compare::compare;
pub use comparison::{Comparison, Side};
pub use decode::{Encoding, MAX_DIFF_LEN, MAX_DIFF_LINES};
pub use entry_change::{ChangeKind, DirectoryCounts, EntryChange, EntryCounts};
pub use error::{ComparisonError, LineDiffError};
pub use line_diff::{DiffBody, LineDiff, NotText, line_diff};
pub use options::{CompareOptions, ComparisonProgress, Level, Levels};
pub use p4kx_archive::{Archive, ArchiveError, Build, Entry};
pub use p4kx_cryxml::Signature;
pub use p4kx_extraction::{CancelToken, Filter};
pub use report::{REPORT_FORMAT_VERSION, Report, report_schema};
pub use text_diff::{DEFAULT_CONTEXT, DiffLine, Hunk, LineKind, TextDiff};
