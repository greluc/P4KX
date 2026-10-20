use std::ffi::OsString;
use std::path::PathBuf;

use p4kx_foundation::ArchiveKey;

/// The environment variable that holds the key of a real `Data.p4k` for the real-Archive suite.
pub const REAL_ARCHIVE_KEY_VAR: &str = "P4KX_ARCHIVE_KEY";

/// The real-Archive suite was run without a usable `P4KX_ARCHIVE_KEY`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct MissingRealKey(bool);

impl std::fmt::Debug for MissingRealKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

impl std::fmt::Display for MissingRealKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0 {
            write!(
                f,
                "{REAL_ARCHIVE_KEY_VAR} is not 32 hexadecimal digits; the real-Archive suite needs the Archive key in it"
            )
        } else {
            write!(
                f,
                "{REAL_ARCHIVE_KEY_VAR} is not set; the real-Archive suite needs the Archive key in it"
            )
        }
    }
}

impl std::error::Error for MissingRealKey {}

/// Returns the Archive key named by `P4KX_ARCHIVE_KEY`.
///
/// # Errors
///
/// Returns [`MissingRealKey`] when the variable is unset, empty or not 32 hexadecimal digits.
pub fn real_archive_key() -> Result<ArchiveKey, MissingRealKey> {
    let text = std::env::var(REAL_ARCHIVE_KEY_VAR)
        .ok()
        .filter(|text| !text.trim().is_empty())
        .ok_or(MissingRealKey(false))?;
    ArchiveKey::from_hex(&text).map_err(|_| MissingRealKey(true))
}

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
    non_empty(std::env::var_os(REAL_ARCHIVE_VAR)).ok_or(MissingRealArchive(REAL_ARCHIVE_VAR))
}

/// Returns the path of the second real Archive named by `P4KX_DATA_P4K_B`, `None` when the
/// variable is unset or empty, so that a test needing it can return early.
#[must_use]
pub fn real_archive_b_path() -> Option<PathBuf> {
    non_empty(std::env::var_os(REAL_ARCHIVE_B_VAR))
}

/// Returns the paths of the real Archives named by `P4KX_DATA_P4K` and `P4KX_DATA_P4K_B`, `None`
/// when the second is unset or empty, so that a test over two Archives can skip.
///
/// # Errors
///
/// Returns [`MissingRealArchive`] when `P4KX_DATA_P4K` is unset or empty.
pub fn real_archive_pair() -> Result<Option<(PathBuf, PathBuf)>, MissingRealArchive> {
    let first = real_archive_path()?;
    Ok(real_archive_b_path().map(|second| (first, second)))
}

fn non_empty(value: Option<OsString>) -> Option<PathBuf> {
    value.filter(|path| !path.is_empty()).map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::path::PathBuf;

    use super::non_empty;

    #[test]
    fn an_unset_or_empty_variable_names_no_archive() {
        assert_eq!(non_empty(None), None, "unset");
        assert_eq!(non_empty(Some(OsString::new())), None, "empty");
    }

    #[test]
    fn a_set_variable_names_the_archive() {
        assert_eq!(
            non_empty(Some(OsString::from("b.p4k"))),
            Some(PathBuf::from("b.p4k")),
            "the path"
        );
    }
}
