// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! A folder of files on disk as the source of a Conversion.

use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};

use p4kx_dataforge::SNIFF_LEN;
use rayon::prelude::*;
use thiserror::Error;

use crate::source::{EntrySource, Handle, SourceEntry, SourceReader};
use crate::{CancelToken, EntryFailure, ExtractionError, Filter, is_dataforge_file};

/// The most files [`FolderSource::scan`] lists.
pub const MAX_FOLDER_FILES: usize = 1 << 22;
/// The deepest nesting of folders [`FolderSource::scan`] descends into.
pub const MAX_FOLDER_DEPTH: usize = 128;

/// Why a file of an input folder is not read as an Entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum FolderFileError {
    /// A symbolic link or junction; links are never followed.
    #[error("a link is not followed out of the input folder")]
    Link,
    /// A file name that is not valid UTF-8.
    #[error("the name is not valid UTF-8")]
    NotUtf8,
    /// A file name that holds a path separator.
    #[error("the name contains a path separator")]
    Separator,
    /// Neither a file, a folder nor a link.
    #[error("it is neither a file nor a folder")]
    Special,
    /// The file or folder cannot be read.
    #[error("cannot be read: {0}")]
    Unreadable(io::ErrorKind),
    /// The size of the file differs from the one when the folder was read.
    #[error("the file changed after the folder was read")]
    Changed,
}

/// Why an input folder cannot be converted.
#[derive(Debug, Error)]
pub enum FolderError {
    /// The input folder cannot be read.
    #[error("cannot read {}", path.display())]
    Read {
        /// The folder.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: io::Error,
    },
    /// The input path is no folder.
    #[error("{} is not a folder", .0.display())]
    NotAFolder(PathBuf),
    /// The folder holds more than [`MAX_FOLDER_FILES`] files.
    #[error("the folder holds more than {limit} files")]
    TooManyFiles {
        /// The limit.
        limit: usize,
    },
    /// The folder nests deeper than [`MAX_FOLDER_DEPTH`] levels.
    #[error("the folder nests deeper than {limit} levels")]
    TooDeep {
        /// The limit.
        limit: usize,
    },
    /// One of the input and output folders lies inside the other, or they are the same.
    #[error("the output folder {} and the input folder {} overlap", output.display(), input.display())]
    Overlap {
        /// The input folder.
        input: PathBuf,
        /// The output folder.
        output: PathBuf,
    },
    /// The output folder lies inside a Star Citizen installation.
    #[error(transparent)]
    Installation(#[from] p4kx_guard::WriteRefused),
    /// The run was cancelled while the folder was read.
    #[error("cancelled")]
    Cancelled,
}

#[derive(Debug)]
struct FolderFile {
    path: String,
    size: u64,
    disk: PathBuf,
}

/// The files below an input folder, listed once, with the paths below it using `\`.
///
/// Links are never followed and so never listed; each is reported as a rejected file.
#[derive(Debug)]
pub struct FolderSource {
    root: PathBuf,
    files: Vec<FolderFile>,
    rejected: Vec<(String, FolderFileError)>,
}

impl FolderSource {
    /// Lists the files below the folder `root`, sorted by path, stopping once `cancel` is
    /// cancelled.
    ///
    /// # Errors
    ///
    /// Returns [`FolderError::NotAFolder`] when `root` is no folder, [`FolderError::Read`] when
    /// it cannot be read, [`FolderError::TooManyFiles`] and [`FolderError::TooDeep`] beyond the
    /// limits, and [`FolderError::Cancelled`].
    pub fn scan(root: &Path, cancel: Option<&CancelToken>) -> Result<Self, FolderError> {
        let read = |source| FolderError::Read {
            path: root.to_path_buf(),
            source,
        };
        if !fs::metadata(root).map_err(read)?.is_dir() {
            return Err(FolderError::NotAFolder(root.to_path_buf()));
        }
        let root = fs::canonicalize(root).map_err(read)?;
        let mut source = Self {
            root,
            files: Vec::new(),
            rejected: Vec::new(),
        };
        let mut pending = vec![(source.root.clone(), String::new(), 0)];
        while let Some((folder, prefix, depth)) = pending.pop() {
            if cancel.is_some_and(CancelToken::is_cancelled) {
                return Err(FolderError::Cancelled);
            }
            source.list(&folder, &prefix, depth, &mut pending)?;
        }
        source.files.sort_unstable_by(|a, b| a.path.cmp(&b.path));
        Ok(source)
    }

    /// Returns the folder, canonical.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Returns the number of files listed.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.files.len()
    }

    /// Returns whether no file is listed.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// Returns the files and folders that were not listed, with the path below the folder and
    /// the reason.
    #[must_use]
    pub fn rejected(&self) -> &[(String, FolderFileError)] {
        &self.rejected
    }

    /// Returns the files whose path matches `filter`, or every file, in path order.
    pub(crate) fn select(&self, filter: Option<&Filter>) -> Vec<SourceEntry<'_>> {
        self.files
            .iter()
            .enumerate()
            .filter(|(_, file)| filter.is_none_or(|filter| filter.matches(&file.path)))
            .map(|(index, file)| entry(index, file))
            .collect()
    }

    /// Returns the failures of the files and folders that were not listed.
    pub(crate) fn rejected_failures(&self) -> impl Iterator<Item = EntryFailure> {
        self.rejected.iter().map(|(path, reason)| EntryFailure {
            path: path.clone(),
            error: ExtractionError::Folder(*reason),
        })
    }

    fn list(
        &mut self,
        folder: &Path,
        prefix: &str,
        depth: usize,
        pending: &mut Vec<(PathBuf, String, usize)>,
    ) -> Result<(), FolderError> {
        let entries = match fs::read_dir(folder) {
            Ok(entries) => entries,
            Err(error) if depth > 0 => {
                self.reject(
                    prefix.trim_end_matches('\\'),
                    FolderFileError::Unreadable(error.kind()),
                );
                return Ok(());
            }
            Err(source) => {
                return Err(FolderError::Read {
                    path: folder.to_path_buf(),
                    source,
                });
            }
        };
        for item in entries {
            let item = match item {
                Ok(item) => item,
                Err(error) => {
                    self.reject(
                        prefix.trim_end_matches('\\'),
                        FolderFileError::Unreadable(error.kind()),
                    );
                    continue;
                }
            };
            let Ok(name) = item.file_name().into_string() else {
                let lossy = item.file_name().to_string_lossy().into_owned();
                self.reject(&format!("{prefix}{lossy}"), FolderFileError::NotUtf8);
                continue;
            };
            let path = format!("{prefix}{name}");
            if name.contains(['\\', '/']) {
                self.reject(&path, FolderFileError::Separator);
                continue;
            }
            let kind = match item.file_type() {
                Ok(kind) => kind,
                Err(error) => {
                    self.reject(&path, FolderFileError::Unreadable(error.kind()));
                    continue;
                }
            };
            if kind.is_symlink() {
                self.reject(&path, FolderFileError::Link);
            } else if kind.is_dir() {
                if depth + 1 > MAX_FOLDER_DEPTH {
                    return Err(FolderError::TooDeep {
                        limit: MAX_FOLDER_DEPTH,
                    });
                }
                pending.push((item.path(), format!("{path}\\"), depth + 1));
            } else if kind.is_file() {
                self.add(&item, path)?;
            } else {
                self.reject(&path, FolderFileError::Special);
            }
        }
        Ok(())
    }

    fn add(&mut self, item: &fs::DirEntry, path: String) -> Result<(), FolderError> {
        if self.files.len() >= MAX_FOLDER_FILES {
            return Err(FolderError::TooManyFiles {
                limit: MAX_FOLDER_FILES,
            });
        }
        match item.metadata() {
            Ok(metadata) => self.files.push(FolderFile {
                path,
                size: metadata.len(),
                disk: item.path(),
            }),
            Err(error) => self.reject(&path, FolderFileError::Unreadable(error.kind())),
        }
        Ok(())
    }

    fn reject(&mut self, path: &str, reason: FolderFileError) {
        self.rejected.push((path.to_owned(), reason));
    }
}

fn entry(index: usize, file: &FolderFile) -> SourceEntry<'_> {
    SourceEntry::file(&file.path, file.size, index, &file.disk)
}

impl EntrySource for FolderSource {
    fn open(
        &self,
        entry: &SourceEntry<'_>,
        _verify: bool,
    ) -> Result<SourceReader<'_>, ExtractionError> {
        let Handle::File(path) = entry.handle() else {
            return Err(ExtractionError::Read(io::Error::new(
                io::ErrorKind::InvalidInput,
                "the file is not a file of this folder",
            )));
        };
        let unreadable =
            |error: io::Error| ExtractionError::Folder(FolderFileError::Unreadable(error.kind()));
        if fs::symlink_metadata(path).map_err(unreadable)?.is_symlink() {
            return Err(FolderFileError::Link.into());
        }
        let file = File::open(path).map_err(unreadable)?;
        let metadata = file.metadata().map_err(unreadable)?;
        if !metadata.is_file() || metadata.len() != entry.uncompressed_size() {
            return Err(FolderFileError::Changed.into());
        }
        Ok(Box::new(file))
    }

    fn raw_path(&self, _index: usize) -> Option<&[u8]> {
        None
    }

    fn path_of(&self, index: usize) -> Option<&str> {
        self.files.get(index).map(|file| file.path.as_str())
    }

    fn members(&self, wanted: &(dyn Fn(&str) -> bool + Sync)) -> Vec<SourceEntry<'_>> {
        self.files
            .par_iter()
            .enumerate()
            .filter(|(_, file)| wanted(&file.path))
            .map(|(index, file)| entry(index, file))
            .collect()
    }

    fn dataforge_position(&self, entries: &[SourceEntry<'_>]) -> Option<usize> {
        entries
            .par_iter()
            .enumerate()
            .filter(|(_, entry)| entry.uncompressed_size() >= SNIFF_LEN as u64)
            .filter(|(_, entry)| match entry.handle() {
                Handle::File(path) => is_dataforge_file(path).unwrap_or(false),
                Handle::Archive(_) => false,
            })
            .map(|(position, _)| position)
            .min()
    }
}
