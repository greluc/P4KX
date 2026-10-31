// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::path::Path;

use p4kx_extraction::ArchiveWatch;

use super::Model;
use crate::ExtractionScope;

impl Model {
    /// Returns whether the open Archive was seen to have changed on disk since it was opened.
    #[must_use]
    pub fn archive_changed(&self) -> bool {
        self.browser
            .as_ref()
            .is_some_and(|browser| browser.archive_index().watch().is_changed())
    }

    /// Returns whether a change of the open Archive on disk stopped a running Extraction of it.
    #[must_use]
    pub const fn change_stopped_extraction(&self) -> bool {
        self.stopped_by_change
    }

    /// Compares the open Archive with the disk now; returns whether it changed.
    ///
    /// Once it changed, an Extraction setup and a running Extraction of the open Archive close
    /// and are cancelled; those of a Comparison side check their own Archive.
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
        let path = browser.archive_index().path().to_path_buf();
        self.open(path, wake);
    }

    fn react_to<E>(&mut self, compare: impl Fn(&ArchiveWatch) -> Result<(), E>) -> bool {
        let Some(browser) = &mut self.browser else {
            return false;
        };
        let watch = browser.archive_index().watch();
        if compare(watch).is_ok() {
            return false;
        }
        let path = browser.archive_index().path().to_path_buf();
        browser.archive_index().forge().clear();
        browser.archive_index().texts().clear();
        browser.drop_records();
        browser.drop_containers();
        self.record_export.archive_changed();
        self.glb_export.archive_changed();
        self.assembly.archive_changed();
        self.characters.reset(true);
        self.sounds.archive_changed();
        self.tags.archive_changed();
        self.ui.archive_changed();
        if self
            .extraction_setup
            .as_ref()
            .is_some_and(|setup| !matches!(setup.scope(), ExtractionScope::Changes(_)))
        {
            self.extraction_setup = None;
        }
        if let Some(extraction) = &self.extraction
            && extraction.is_running()
            && !extraction.is_cancelled()
            && same_file(extraction.archive(), &path)
        {
            extraction.cancel();
            self.stopped_by_change = true;
        }
        true
    }
}

/// Returns whether `a` and `b` name the same file, compared as canonical paths, or as absolute
/// paths when a file cannot be resolved.
fn same_file(a: &Path, b: &Path) -> bool {
    let normal = |path: &Path| {
        path.canonicalize()
            .or_else(|_| std::path::absolute(path))
            .unwrap_or_else(|_| path.to_path_buf())
    };
    a == b || normal(a) == normal(b)
}
