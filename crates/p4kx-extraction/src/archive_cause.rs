// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::error::Error;

use p4kx_archive::ArchiveError;

use crate::{
    ConversionError, DataForgeLoadError, EntryReadError, ExtractionError, LocalizationLoadError,
};

/// Finds the [`ArchiveError`] behind `error`, looking through the causes of `error` and through
/// the error types of this crate that pass an Archive error on unchanged.
///
/// Front-ends use it to tell a missing or wrong Archive key from other failures, whichever API
/// the failure came through.
#[must_use]
pub fn archive_error_in<'a>(error: &'a (dyn Error + 'static)) -> Option<&'a ArchiveError> {
    let mut current: Option<&'a (dyn Error + 'static)> = Some(error);
    while let Some(cause) = current {
        if let Some(found) = archive_error_of(cause) {
            return Some(found);
        }
        current = cause.source();
    }
    None
}

fn archive_error_of<'a>(error: &'a (dyn Error + 'static)) -> Option<&'a ArchiveError> {
    if let Some(found) = error.downcast_ref::<ArchiveError>() {
        return Some(found);
    }
    match error.downcast_ref::<EntryReadError>() {
        Some(EntryReadError::Archive(found)) => return Some(found),
        Some(_) => return None,
        None => {}
    }
    match error.downcast_ref::<ConversionError>() {
        Some(ConversionError::Archive(found)) => return Some(found),
        Some(_) => return None,
        None => {}
    }
    match error.downcast_ref::<DataForgeLoadError>() {
        Some(DataForgeLoadError::Archive(found)) => return Some(found),
        Some(_) => return None,
        None => {}
    }
    match error.downcast_ref::<LocalizationLoadError>() {
        Some(LocalizationLoadError::Archive(found)) => return Some(found),
        Some(_) => return None,
        None => {}
    }
    match error.downcast_ref::<ExtractionError>() {
        Some(ExtractionError::Archive(found)) => Some(found),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use p4kx_archive::ArchiveError;

    use super::archive_error_in;
    use crate::{ConversionError, EntryReadError};

    fn key_required() -> ArchiveError {
        ArchiveError::KeyRequired {
            path: "Data.p4k".into(),
            entry: "a".to_owned(),
        }
    }

    #[test]
    fn archive_errors_are_found_directly_and_behind_passing_wrappers() {
        assert!(archive_error_in(&key_required()).is_some(), "direct");
        let read = EntryReadError::from(key_required());
        assert!(
            archive_error_in(&read).is_some_and(ArchiveError::is_key_required),
            "entry read"
        );
        let conversion = ConversionError::from(key_required());
        assert!(archive_error_in(&conversion).is_some(), "conversion");
    }

    #[test]
    fn other_errors_have_no_archive_error() {
        assert!(
            archive_error_in(&EntryReadError::Binary).is_none(),
            "binary"
        );
    }
}
