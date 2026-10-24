// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::path::{Component, Path, PathBuf, Prefix};

use p4kx_channels::{Channel, SearchPaths, detect};
use p4kx_settings::Settings;

use crate::error::{ToolError, capped};

/// The longest `archive` argument accepted, in bytes.
pub(crate) const MAX_ARCHIVE_LEN: usize = 4096;
const ARCHIVE_EXTENSION: &str = "p4k";

/// Where Channels are looked for: the default Library folder, the launcher folder and the
/// settings file with the manual entries.
#[derive(Debug, Clone)]
pub(crate) struct Locations {
    default_library: Option<PathBuf>,
    launcher: Option<PathBuf>,
    settings_file: Option<PathBuf>,
}

impl Locations {
    /// Creates the locations; `None` falls back to the platform default.
    pub(crate) fn new(
        default_library: Option<PathBuf>,
        launcher: Option<PathBuf>,
        settings_file: Option<PathBuf>,
    ) -> Self {
        let standard = SearchPaths::standard();
        Self {
            default_library: default_library.or(standard.default_library),
            launcher: launcher.or(standard.launcher),
            settings_file,
        }
    }

    /// Returns the detected Channels; only directory listings, manifests, the launcher log and
    /// the settings file are read.
    pub(crate) fn channels(&self) -> Result<Vec<Channel>, ToolError> {
        let settings = match &self.settings_file {
            Some(file) => Settings::load(file)?,
            None => Settings::default(),
        };
        Ok(detect(&SearchPaths {
            default_library: self.default_library.clone(),
            launcher: self.launcher.clone(),
            manual_folders: settings.channels.folders,
            manual_archives: settings.channels.archives,
        }))
    }

    /// Returns the Archive path `text` names: a detected Channel by name, ignoring ASCII case,
    /// or the absolute local path of an existing `.p4k` file.
    pub(crate) fn resolve(&self, text: &str) -> Result<PathBuf, ToolError> {
        let text = capped("archive", text, MAX_ARCHIVE_LEN)?;
        if is_channel_name(text) {
            let channels = self.channels()?;
            return channels
                .iter()
                .find(|channel| channel.name().eq_ignore_ascii_case(text))
                .map(|channel| channel.archive().to_path_buf())
                .ok_or_else(|| ToolError::UnknownChannel {
                    name: text.to_owned(),
                    detected: channels
                        .iter()
                        .map(Channel::name)
                        .collect::<Vec<_>>()
                        .join(", "),
                });
        }
        archive_path(text)
    }
}

fn is_channel_name(text: &str) -> bool {
    !text.is_empty() && !text.contains(['\\', '/', ':', '\0'])
}

fn archive_path(text: &str) -> Result<PathBuf, ToolError> {
    let path = Path::new(text);
    let local = path.is_absolute()
        && !text.contains('\0')
        && path.components().all(|component| match component {
            Component::Prefix(prefix) => matches!(prefix.kind(), Prefix::Disk(_)),
            Component::RootDir => true,
            Component::Normal(name) => !name.as_encoded_bytes().contains(&b':'),
            Component::CurDir | Component::ParentDir => false,
        })
        && has_archive_extension(path);
    if !local {
        return Err(ToolError::NotAnArchivePath(text.to_owned()));
    }
    if !path.is_file() {
        return Err(ToolError::Missing(path.to_path_buf()));
    }
    if !resolves_locally(path) {
        return Err(ToolError::NotLocal(path.to_path_buf()));
    }
    Ok(path.to_path_buf())
}

fn has_archive_extension(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case(ARCHIVE_EXTENSION))
}

/// Returns whether `path`, with every link followed, is a `.p4k` file on a local drive.
fn resolves_locally(path: &Path) -> bool {
    let Ok(canonical) = std::fs::canonicalize(path) else {
        return false;
    };
    let on_local_drive = !cfg!(windows)
        || matches!(
            canonical.components().next(),
            Some(Component::Prefix(prefix)) if matches!(prefix.kind(), Prefix::VerbatimDisk(_))
        );
    on_local_drive && has_archive_extension(&canonical)
}

#[cfg(test)]
mod tests {
    use super::{ToolError, archive_path, is_channel_name};

    #[test]
    fn plain_names_are_channel_names() {
        for name in ["LIVE", "ptu", "TECH-PREVIEW", "4.3"] {
            assert!(is_channel_name(name), "{name} is a Channel name");
        }
        for text in ["", "a/b", r"a\b", "C:", "a\0"] {
            assert!(!is_channel_name(text), "{text:?} is no Channel name");
        }
    }

    #[test]
    fn only_absolute_local_p4k_paths_are_accepted() {
        let rejected = [
            "Data.p4k",
            "./Data.p4k",
            "LIVE/Data.p4k",
            r"\\server\share\Data.p4k",
            r"\\?\C:\Data.p4k",
            r"\\.\C:\Data.p4k",
            "/games/../Data.p4k",
            r"C:\games\..\Data.p4k",
            "/games/Data.zip",
            r"C:\games\Data.zip",
            "/games/Data",
            r"C:\games:stream\Data.p4k",
            r"C:\games\Data.p4k:stream",
            "/games:stream/Data.p4k",
        ];
        for text in rejected {
            assert!(
                matches!(archive_path(text), Err(ToolError::NotAnArchivePath(_))),
                "{text} is rejected"
            );
        }
    }

    #[test]
    fn a_missing_p4k_file_is_named() {
        let text = if cfg!(windows) {
            r"C:\p4kx-mcp-missing\Data.P4K"
        } else {
            "/p4kx-mcp-missing/Data.P4K"
        };
        assert!(
            matches!(archive_path(text), Err(ToolError::Missing(_))),
            "{text} is missing"
        );
    }
}
