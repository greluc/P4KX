use std::collections::{BTreeMap, HashMap};

use p4kx_archive::{Archive, Entry};
use rayon::prelude::*;

/// An Entry of the run with its position in the requested order.
pub(crate) type Indexed<'a> = (usize, &'a Entry);

/// One unit of work of an Extraction.
pub(crate) enum Item<'a> {
    /// An Entry written on its own.
    Entry(Indexed<'a>),
    /// A DDS texture converted as a whole.
    Texture(TextureJob<'a>),
}

/// A texture of the run: its header Entry, the parts that were requested with it and every part
/// the Archive holds for it.
pub(crate) struct TextureJob<'a> {
    pub(crate) header: Indexed<'a>,
    pub(crate) requested_parts: Vec<Indexed<'a>>,
    pub(crate) parts: BTreeMap<u32, &'a Entry>,
}

impl<'a> TextureJob<'a> {
    /// Returns every Entry of the run that belongs to the texture, the header first.
    pub(crate) fn entries(&self) -> impl Iterator<Item = Indexed<'a>> + '_ {
        std::iter::once(self.header).chain(self.requested_parts.iter().copied())
    }
}

impl Item<'_> {
    /// Returns the position of the item's first Entry in the requested order.
    pub(crate) const fn first_index(&self) -> usize {
        match self {
            Self::Entry((index, _)) => *index,
            Self::Texture(job) => job.header.0,
        }
    }

    /// Calls `visit` with each path below the target the item may write to, in lower case with
    /// `\` separators.
    pub(crate) fn output_keys(&self, mut visit: impl FnMut(String)) {
        match self {
            Self::Entry((_, entry)) => visit(key(entry.path())),
            Self::Texture(job) => {
                let header = key(job.header.1.path());
                let stem = header
                    .get(..header.len().saturating_sub(4))
                    .unwrap_or_default();
                visit(format!("{stem}.png"));
                visit(header);
                for (_, part) in &job.requested_parts {
                    visit(key(part.path()));
                }
            }
        }
    }
}

/// Splits `entries` into items: each requested `.dds` header with its requested parts and the
/// Archive's parts becomes one texture, every other Entry stays on its own.
pub(crate) fn group_textures<'a>(archive: &'a Archive, entries: &[&'a Entry]) -> Vec<Item<'a>> {
    let mut headers: HashMap<String, usize> = HashMap::new();
    for (index, entry) in entries.iter().enumerate() {
        if role(entry.path()) == Some(Role::Header) {
            headers.entry(key(entry.path())).or_insert(index);
        }
    }
    if headers.is_empty() {
        return entries
            .iter()
            .copied()
            .enumerate()
            .map(Item::Entry)
            .collect();
    }
    let mut jobs: HashMap<usize, TextureJob<'a>> = headers
        .values()
        .filter_map(|&index| {
            let header = *entries.get(index)?;
            Some((
                index,
                TextureJob {
                    header: (index, header),
                    requested_parts: Vec::new(),
                    parts: BTreeMap::new(),
                },
            ))
        })
        .collect();
    for (header, number, entry) in archive_parts(archive, &headers) {
        if let Some(job) = jobs.get_mut(&header) {
            job.parts.entry(number).or_insert(entry);
        }
    }
    let mut items = Vec::with_capacity(entries.len());
    for (index, entry) in entries.iter().copied().enumerate() {
        let owner = match role(entry.path()) {
            Some(Role::Part(header_path, _)) => headers.get(&key(header_path)).copied(),
            _ => None,
        };
        if jobs.contains_key(&index) {
            continue;
        }
        match owner.and_then(|header| jobs.get_mut(&header)) {
            Some(job) => job.requested_parts.push((index, entry)),
            None => items.push(Item::Entry((index, entry))),
        }
    }
    let mut jobs: Vec<_> = jobs.into_values().collect();
    jobs.sort_unstable_by_key(|job| job.header.0);
    items.extend(jobs.into_iter().map(Item::Texture));
    items.sort_unstable_by_key(Item::first_index);
    items
}

/// Finds the parts of the `headers` in one parallel pass over the Archive.
fn archive_parts<'a>(
    archive: &'a Archive,
    headers: &HashMap<String, usize>,
) -> Vec<(usize, u32, &'a Entry)> {
    archive
        .entries()
        .par_iter()
        .filter_map(|entry| {
            let Some(Role::Part(header_path, number)) = role(entry.path()) else {
                return None;
            };
            let header = *headers.get(&key(header_path))?;
            Some((header, number, entry))
        })
        .collect()
}

/// What an Entry is to texture Conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Role<'p> {
    /// A `.dds` header.
    Header,
    /// Part `N` of the header at the given path.
    Part(&'p str, u32),
}

/// Returns the role of the Entry at `path`, or `None` when it is no part of a texture.
pub(crate) fn role(path: &str) -> Option<Role<'_>> {
    if has_dds_extension(path) {
        return Some(Role::Header);
    }
    let (header, digits) = path.rsplit_once('.')?;
    if digits.is_empty()
        || digits.starts_with('0')
        || !digits.bytes().all(|byte| byte.is_ascii_digit())
        || !has_dds_extension(header)
    {
        return None;
    }
    digits.parse().ok().map(|number| Role::Part(header, number))
}

fn has_dds_extension(path: &str) -> bool {
    path.len() > 4
        && path
            .get(path.len() - 4..)
            .is_some_and(|extension| extension.eq_ignore_ascii_case(".dds"))
}

/// Returns the path in lower case with `\` separators, the key two paths collide on.
pub(crate) fn key(path: &str) -> String {
    path.replace('/', "\\").to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::{Role, role};

    #[test]
    fn headers_and_numbered_parts_are_recognised() {
        assert_eq!(role("Data\\a.dds"), Some(Role::Header), "header");
        assert_eq!(role("Data/A.DDS"), Some(Role::Header), "any case");
        assert_eq!(
            role("Data\\a.dds.12"),
            Some(Role::Part("Data\\a.dds", 12)),
            "part 12"
        );
        for other in [
            "Data\\a.dds.a",
            "Data\\a.dds.1a",
            "Data\\a.dds.0",
            "Data\\a.dds.01",
            "Data\\a.dds.",
            "Data\\a.ddsx",
            "Data\\a.png.1",
            "Data\\a.dds.99999999999",
            ".dds",
        ] {
            assert_eq!(role(other), None, "{other} is no texture Entry");
        }
    }
}
