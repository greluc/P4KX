use std::error::Error;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

use p4kx_extraction::{Archive, ArchiveError};

use crate::{Browser, EntryIndex, Language, Texts};

/// Why an Archive could not be opened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    /// The Archive that was opened.
    pub path: PathBuf,
    /// The error and its causes, joined by colons, or `None` when the opening stopped without
    /// a result.
    pub reason: Option<String>,
}

struct Opening {
    path: PathBuf,
    result: Receiver<Result<EntryIndex, ArchiveError>>,
}

/// The whole state of the window, independent of how it is drawn.
pub struct Model {
    texts: Texts,
    browser: Option<Browser>,
    opening: Option<Opening>,
    failure: Option<Failure>,
    licence_shown: bool,
}

impl std::fmt::Debug for Model {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Model")
            .field("texts", &self.texts)
            .field("browser", &self.browser)
            .field("opening", &self.opening_path())
            .field("failure", &self.failure)
            .field("licence_shown", &self.licence_shown)
            .finish()
    }
}

impl Model {
    /// Creates the state of a window without an Archive, in `language`.
    #[must_use]
    pub fn new(language: Language) -> Self {
        Self {
            texts: Texts::new(language),
            browser: None,
            opening: None,
            failure: None,
            licence_shown: false,
        }
    }

    /// Returns the interface texts in the current language.
    #[must_use]
    pub const fn texts(&self) -> &Texts {
        &self.texts
    }

    /// Switches the interface language.
    pub const fn set_language(&mut self, language: Language) {
        self.texts.set_language(language);
    }

    /// Starts opening the Archive at `path` on a background thread, replacing an opening that
    /// is still running; `wake` is called from that thread once the result is ready.
    pub fn open(&mut self, path: PathBuf, wake: impl Fn() + Send + 'static) {
        let (sender, result) = mpsc::channel();
        let thread_path = path.clone();
        let spawned = thread::Builder::new()
            .name("open-archive".to_owned())
            .spawn(move || {
                let index = Archive::open(&thread_path).map(EntryIndex::build);
                if sender.send(index).is_ok() {
                    wake();
                }
            });
        self.opening = None;
        match spawned {
            Ok(_handle) => {
                self.failure = None;
                self.opening = Some(Opening { path, result });
            }
            Err(error) => self.fail(path, &error),
        }
    }

    /// Takes the result of a finished opening; returns whether one finished.
    pub fn poll(&mut self) -> bool {
        let Some(opening) = &self.opening else {
            return false;
        };
        let received = match opening.result.try_recv() {
            Ok(received) => received,
            Err(TryRecvError::Empty) => return false,
            Err(TryRecvError::Disconnected) => {
                let path = opening.path.clone();
                self.opening = None;
                self.failure = Some(Failure { path, reason: None });
                return true;
            }
        };
        let Some(opening) = self.opening.take() else {
            return false;
        };
        match received {
            Ok(index) => self.browser = Some(Browser::new(index)),
            Err(error) => self.fail(opening.path, &error),
        }
        true
    }

    /// Returns the Archive being opened, or `None` when nothing is opening.
    #[must_use]
    pub fn opening_path(&self) -> Option<&Path> {
        self.opening.as_ref().map(|opening| opening.path.as_path())
    }

    /// Returns why the last opening failed, or `None` when it did not.
    #[must_use]
    pub const fn failure(&self) -> Option<&Failure> {
        self.failure.as_ref()
    }

    /// Returns the browsing state of the open Archive, or `None` when none is open.
    #[must_use]
    pub const fn browser(&self) -> Option<&Browser> {
        self.browser.as_ref()
    }

    /// Returns the browsing state of the open Archive for changes.
    pub const fn browser_mut(&mut self) -> Option<&mut Browser> {
        self.browser.as_mut()
    }

    /// Returns whether the font licence is shown.
    #[must_use]
    pub const fn licence_shown(&self) -> bool {
        self.licence_shown
    }

    /// Shows or hides the font licence.
    pub const fn set_licence_shown(&mut self, shown: bool) {
        self.licence_shown = shown;
    }

    fn fail(&mut self, path: PathBuf, error: &dyn Error) {
        let mut reason = error.to_string();
        let mut source = error.source();
        while let Some(cause) = source {
            reason.push_str(": ");
            reason.push_str(&cause.to_string());
            source = cause.source();
        }
        self.failure = Some(Failure {
            path,
            reason: Some(reason),
        });
    }
}
