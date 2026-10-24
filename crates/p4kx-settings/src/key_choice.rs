// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_foundation::{ArchiveKey, ArchiveKeyError};
use thiserror::Error;

use crate::{KeyStore, KeyStoreError};

/// Where the Archive key in use came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeySource {
    /// The `--key` option of the command line.
    Flag,
    /// The `P4KX_ARCHIVE_KEY` environment variable.
    Environment,
    /// The key stored in the user's profile.
    Profile,
}

/// The Archive key in use and where it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChosenKey {
    /// The key.
    pub key: ArchiveKey,
    /// Where it came from.
    pub source: KeySource,
}

/// Why no Archive key could be chosen; it never contains the key.
#[derive(Debug, Error)]
pub enum KeyChoiceError {
    /// The `--key` option is not a key.
    #[error("the --key option is not a valid Archive key")]
    Flag(#[source] ArchiveKeyError),
    /// The `P4KX_ARCHIVE_KEY` environment variable is not a key.
    #[error(
        "the {} environment variable is not a valid Archive key",
        crate::KEY_VARIABLE
    )]
    Environment(#[source] ArchiveKeyError),
    /// The stored key cannot be read.
    #[error(transparent)]
    Profile(#[from] KeyStoreError),
}

/// Chooses the Archive key: `flag` before `environment` before the key stored in `store`.
/// Empty values count as unset.
///
/// # Errors
///
/// Returns a [`KeyChoiceError`] for the first source that is set but holds no valid key, so that a
/// mistyped key is reported instead of silently replaced by a lower-ranking one.
pub fn choose_key(
    flag: Option<&str>,
    environment: Option<&str>,
    store: Option<&KeyStore>,
) -> Result<Option<ChosenKey>, KeyChoiceError> {
    if let Some(text) = set(flag) {
        let key = ArchiveKey::from_hex(text).map_err(KeyChoiceError::Flag)?;
        return Ok(Some(ChosenKey {
            key,
            source: KeySource::Flag,
        }));
    }
    if let Some(text) = set(environment) {
        let key = ArchiveKey::from_hex(text).map_err(KeyChoiceError::Environment)?;
        return Ok(Some(ChosenKey {
            key,
            source: KeySource::Environment,
        }));
    }
    Ok(match store {
        Some(store) => store.load()?.map(|key| ChosenKey {
            key,
            source: KeySource::Profile,
        }),
        None => None,
    })
}

fn set(text: Option<&str>) -> Option<&str> {
    text.filter(|text| !text.trim().is_empty())
}
