use std::fmt;

use p4kx_extraction::{Archive, ArchiveError, ArchiveKey, ArchiveKeyError};
use p4kx_settings::KeyStore;

use super::{Model, describe};

/// Why the Archive key dialog is open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyReason {
    /// The user opened it from the menu.
    Menu,
    /// An encrypted Entry was needed while the Archive had no key.
    Needed,
    /// The key in use does not decrypt the Archive.
    Rejected,
}

/// What is wrong with the key typed into the dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyProblem {
    /// The text is not 32 characters long; the number found.
    Length(usize),
    /// A character is no hexadecimal digit; its 1-based position.
    Character(usize),
    /// The key does not decrypt the open Archive.
    Wrong,
    /// The key could not be checked against the open Archive, for the reason given.
    Check(String),
    /// The key could not be stored in the profile, for the reason given.
    Store(String),
    /// The stored key could not be removed, for the reason given.
    Remove(String),
}

/// The Archive key dialog: the text typed and what is wrong with it.
#[derive(Clone, PartialEq, Eq)]
pub struct KeyDialog {
    text: String,
    reason: KeyReason,
    problem: Option<KeyProblem>,
    revealed: bool,
}

impl fmt::Debug for KeyDialog {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("KeyDialog")
            .field("reason", &self.reason)
            .field("problem", &self.problem)
            .finish_non_exhaustive()
    }
}

impl KeyDialog {
    /// Returns why the dialog is open.
    #[must_use]
    pub const fn reason(&self) -> KeyReason {
        self.reason
    }

    /// Returns what is wrong with the last key submitted, until the text changes.
    #[must_use]
    pub const fn problem(&self) -> Option<&KeyProblem> {
        self.problem.as_ref()
    }

    /// Returns the text typed.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Replaces the text typed and drops what was said about the one before.
    pub fn set_text(&mut self, text: String) {
        if text != self.text {
            self.text = text;
            self.problem = None;
        }
    }

    /// Returns whether the text typed is shown instead of hidden.
    #[must_use]
    pub const fn revealed(&self) -> bool {
        self.revealed
    }

    /// Shows or hides the text typed.
    pub const fn set_revealed(&mut self, revealed: bool) {
        self.revealed = revealed;
    }
}

#[derive(Debug, Default)]
pub(crate) struct KeyState {
    store: Option<KeyStore>,
    stored: Option<String>,
    key: Option<ArchiveKey>,
    dialog: Option<KeyDialog>,
    declined: bool,
}

impl Model {
    /// Sets where the key is stored in the user's profile and reads what is stored there.
    pub fn set_key_store(&mut self, store: Option<KeyStore>) {
        self.key.stored = store
            .as_ref()
            .and_then(|store| store.load().ok().flatten())
            .map(|key| key.masked());
        self.key.store = store;
    }

    /// Sets the key the Archives are opened with; `None` opens them without one.
    pub fn set_archive_key(&mut self, key: Option<ArchiveKey>) {
        if let Some(browser) = &self.browser {
            match &key {
                Some(key) => browser.archive_index().archive().set_key(key.clone()),
                None => browser.archive_index().archive().clear_key(),
            }
        }
        self.comparison.set_key(key.clone());
        self.key.key = key;
    }

    pub(super) fn key_in_use(&self) -> Option<ArchiveKey> {
        self.key.key.clone()
    }

    /// Returns whether a key is in use.
    #[must_use]
    pub const fn has_archive_key(&self) -> bool {
        self.key.key.is_some()
    }

    /// Returns the stored key with all but its last four digits hidden, or `None` when none is
    /// stored.
    #[must_use]
    pub fn stored_key_masked(&self) -> Option<&str> {
        self.key.stored.as_deref()
    }

    /// Returns the Archive key dialog, or `None` when it is closed.
    #[must_use]
    pub const fn key_dialog(&self) -> Option<&KeyDialog> {
        self.key.dialog.as_ref()
    }

    /// Returns the Archive key dialog for typing into it.
    pub const fn key_dialog_mut(&mut self) -> Option<&mut KeyDialog> {
        self.key.dialog.as_mut()
    }

    /// Opens the Archive key dialog for `reason`.
    pub fn open_key_dialog(&mut self, reason: KeyReason) {
        if reason == KeyReason::Menu {
            self.key.declined = false;
        }
        self.key.dialog = Some(KeyDialog {
            text: String::new(),
            reason,
            problem: None,
            revealed: false,
        });
    }

    /// Closes the dialog without a change; a dialog the window opened by itself does not open
    /// again until another Archive is opened.
    pub fn close_key_dialog(&mut self) {
        if let Some(dialog) = self.key.dialog.take() {
            self.key.declined |= dialog.reason != KeyReason::Menu;
        }
    }

    /// Opens the dialog when an encrypted Entry was needed while the Archive had no key that fits,
    /// unless the user closed it already for this Archive; returns whether it opened.
    pub fn poll_key_request(&mut self) -> bool {
        let archive_asked = self
            .browser
            .as_ref()
            .is_some_and(|browser| browser.archive_index().archive().take_key_request());
        let comparison_asked = self.comparison.take_key_request();
        if !(archive_asked || comparison_asked) || self.key.declined || self.key.dialog.is_some() {
            return false;
        }
        let reason = if self.key.key.is_some() {
            KeyReason::Rejected
        } else {
            KeyReason::Needed
        };
        self.open_key_dialog(reason);
        true
    }

    /// Checks the key typed against the open Archive and, when it fits, stores it in the profile,
    /// uses it from now on and closes the dialog; `wake` is called from the threads that read
    /// again what the missing key left unread. Returns whether the key was taken.
    pub fn submit_key_dialog(&mut self, wake: impl Fn() + Send + Sync + 'static) -> bool {
        let Some(dialog) = &self.key.dialog else {
            return false;
        };
        let key = match ArchiveKey::from_hex(&dialog.text) {
            Ok(key) => key,
            Err(error) => {
                self.key_problem(match error {
                    ArchiveKeyError::WrongLength { digits } => KeyProblem::Length(digits),
                    ArchiveKeyError::NotHexadecimal { position } => KeyProblem::Character(position),
                });
                return false;
            }
        };
        if let Some(problem) = self.check_key(&key) {
            self.key_problem(problem);
            return false;
        }
        if let Some(store) = &self.key.store {
            if let Err(error) = store.save(&key) {
                self.key_problem(KeyProblem::Store(describe(&error)));
                return false;
            }
            self.key.stored = Some(key.masked());
        }
        self.key.dialog = None;
        self.key.declined = false;
        self.set_archive_key(Some(key));
        if let Some(browser) = &mut self.browser {
            browser.key_changed(wake);
        }
        true
    }

    /// Removes the stored key and stops using the key in use; returns whether it was removed.
    pub fn remove_stored_key(&mut self) -> bool {
        let Some(store) = &self.key.store else {
            return false;
        };
        if let Err(error) = store.remove() {
            self.key_problem(KeyProblem::Remove(describe(&error)));
            return false;
        }
        self.key.stored = None;
        self.set_archive_key(None);
        true
    }

    fn check_key(&self, key: &ArchiveKey) -> Option<KeyProblem> {
        let archive: &Archive = self.browser.as_ref()?.archive_index().archive();
        match archive.verify_key(key) {
            Ok(_) => None,
            Err(ArchiveError::WrongKey { .. }) => Some(KeyProblem::Wrong),
            Err(error) => Some(KeyProblem::Check(describe(&error))),
        }
    }

    fn key_problem(&mut self, problem: KeyProblem) {
        if let Some(dialog) = &mut self.key.dialog {
            dialog.problem = Some(problem);
        }
    }

    pub(super) const fn key_archive_opened(&mut self) {
        self.key.declined = false;
    }
}
