use p4kx_extraction::{
    DataForgeError, DataForgeLoadError, LocalizationError, LocalizationLoadError,
};
use thiserror::Error;

/// A Basetool catalog cannot be built.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum CatalogError {
    /// The DataForge Entry cannot be found or read.
    #[error(transparent)]
    Load(DataForgeLoadError),
    /// The DataForge has an unsupported version or is corrupt.
    #[error(transparent)]
    DataForge(DataForgeError),
    /// A `global.ini` the Archive holds cannot be read.
    #[error(transparent)]
    LoadTexts(LocalizationLoadError),
    /// A `global.ini` is corrupt or exceeds a limit.
    #[error("cannot read the localisation file {path}")]
    Texts {
        /// The Entry path.
        path: String,
        /// Why it cannot be read.
        #[source]
        source: LocalizationError,
    },
    /// The catalog cannot be written as JSON.
    #[error("cannot write the catalog as JSON")]
    Json(#[source] serde_json::Error),
    /// The catalog cannot be written to its output.
    #[error("cannot write the catalog")]
    Write(#[source] std::io::Error),
    /// The build was cancelled.
    #[error("the catalog build was cancelled")]
    Cancelled,
}
