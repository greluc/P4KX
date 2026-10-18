use thiserror::Error;

/// Why a Comparison did not produce a result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ComparisonError {
    /// No level was selected.
    #[error("no Comparison level is selected")]
    NoLevel,
    /// The Comparison was cancelled through its [`CancelToken`](crate::CancelToken).
    #[error("the Comparison was cancelled")]
    Cancelled,
}
