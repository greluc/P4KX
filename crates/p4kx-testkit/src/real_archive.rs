use std::path::PathBuf;

/// The environment variable that names a real `Data.p4k` for the real-Archive suite.
pub const REAL_ARCHIVE_VAR: &str = "P4KX_DATA_P4K";

/// The environment variable that names a second real `Data.p4k` for Comparisons.
pub const REAL_ARCHIVE_B_VAR: &str = "P4KX_DATA_P4K_B";

/// The real-Archive suite was run without the variable naming the Archive it needs.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct MissingRealArchive(&'static str);

impl std::fmt::Debug for MissingRealArchive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

impl std::fmt::Display for MissingRealArchive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} is not set; point it to a real Data.p4k to run the real-Archive suite",
            self.0
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
    path_from(REAL_ARCHIVE_VAR)
}

/// Returns the path of the second real Archive named by `P4KX_DATA_P4K_B`.
///
/// # Errors
///
/// Returns [`MissingRealArchive`] when the variable is unset or empty.
pub fn real_archive_b_path() -> Result<PathBuf, MissingRealArchive> {
    path_from(REAL_ARCHIVE_B_VAR)
}

fn path_from(var: &'static str) -> Result<PathBuf, MissingRealArchive> {
    std::env::var_os(var)
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .ok_or(MissingRealArchive(var))
}
