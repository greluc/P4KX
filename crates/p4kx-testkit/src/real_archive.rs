use std::ffi::OsString;
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

/// Returns the paths of the real Archives named by `P4KX_DATA_P4K` and `P4KX_DATA_P4K_B`, `None`
/// when the second is unset or empty, so that a test over two Archives can skip.
///
/// # Errors
///
/// Returns [`MissingRealArchive`] when `P4KX_DATA_P4K` is unset or empty.
pub fn real_archive_pair() -> Result<Option<(PathBuf, PathBuf)>, MissingRealArchive> {
    pair_from(
        std::env::var_os(REAL_ARCHIVE_VAR),
        std::env::var_os(REAL_ARCHIVE_B_VAR),
    )
}

fn path_from(var: &'static str) -> Result<PathBuf, MissingRealArchive> {
    non_empty(std::env::var_os(var)).ok_or(MissingRealArchive(var))
}

fn non_empty(value: Option<OsString>) -> Option<PathBuf> {
    value.filter(|path| !path.is_empty()).map(PathBuf::from)
}

fn pair_from(
    first: Option<OsString>,
    second: Option<OsString>,
) -> Result<Option<(PathBuf, PathBuf)>, MissingRealArchive> {
    let first = non_empty(first).ok_or(MissingRealArchive(REAL_ARCHIVE_VAR))?;
    Ok(non_empty(second).map(|second| (first, second)))
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::path::PathBuf;

    use super::{MissingRealArchive, REAL_ARCHIVE_VAR, pair_from};

    #[test]
    fn a_pair_needs_the_first_archive() {
        assert_eq!(
            pair_from(None, Some(OsString::from("b.p4k"))),
            Err(MissingRealArchive(REAL_ARCHIVE_VAR)),
            "the first Archive is required"
        );
        assert_eq!(
            pair_from(Some(OsString::new()), None),
            Err(MissingRealArchive(REAL_ARCHIVE_VAR)),
            "an empty first Archive is missing"
        );
    }

    #[test]
    fn a_pair_without_the_second_archive_skips() {
        for second in [None, Some(OsString::new())] {
            assert_eq!(
                pair_from(Some(OsString::from("a.p4k")), second),
                Ok(None),
                "no second Archive"
            );
        }
    }

    #[test]
    fn a_pair_holds_both_archives() {
        assert_eq!(
            pair_from(Some(OsString::from("a.p4k")), Some(OsString::from("b.p4k"))),
            Ok(Some((PathBuf::from("a.p4k"), PathBuf::from("b.p4k")))),
            "both Archives"
        );
    }
}
