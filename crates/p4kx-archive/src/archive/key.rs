use std::path::Path;

use p4kx_foundation::ArchiveKey;

use super::Archive;
use crate::key_check::{self, Verdict};
use crate::key_slot::KeySlot;
use crate::{ArchiveError, Entry, KeyCheck};

impl Archive {
    /// Opens the Archive at `path` like [`Archive::open`] and sets `key`, when given, as the key
    /// of its encrypted Entries; see [`Archive::set_key`].
    ///
    /// # Errors
    ///
    /// Returns an [`ArchiveError`] when the file cannot be read or is not a valid Archive.
    pub fn open_with_key(
        path: impl AsRef<Path>,
        key: Option<ArchiveKey>,
    ) -> Result<Self, ArchiveError> {
        let mut archive = Self::open(path)?;
        archive.key = KeySlot::with(key);
        Ok(archive)
    }

    /// Sets the key that decrypts the Entries of this Archive and of the containers opened from
    /// it, also those opened already.
    ///
    /// The key is checked once, when an encrypted Entry is first opened with it: a key that does
    /// not decrypt the smallest suitable encrypted Entry to data matching its checksum makes
    /// opening fail with [`ArchiveError::WrongKey`], from then on without checking again.
    /// [`Archive::check_key`] checks it at once.
    pub fn set_key(&self, key: ArchiveKey) {
        self.key.set(key);
    }

    /// Removes the key; encrypted Entries then fail to open with [`ArchiveError::KeyRequired`].
    pub fn clear_key(&self) {
        self.key.clear();
    }

    /// Returns whether a key is set.
    #[must_use]
    pub fn has_key(&self) -> bool {
        self.key.is_set()
    }

    /// Returns whether an encrypted Entry was needed since the last call while no key was set or
    /// the one set was found not to fit, and forgets that, so that a window can ask for the key
    /// once.
    pub fn take_key_request(&self) -> bool {
        self.key.take_wanted()
    }

    /// Checks the key that is set, which is [`KeyCheck::Unchecked`] when none is.
    ///
    /// # Errors
    ///
    /// Returns [`ArchiveError::WrongKey`] when the key does not decrypt the Archive's Entries, and
    /// another [`ArchiveError`] when the Entry used for the check cannot be read.
    pub fn check_key(&self) -> Result<KeyCheck, ArchiveError> {
        match self.key.get() {
            None => Ok(KeyCheck::Unchecked),
            Some((key, _)) => self.check_set_key(&key),
        }
    }

    /// Checks whether `key` decrypts this Archive's encrypted Entries, by decrypting the smallest
    /// suitable one and comparing the CRC-32C of the result with its directory. Nothing is set.
    ///
    /// # Errors
    ///
    /// Returns [`ArchiveError::WrongKey`] when the data does not match, and another
    /// [`ArchiveError`] when the Entry cannot be read.
    pub fn verify_key(&self, key: &ArchiveKey) -> Result<KeyCheck, ArchiveError> {
        let verdict = key_check::verify(&self.source, &self.entries, key)
            .map_err(|error| error.at(&self.location))?;
        match verdict {
            Verdict::Verified => Ok(KeyCheck::Verified),
            Verdict::Unchecked => Ok(KeyCheck::Unchecked),
            Verdict::Wrong { entry } => Err(ArchiveError::WrongKey {
                path: self.location.clone(),
                entry,
            }),
        }
    }

    fn check_set_key(&self, key: &ArchiveKey) -> Result<KeyCheck, ArchiveError> {
        match self.verify_key(key) {
            Ok(check) => {
                self.key.record(key, Ok(()));
                Ok(check)
            }
            Err(ArchiveError::WrongKey { path, entry }) => {
                self.key.record(key, Err(entry.clone()));
                Err(ArchiveError::WrongKey { path, entry })
            }
            Err(error) => Err(error),
        }
    }

    /// Returns the key to decrypt `entry` with, `None` for an unencrypted Entry.
    pub(super) fn key_for(&self, entry: &Entry) -> Result<Option<ArchiveKey>, ArchiveError> {
        if !entry.encrypted {
            return Ok(None);
        }
        let Some((key, outcome)) = self.key.get() else {
            self.key.want();
            return Err(ArchiveError::KeyRequired {
                path: self.location.clone(),
                entry: entry.path.clone(),
            });
        };
        match outcome {
            Some(Ok(())) => {}
            Some(Err(entry)) => {
                return Err(ArchiveError::WrongKey {
                    path: self.location.clone(),
                    entry,
                });
            }
            None => {
                self.check_set_key(&key)?;
            }
        }
        Ok(Some(key))
    }
}
