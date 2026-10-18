use thiserror::Error;

use crate::{DataForgeError, Guid};

/// Why a Comparison did not produce a result.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum ComparisonError {
    /// No level was selected.
    #[error("no Comparison level is selected")]
    NoLevel,
    /// The Record level was selected without the DataForges of both Archives.
    #[error("the Record level needs the DataForges of both Archives")]
    NoDataForge,
    /// The Comparison was cancelled through its [`CancelToken`](crate::CancelToken).
    #[error("the Comparison was cancelled")]
    Cancelled,
    /// A Record of one DataForge cannot be read.
    #[error("the Record {guid} of the {side} DataForge cannot be read")]
    Record {
        /// The side the Record belongs to, `old` or `new`.
        side: &'static str,
        /// The GUID of the Record.
        guid: Guid,
        /// What cannot be read.
        #[source]
        source: DataForgeError,
    },
}
