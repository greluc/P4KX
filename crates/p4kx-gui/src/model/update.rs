// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::mpsc::{self, Receiver};
use std::thread;

use p4kx_update::{Release, ReleaseCheck};

use super::Model;

const CURRENT: &str = env!("CARGO_PKG_VERSION");

/// The switchable check for a newer Release and what it found.
#[derive(Debug)]
pub(crate) struct UpdateState {
    enabled: bool,
    check: ReleaseCheck,
    running: Option<Receiver<Option<Release>>>,
    newer: Option<Release>,
}

impl Default for UpdateState {
    fn default() -> Self {
        Self {
            enabled: false,
            check: ReleaseCheck::github(),
            running: None,
            newer: None,
        }
    }
}

impl Model {
    /// Returns whether the window looks for a newer Release.
    #[must_use]
    pub const fn update_check(&self) -> bool {
        self.update.enabled
    }

    /// Switches the check for a newer Release on or off; switching it off forgets the hint.
    pub fn set_update_check(&mut self, enabled: bool) {
        self.update.enabled = enabled;
        if !enabled {
            self.update.running = None;
            self.update.newer = None;
        }
    }

    /// Switches the check as the user chose it, asking at once when it is now on; `wake` is
    /// called once the answer is in.
    pub fn switch_update_check(&mut self, enabled: bool, wake: impl Fn() + Send + 'static) {
        self.set_update_check(enabled);
        self.check_for_update(wake);
    }

    /// Replaces the endpoint asked for the latest Release.
    pub fn set_release_check(&mut self, check: ReleaseCheck) {
        self.update.check = check;
    }

    /// Asks for a newer Release on a background thread when the check is on and has not run
    /// yet; `wake` is called from that thread once the answer is in.
    pub fn check_for_update(&mut self, wake: impl Fn() + Send + 'static) {
        let update = &mut self.update;
        if !update.enabled || update.running.is_some() || update.newer.is_some() {
            return;
        }
        let (sender, receiver) = mpsc::channel();
        let check = update.check.clone();
        let spawned = thread::Builder::new()
            .name("update-check".to_owned())
            .spawn(move || {
                let newer = check.newer_than(CURRENT).ok().flatten();
                if sender.send(newer).is_ok() {
                    wake();
                }
            });
        if spawned.is_ok() {
            update.running = Some(receiver);
        }
    }

    /// Takes the answer of a finished check; returns whether one finished.
    pub fn poll_update(&mut self) -> bool {
        let Some(running) = &self.update.running else {
            return false;
        };
        match running.try_recv() {
            Ok(newer) => {
                self.update.newer = newer;
                self.update.running = None;
                true
            }
            Err(mpsc::TryRecvError::Empty) => false,
            Err(mpsc::TryRecvError::Disconnected) => {
                self.update.running = None;
                true
            }
        }
    }

    /// Returns the newer Release the check found, or `None`.
    #[must_use]
    pub const fn newer_release(&self) -> Option<&Release> {
        self.update.newer.as_ref()
    }
}
