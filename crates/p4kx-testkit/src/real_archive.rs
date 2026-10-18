use std::path::PathBuf;

/// The environment variable that names a real `Data.p4k` for the real-Archive suite.
pub const REAL_ARCHIVE_VAR: &str = "P4KX_DATA_P4K";

/// The real-Archive suite was run without `P4KX_DATA_P4K`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct MissingRealArchive;

impl std::fmt::Debug for MissingRealArchive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

impl std::fmt::Display for MissingRealArchive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{REAL_ARCHIVE_VAR} is not set; point it to a real Data.p4k to run the real-Archive suite"
        )
    }
}

impl std::error::Error for MissingRealArchive {}

/// Returns the path of the real Archive named by `P4KX_DATA_P4K`.
///
/// # Errors
///
/// Returns [`MissingRealArchive`] when the variable is unset or empty.
pub fn real_archive_path() -> Result<PathBuf, MissingRealArchive> {
    std::env::var_os(REAL_ARCHIVE_VAR)
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .ok_or(MissingRealArchive)
}
