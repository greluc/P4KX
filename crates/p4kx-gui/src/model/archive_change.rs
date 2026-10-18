use p4kx_extraction::ArchiveWatch;

use super::Model;

impl Model {
    /// Returns whether the open Archive was seen to have changed on disk since it was opened.
    #[must_use]
    pub fn archive_changed(&self) -> bool {
        self.browser
            .as_ref()
            .is_some_and(|browser| browser.index().watch().is_changed())
    }

    /// Compares the open Archive with the disk now; returns whether it changed.
    ///
    /// Once it changed, the Extraction setup closes and a running Extraction is cancelled.
    pub fn check_archive(&mut self) -> bool {
        self.react_to(ArchiveWatch::check)
    }

    /// Compares the open Archive with the disk when the watch interval passed since the last
    /// comparison, like [`check_archive`](Self::check_archive); returns whether it changed.
    pub fn check_archive_when_due(&mut self) -> bool {
        self.react_to(ArchiveWatch::poll)
    }

    /// Opens the open Archive again, as it is on disk now, calling `wake` from the opening
    /// thread once it is open.
    pub fn reload(&mut self, wake: impl Fn() + Send + 'static) {
        let Some(browser) = &self.browser else {
            return;
        };
        let path = browser.index().archive().path().to_path_buf();
        self.open(path, wake);
    }

    fn react_to<E>(&mut self, compare: impl Fn(&ArchiveWatch) -> Result<(), E>) -> bool {
        let changed = self
            .browser
            .as_ref()
            .is_some_and(|browser| compare(browser.index().watch()).is_err());
        if changed {
            if let Some(browser) = &mut self.browser {
                browser.drop_records();
            }
            self.extraction_setup = None;
            self.cancel_extraction();
        }
        changed
    }
}
