use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use p4kx_archive::{Archive, Entry, EntryReader};
use p4kx_texture::SiblingReader;
use rayon::prelude::*;

use crate::entry_path::same_entry_path;

const ALPHA_SUFFIX: &str = "a";

/// Where the header and the parts of one split texture are stored.
#[derive(Debug, Clone)]
pub(crate) enum Parts<'a> {
    /// Entries of an Archive, by part number.
    Archive {
        archive: &'a Archive,
        header: &'a Entry,
        parts: BTreeMap<u32, &'a Entry>,
    },
    /// Files next to a header file on disk, part `N` named `<base>.<N><suffix>`.
    Files {
        header: PathBuf,
        base: PathBuf,
        suffix: &'static str,
    },
}

/// The parts of a texture and of its alpha tail, if it has one.
#[derive(Debug)]
pub(crate) struct Found<'a> {
    pub(crate) colour: Parts<'a>,
    pub(crate) alpha: Option<Parts<'a>>,
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
    /// Finds the Entry at `path`, its parts and its alpha tail in one parallel pass over the
    /// Archive, or `None` when no Entry has that path.
    ///
    /// A path ending in `.a` names an alpha tail itself; its parts are `.1a` … `.Na` and it has
    /// no alpha tail of its own.
    pub(crate) fn in_archive(archive: &'a Archive, path: &str) -> Option<Found<'a>> {
        let (base, is_alpha) = split_alpha(path);
        let found: Vec<(Role, &Entry)> = archive
            .entries()
            .par_iter()
            .filter_map(|entry| classify(entry.path(), base).map(|role| (role, entry)))
            .collect();
        let find = |wanted: Role| {
            found
                .iter()
                .find_map(|(role, entry)| (*role == wanted).then_some(*entry))
        };
        let parts_of = |alpha: bool| {
            let mut parts = BTreeMap::new();
            for (role, entry) in &found {
                if let Role::Part { index, alpha: is } = *role
                    && is == alpha
                {
                    parts.entry(index).or_insert(*entry);
                }
            }
            parts
        };
        let header = find(if is_alpha {
            Role::AlphaHeader
        } else {
            Role::Header
        })?;
        let alpha = if is_alpha {
            None
        } else {
            find(Role::AlphaHeader).map(|header| Self::Archive {
                archive,
                header,
                parts: parts_of(true),
            })
        };
        Some(Found {
            colour: Self::Archive {
                archive,
                header,
                parts: parts_of(is_alpha),
            },
            alpha,
        })
    }

    /// Returns the header file at `path` with its parts next to it, and its alpha tail
    /// `<path>.a` when that file exists; a path ending in `.a` names an alpha tail itself.
    pub(crate) fn on_disk(path: &Path) -> Found<'static> {
        let is_alpha = path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case(ALPHA_SUFFIX));
        if is_alpha {
            return Found {
                colour: Parts::Files {
                    header: path.to_path_buf(),
                    base: path.with_extension(""),
                    suffix: ALPHA_SUFFIX,
                },
                alpha: None,
            };
        }
        let alpha_header = with_suffix(path, ".a");
        let alpha = alpha_header.is_file().then(|| Parts::Files {
            header: alpha_header,
            base: path.to_path_buf(),
            suffix: ALPHA_SUFFIX,
        });
        Found {
            colour: Parts::Files {
                header: path.to_path_buf(),
                base: path.to_path_buf(),
                suffix: "",
            },
            alpha,
        }
    }

    /// Returns the path of part `index` on disk, or `None` for parts in an Archive.
    pub(crate) fn file_of(&self, index: u32) -> Option<PathBuf> {
        match self {
            Self::Archive { .. } => None,
            Self::Files { base, suffix, .. } => {
                Some(with_suffix(base, &format!(".{index}{suffix}")))
            }
        }
    }
}

fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = OsString::from(path.as_os_str());
    name.push(suffix);
    PathBuf::from(name)
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
    AlphaHeader,
    Part { index: u32, alpha: bool },
}

/// Returns `path` without a trailing `.a`, and whether it had one.
fn split_alpha(path: &str) -> (&str, bool) {
    match path
        .len()
        .checked_sub(2)
        .and_then(|at| path.split_at_checked(at))
    {
        Some((base, ".a" | ".A")) => (base, true),
        _ => (path, false),
    }
}

/// Returns the role of `candidate` for the texture whose header path is `base`, or `None` when it
/// is neither the header, its alpha tail nor one of their parts; separators and ASCII case do
/// not matter.
fn classify(candidate: &str, base: &str) -> Option<Role> {
    let (start, rest) = candidate.split_at_checked(base.len())?;
    if !same_entry_path(start, base) {
        return None;
    }
    let Some(suffix) = rest.strip_prefix('.') else {
        return rest.is_empty().then_some(Role::Header);
    };
    if suffix.eq_ignore_ascii_case(ALPHA_SUFFIX) {
        return Some(Role::AlphaHeader);
    }
    let (digits, alpha) = suffix
        .strip_suffix(['a', 'A'])
        .map_or((suffix, false), |digits| (digits, true));
    if digits.starts_with('0') || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok().map(|index| Role::Part { index, alpha })
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
    use super::{Role, classify, split_alpha};

    #[test]
    fn parts_and_the_alpha_tail_are_siblings_of_the_header() {
        let header = "Data\\a.dds";
        let part = |index, alpha| Some(Role::Part { index, alpha });
        for (candidate, role) in [
            ("data/A.DDS", Some(Role::Header)),
            ("Data\\a.dds.1", part(1, false)),
            ("Data\\a.dds.12", part(12, false)),
            ("Data\\a.dds.a", Some(Role::AlphaHeader)),
            ("Data/A.DDS.A", Some(Role::AlphaHeader)),
            ("Data\\a.dds.1a", part(1, true)),
            ("Data\\a.dds.7A", part(7, true)),
        ] {
            assert_eq!(classify(candidate, header), role, "{candidate}");
        }
        for other in [
            "Data\\a.dds.0",
            "Data\\a.dds.01",
            "Data\\a.dds.0a",
            "Data\\a.dds.aa",
            "Data\\a.dds.1b",
            "Data\\a.dds.",
            "Data\\a.ddsx",
            "Data\\b.dds.1",
            "Data\\a.dds.99999999999",
        ] {
            assert_eq!(classify(other, header), None, "{other} is no part");
        }
    }

    #[test]
    fn a_path_ending_in_a_names_an_alpha_tail() {
        assert_eq!(split_alpha("x.dds.a"), ("x.dds", true), "lower case");
        assert_eq!(split_alpha("x.dds.A"), ("x.dds", true), "upper case");
        assert_eq!(split_alpha("x.dds"), ("x.dds", false), "a colour header");
        assert_eq!(split_alpha("a"), ("a", false), "too short");
    }
}
