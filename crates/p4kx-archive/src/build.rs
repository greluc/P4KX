use std::io;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use thiserror::Error;

use crate::public_patch;

const MANIFEST: &str = "build_manifest.id";

/// The Build of an Archive as stated in its Channel's `build_manifest.id`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Build {
    branch: String,
    version: String,
    change_number: String,
    id: String,
    public_patch: Option<String>,
}

/// A `build_manifest.id` that exists but cannot be read.
#[derive(Debug, Error)]
pub enum BuildError {
    /// The manifest could not be read.
    #[error("cannot read {path}")]
    Io {
        /// Path of the manifest.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: io::Error,
    },
    /// The manifest is not the JSON the launcher writes.
    #[error("{path} is not a valid build manifest")]
    Malformed {
        /// Path of the manifest.
        path: PathBuf,
        /// What the JSON parser reported.
        #[source]
        source: serde_json::Error,
    },
}

#[derive(Deserialize)]
struct Manifest {
    #[serde(rename = "Data")]
    data: ManifestData,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ManifestData {
    branch: String,
    version: String,
    #[serde(rename = "RequestedP4ChangeNum")]
    change_number: String,
    build_id: String,
}

impl Build {
    /// Reads the Build from the `build_manifest.id` in `channel_folder`, or returns `None` when
    /// the folder has no manifest.
    ///
    /// # Errors
    ///
    /// Returns a [`BuildError`] when the manifest exists but cannot be read or parsed.
    pub fn read_from(channel_folder: impl AsRef<Path>) -> Result<Option<Self>, BuildError> {
        let path = channel_folder.as_ref().join(MANIFEST);
        let text = match std::fs::read(&path) {
            Ok(text) => text,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(source) => return Err(BuildError::Io { path, source }),
        };
        let manifest: Manifest = serde_json::from_slice(&text)
            .map_err(|source| BuildError::Malformed { path, source })?;
        let data = manifest.data;
        Ok(Some(Self {
            branch: data.branch,
            version: data.version,
            change_number: data.change_number,
            id: data.build_id,
            public_patch: None,
        }))
    }

    /// Returns the Build with the public patch the RSI Launcher logged for its change number
    /// in the log files of `log_folder`, or unchanged when no log line names that change number.
    ///
    /// Only `log.log` and `log.old.log` are read; a missing or unreadable file is skipped.
    #[must_use]
    pub fn with_public_patch_from(mut self, log_folder: &Path) -> Self {
        self.public_patch = public_patch::find(log_folder, &self.change_number);
        self
    }

    /// Returns the Build with the public patch from the RSI Launcher's own log folder, see
    /// [`Build::with_public_patch_from`]; `P4KX_LAUNCHER_LOGS` names another log folder.
    #[must_use]
    pub fn with_public_patch(self) -> Self {
        match public_patch::standard_log_folder() {
            Some(folder) => self.with_public_patch_from(&folder),
            None => self,
        }
    }

    /// Returns the public patch version, e.g. `4.7.2`, or `None` when the launcher log has no
    /// line for this Build's change number.
    #[must_use]
    pub fn public_patch(&self) -> Option<&str> {
        self.public_patch.as_deref()
    }

    /// Returns the branch, e.g. `sc-alpha-4.10.2`.
    #[must_use]
    pub fn branch(&self) -> &str {
        &self.branch
    }

    /// Returns the version, e.g. `4.10.197.2878`.
    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Returns the Perforce change number the Build was made from.
    #[must_use]
    pub fn change_number(&self) -> &str {
        &self.change_number
    }

    /// Returns the `BuildId` GUID.
    #[must_use]
    pub fn build_id(&self) -> &str {
        &self.id
    }
}
