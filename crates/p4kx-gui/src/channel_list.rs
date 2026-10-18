use std::path::PathBuf;

use p4kx_channels::{Channel, SearchPaths, detect};
use p4kx_settings::ManualChannels;

/// The Channels the picker offers: detected in the standard places and the manual entries.
#[derive(Debug, Clone, Default)]
pub struct ChannelList {
    base: SearchPaths,
    manual: ManualChannels,
    detected: Vec<Channel>,
}

impl ChannelList {
    /// Detects the Channels in `base` and in `manual`, which replaces the manual entries of
    /// `base`.
    #[must_use]
    pub fn new(base: SearchPaths, manual: ManualChannels) -> Self {
        let mut list = Self {
            base: SearchPaths {
                manual_folders: Vec::new(),
                manual_archives: Vec::new(),
                ..base
            },
            manual,
            detected: Vec::new(),
        };
        list.detect();
        list
    }

    /// Returns the detected Channels in the order of [`detect`].
    #[must_use]
    pub fn detected(&self) -> &[Channel] {
        &self.detected
    }

    /// Returns the manual entries.
    #[must_use]
    pub const fn manual(&self) -> &ManualChannels {
        &self.manual
    }

    /// Adds `path` as a manual Archive when it is a file, otherwise as a manual folder, and
    /// detects again; returns whether it was not listed yet.
    pub fn add(&mut self, path: PathBuf) -> bool {
        let path = std::path::absolute(&path).unwrap_or(path);
        let added = if path.is_file() {
            self.manual.add_archive(path)
        } else {
            self.manual.add_folder(path)
        };
        if added {
            self.detect();
        }
        added
    }

    fn detect(&mut self) {
        self.detected = detect(&SearchPaths {
            manual_folders: self.manual.folders.clone(),
            manual_archives: self.manual.archives.clone(),
            ..self.base.clone()
        });
    }
}
