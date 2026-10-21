use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

use p4kx_extraction::game_is_running;

use super::Model;

/// How long a look at the process list stays valid.
pub const GAME_CHECK_INTERVAL: Duration = Duration::from_secs(20);

/// Whether `StarCitizen.exe` runs, looked up on a background thread now and then.
#[derive(Debug, Default)]
pub(super) struct GameWatch {
    enabled: bool,
    running: bool,
    next: Option<Instant>,
    pending: Option<Receiver<bool>>,
}

impl GameWatch {
    fn poll(&mut self, wake: impl Fn() + Send + 'static) {
        if !self.enabled {
            return;
        }
        if let Some(pending) = &self.pending {
            match pending.try_recv() {
                Ok(running) => self.running = running,
                Err(TryRecvError::Empty) => return,
                Err(TryRecvError::Disconnected) => {}
            }
            self.pending = None;
            self.next = Some(Instant::now() + GAME_CHECK_INTERVAL);
        }
        if self.next.is_some_and(|due| Instant::now() < due) {
            return;
        }
        let (sender, result) = mpsc::channel();
        let spawned = thread::Builder::new()
            .name("game-check".to_owned())
            .spawn(move || {
                if sender.send(game_is_running()).is_ok() {
                    wake();
                }
            });
        if spawned.is_ok() {
            self.pending = Some(result);
        } else {
            self.next = Some(Instant::now() + GAME_CHECK_INTERVAL);
        }
    }
}

impl Model {
    /// Switches the look at the process list for a running game on or off; it starts off.
    pub const fn watch_game(&mut self, enabled: bool) {
        self.game.enabled = enabled;
    }

    /// Looks at the process list when [`GAME_CHECK_INTERVAL`] passed since the last look, on a
    /// background thread that calls `wake` once it has the answer.
    pub fn check_game_when_due(&mut self, wake: impl Fn() + Send + 'static) {
        self.game.poll(wake);
    }

    /// Returns whether the last look at the process list found `StarCitizen.exe` running.
    #[must_use]
    pub const fn game_running(&self) -> bool {
        self.game.enabled && self.game.running
    }

    /// Makes the next [`check_game_when_due`](Self::check_game_when_due) look again, as when an
    /// Archive is opened.
    pub(super) const fn recheck_game(&mut self) {
        self.game.next = None;
    }
}
