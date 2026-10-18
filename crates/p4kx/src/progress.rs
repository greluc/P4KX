use std::io::{self, IsTerminal, Write};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use p4kx_export::CatalogProgress;
use p4kx_extraction::Progress;

const INTERVAL: Duration = Duration::from_millis(100);
const WIDTH: usize = 60;

/// A single status line on stderr that shows the progress of an Extraction or a catalog build
/// while stderr is a terminal.
pub(crate) struct ProgressLine {
    enabled: bool,
    last: Mutex<Option<Instant>>,
}

impl ProgressLine {
    /// Creates the line, enabled only when stderr is a terminal.
    pub(crate) fn new() -> Self {
        Self {
            enabled: io::stderr().is_terminal(),
            last: Mutex::new(None),
        }
    }

    /// Redraws the line with `progress`, at most every 100 ms.
    pub(crate) fn show(&self, progress: Progress) {
        self.draw(|| {
            let percent = progress
                .bytes_done
                .saturating_mul(100)
                .checked_div(progress.bytes_total)
                .unwrap_or(100);
            format!(
                "extracting {}/{} Entries, {percent} %",
                progress.entries_done, progress.entries_total
            )
        });
    }

    /// Redraws the line with the Records checked for a catalog, at most every 100 ms.
    pub(crate) fn show_records(&self, progress: CatalogProgress) {
        self.draw(|| {
            format!(
                "checking {}/{} Records",
                progress.records_done, progress.records_total
            )
        });
    }

    fn draw(&self, text: impl FnOnce() -> String) {
        if !self.enabled {
            return;
        }
        let Ok(mut last) = self.last.try_lock() else {
            return;
        };
        let now = Instant::now();
        if last.is_some_and(|at| now.duration_since(at) < INTERVAL) {
            return;
        }
        *last = Some(now);
        let text = text();
        let _ignored = write!(io::stderr().lock(), "\r{text:<WIDTH$}");
    }

    /// Removes the line.
    pub(crate) fn finish(&self) {
        if self.enabled {
            let _ignored = write!(io::stderr().lock(), "\r{:WIDTH$}\r", "");
        }
    }
}
