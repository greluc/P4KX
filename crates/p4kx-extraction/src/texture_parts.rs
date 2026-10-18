use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use p4kx_archive::{Archive, Entry, EntryReader};
use p4kx_texture::SiblingReader;
use rayon::prelude::*;

use crate::entry_path::same_entry_path;

/// Where the header and the parts of a texture are stored.
#[derive(Debug)]
pub(crate) enum Parts<'a> {
    /// Entries of an Archive, by part number.
    Archive {
        archive: &'a Archive,
        header: &'a Entry,
        parts: BTreeMap<u32, &'a Entry>,
    },
    /// Files next to a header file on disk.
    Files { header: PathBuf },
}

/// The data of one part.
pub(crate) enum PartReader<'a> {
    Entry(EntryReader<'a>),
    File(File),
}

impl Read for PartReader<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        match self {
            Self::Entry(reader) => reader.read(buffer),
            Self::File(file) => file.read(buffer),
        }
    }
}

impl<'a> Parts<'a> {
    /// Finds the Entry at `path` and its parts in one parallel pass over the Archive, or `None`
    /// when no Entry has that path.
    pub(crate) fn in_archive(archive: &'a Archive, path: &str) -> Option<Self> {
        let found: Vec<(Role, &Entry)> = archive
            .entries()
            .par_iter()
            .filter_map(|entry| classify(entry.path(), path).map(|part| (part, entry)))
            .collect();
        let header = found
            .iter()
            .find_map(|(role, entry)| (*role == Role::Header).then_some(*entry))?;
        let mut parts = BTreeMap::new();
        for (role, entry) in found {
            if let Role::Part(index) = role {
                parts.entry(index).or_insert(entry);
            }
        }
        Some(Self::Archive {
            archive,
            header,
            parts,
        })
    }

    /// Returns the path of part `index` on disk, or `None` for parts in an Archive.
    pub(crate) fn file_of(&self, index: u32) -> Option<PathBuf> {
        match self {
            Self::Archive { .. } => None,
            Self::Files { header } => {
                let mut name = OsString::from(header.as_os_str());
                name.push(format!(".{index}"));
                Some(PathBuf::from(name))
            }
        }
    }
}

impl SiblingReader for Parts<'_> {
    type Part<'r>
        = PartReader<'r>
    where
        Self: 'r;

    fn part_len(&self, index: u32) -> io::Result<Option<u64>> {
        match self {
            Self::Archive { parts, .. } => {
                Ok(parts.get(&index).map(|entry| entry.uncompressed_size()))
            }
            Self::Files { .. } => match self.file_of(index).map(fs::metadata) {
                Some(Ok(metadata)) if metadata.is_file() => Ok(Some(metadata.len())),
                Some(Ok(_)) | None => Ok(None),
                Some(Err(error)) if error.kind() == io::ErrorKind::NotFound => Ok(None),
                Some(Err(error)) => Err(error),
            },
        }
    }

    fn open_part(&self, index: u32) -> io::Result<Self::Part<'_>> {
        match self {
            Self::Archive { archive, parts, .. } => {
                let entry = parts
                    .get(&index)
                    .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))?;
                archive
                    .open_entry(entry)
                    .map(PartReader::Entry)
                    .map_err(io::Error::other)
            }
            Self::Files { .. } => {
                let path = self
                    .file_of(index)
                    .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))?;
                File::open(path).map(PartReader::File)
            }
        }
    }
}

/// What an Entry is to the texture being looked up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    Header,
    Part(u32),
}

/// Returns the role of `candidate` for the header path `header`, or `None` when it is neither the
/// header nor one of its parts; separators and ASCII case do not matter.
fn classify(candidate: &str, header: &str) -> Option<Role> {
    let (start, rest) = candidate.split_at_checked(header.len())?;
    if !same_entry_path(start, header) {
        return None;
    }
    if rest.is_empty() {
        return Some(Role::Header);
    }
    let digits = rest.strip_prefix('.')?;
    if digits.starts_with('0') || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok().map(Role::Part)
}

/// Returns whether `path` names an existing file that is one of `inputs`.
pub(crate) fn is_same_file(path: &Path, inputs: impl IntoIterator<Item = PathBuf>) -> bool {
    let Ok(target) = fs::canonicalize(path) else {
        return false;
    };
    inputs
        .into_iter()
        .filter_map(|input| fs::canonicalize(input).ok())
        .any(|input| input == target)
}

#[cfg(test)]
mod tests {
    use super::{Role, classify};

    #[test]
    fn parts_are_numbered_siblings_of_the_header() {
        let header = "Data\\a.dds";
        assert_eq!(
            classify("data/A.DDS", header),
            Some(Role::Header),
            "the header"
        );
        assert_eq!(
            classify("Data\\a.dds.1", header),
            Some(Role::Part(1)),
            "part 1"
        );
        assert_eq!(
            classify("Data\\a.dds.12", header),
            Some(Role::Part(12)),
            "part 12"
        );
        for other in [
            "Data\\a.dds.1a",
            "Data\\a.dds.a",
            "Data\\a.dds.0",
            "Data\\a.dds.01",
            "Data\\a.dds.",
            "Data\\a.ddsx",
            "Data\\b.dds.1",
            "Data\\a.dds.99999999999",
        ] {
            assert_eq!(classify(other, header), None, "{other} is no part");
        }
    }
}
