// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::fmt;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

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
    pending: bool,
}

impl fmt::Debug for KeyDialog {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("KeyDialog")
            .field("reason", &self.reason)
            .field("problem", &self.problem)
            .field("pending", &self.pending)
            .finish_non_exhaustive()
    }
}

impl KeyDialog {
    /// Returns whether the key submitted is being checked against the Archive.
    #[must_use]
    pub const fn pending(&self) -> bool {
        self.pending
    }

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

type Wake = Arc<dyn Fn() + Send + Sync>;

struct Check {
    result: Receiver<(ArchiveKey, Option<KeyProblem>)>,
    wake: Wake,
}

impl fmt::Debug for Check {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Check").finish_non_exhaustive()
    }
}

#[derive(Debug, Default)]
pub(crate) struct KeyState {
    store: Option<KeyStore>,
    stored: Option<String>,
    store_problem: Option<String>,
    variable_invalid: bool,
    key: Option<ArchiveKey>,
    dialog: Option<KeyDialog>,
    declined: bool,
    check: Option<Check>,
}

impl Model {
    /// Sets where the key is stored in the user's profile and reads what is stored there; a stored
    /// key that cannot be read is kept as a note for the dialog.
    pub fn set_key_store(&mut self, store: Option<KeyStore>) {
        let loaded = store.as_ref().map(KeyStore::load);
        self.key.store_problem = None;
        self.key.stored = match loaded {
            Some(Ok(key)) => key.map(|key| key.masked()),
            Some(Err(error)) => {
                self.key.store_problem = Some(describe(&error));
                None
            }
            None => None,
        };
        self.key.store = store;
    }

    /// Notes that the `P4KX_ARCHIVE_KEY` environment variable is set but holds no valid key, so
    /// that the dialog says it is ignored.
    pub const fn set_key_variable_invalid(&mut self, invalid: bool) {
        self.key.variable_invalid = invalid;
    }

    /// Returns whether the environment variable with the key was set but invalid.
    #[must_use]
    pub const fn key_variable_invalid(&self) -> bool {
        self.key.variable_invalid
    }

    /// Returns why the stored key could not be read, or `None` when it could or none is stored.
    #[must_use]
    pub fn key_store_problem(&self) -> Option<&str> {
        self.key.store_problem.as_deref()
    }

    /// Sets the key the Archives are opened with; `None` opens them without one.
    pub fn set_archive_key(&mut self, key: Option<ArchiveKey>) {
        if let Some(archive) = self
            .browser
            .as_ref()
            .and_then(|browser| browser.archive_index().archive())
        {
            match &key {
                Some(key) => archive.set_key(key.clone()),
                None => archive.clear_key(),
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
            pending: false,
        });
    }

    /// Closes the dialog without a change; a dialog the window opened by itself does not open
    /// again until another Archive is opened.
    pub fn close_key_dialog(&mut self) {
        self.key.check = None;
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
            .and_then(|browser| browser.archive_index().archive())
            .is_some_and(Archive::take_key_request);
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

    /// Starts checking the key typed against the open Archive on a background thread; the dialog
    /// is pending until [`Model::poll_key_check`] takes the answer. `wake` is called when the
    /// answer is in and from the threads that read again what the missing key left unread.
    /// Returns whether a check started or, without an open Archive, the key was taken at once.
    pub fn submit_key_dialog(&mut self, wake: impl Fn() + Send + Sync + 'static) -> bool {
        let Some(dialog) = &self.key.dialog else {
            return false;
        };
        if dialog.pending {
            return false;
        }
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
        let wake: Wake = Arc::new(wake);
        let Some(index) = self
            .browser
            .as_ref()
            .map(|browser| browser.shared_index().clone())
            .filter(|index| index.archive().is_some())
        else {
            return self.take_key(key, &wake);
        };
        let (sender, result) = mpsc::channel();
        let notify = Arc::clone(&wake);
        let checked = key;
        let spawned = thread::Builder::new()
            .name("key-check".to_owned())
            .spawn(move || {
                let problem = match index.entries_archive().map(|a| a.verify_key(&checked)) {
                    Ok(Ok(_)) => None,
                    Ok(Err(ArchiveError::WrongKey { .. })) => Some(KeyProblem::Wrong),
                    Ok(Err(error)) => Some(KeyProblem::Check(describe(&error))),
                    Err(error) => Some(KeyProblem::Check(describe(&error))),
                };
                if sender.send((checked, problem)).is_ok() {
                    notify();
                }
            });
        if let Err(error) = spawned {
            self.key_problem(KeyProblem::Check(describe(&error)));
            return false;
        }
        self.key.check = Some(Check { result, wake });
        if let Some(dialog) = &mut self.key.dialog {
            dialog.pending = true;
        }
        true
    }

    /// Returns whether a key typed is being checked.
    #[must_use]
    pub const fn key_check_running(&self) -> bool {
        self.key.check.is_some()
    }

    /// Takes the answer of a finished key check: a key that fits is stored, used from now on and
    /// closes the dialog; any other answer is shown in the dialog. Returns whether one finished.
    pub fn poll_key_check(&mut self) -> bool {
        let Some(check) = &self.key.check else {
            return false;
        };
        let answer = match check.result.try_recv() {
            Ok(answer) => Some(answer),
            Err(TryRecvError::Empty) => return false,
            Err(TryRecvError::Disconnected) => None,
        };
        let Some(check) = self.key.check.take() else {
            return false;
        };
        if let Some(dialog) = &mut self.key.dialog {
            dialog.pending = false;
        }
        match answer {
            Some((key, None)) => {
                self.take_key(key, &check.wake);
            }
            Some((_, Some(problem))) => self.key_problem(problem),
            None => self.key_problem(KeyProblem::Check("the check stopped".to_owned())),
        }
        true
    }

    fn take_key(&mut self, key: ArchiveKey, wake: &Wake) -> bool {
        if let Some(store) = &self.key.store {
            if let Err(error) = store.save(&key) {
                self.key_problem(KeyProblem::Store(describe(&error)));
                return false;
            }
            self.key.stored = Some(key.masked());
            self.key.store_problem = None;
        }
        self.key.dialog = None;
        self.key.declined = false;
        self.set_archive_key(Some(key));
        if let Some(browser) = &mut self.browser {
            let wake = Arc::clone(wake);
            browser.key_changed(move || wake());
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
        self.key.store_problem = None;
        self.set_archive_key(None);
        true
    }

    fn key_problem(&mut self, problem: KeyProblem) {
        if let Some(dialog) = &mut self.key.dialog {
            dialog.problem = Some(problem);
        }
    }

    pub(super) fn key_archive_opened(&mut self) {
        self.key.declined = false;
        self.key.check = None;
        if let Some(dialog) = &mut self.key.dialog {
            dialog.pending = false;
        }
    }
}
