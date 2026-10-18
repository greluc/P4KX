use std::cmp::Ordering;
use std::collections::BTreeMap;

use p4kx_extraction::{Archive, Entry, Filter};

use crate::error::ToolError;
use crate::page::Page;
use crate::shapes::{ArchiveInfo, BuildShape, EntryShape, FolderItem, FolderResult, SearchResult};

/// Returns the summary of `archive`.
pub(crate) fn info(archive: &Archive) -> ArchiveInfo {
    let entries = archive.entries();
    ArchiveInfo {
        archive: archive.path().display().to_string(),
        channel: p4kx_channels::channel_name(archive.path()),
        build: archive.build().map(BuildShape::from),
        entry_count: entries.len(),
        encrypted_count: entries.iter().filter(|entry| entry.is_encrypted()).count(),
        compressed_size: entries.iter().map(Entry::compressed_size).sum(),
        size: entries.iter().map(Entry::uncompressed_size).sum(),
    }
}

/// Returns one page of the Entries of `archive` that `filter` matches, in Archive order.
pub(crate) fn search(
    archive: &Archive,
    filter: &str,
    page: Page,
) -> Result<SearchResult, ToolError> {
    let filter = Filter::new(filter);
    let mut total = 0;
    let mut entries = Vec::with_capacity(page.limit());
    for entry in archive.entries() {
        if filter.matches(entry.path()) {
            if page.contains(total) {
                entries.push(EntryShape::from(entry));
            }
            total += 1;
        }
    }
    Ok(SearchResult {
        archive: archive.path().display().to_string(),
        filter: filter.as_str().to_owned(),
        total,
        entries,
        next_cursor: page.next_cursor(total)?,
    })
}

/// Returns one page of the content of `folder`: its subfolders by name ignoring ASCII case,
/// then its Entries in Archive order. Case and separator kind of `folder` do not matter.
pub(crate) fn list(archive: &Archive, folder: &str, page: Page) -> Result<FolderResult, ToolError> {
    let folder = folder.trim_matches(is_separator);
    let mut folders: BTreeMap<FolderKey<'_>, (&str, usize)> = BTreeMap::new();
    let mut files: Vec<&Entry> = Vec::new();
    for entry in archive.entries() {
        let path = entry.path();
        let Some(start) = content_start(path, folder) else {
            continue;
        };
        let rest = path.get(start..).unwrap_or_default();
        match rest.find(is_separator) {
            None => files.push(entry),
            Some(end) => {
                let folder_path = path.get(..start + end).unwrap_or(path);
                let name = rest.get(..end).unwrap_or(rest);
                folders.entry(FolderKey(name)).or_insert((folder_path, 0)).1 += 1;
            }
        }
    }
    let total = folders.len() + files.len();
    let mut items = Vec::with_capacity(page.limit().min(total));
    for (index, (path, entry_count)) in folders.into_values().enumerate() {
        if page.contains(index) {
            items.push(FolderItem::Folder {
                path: path.to_owned(),
                entry_count,
            });
        }
    }
    let first_file = total - files.len();
    for (index, entry) in files.into_iter().enumerate() {
        if page.contains(first_file + index) {
            items.push(FolderItem::Entry(EntryShape::from(entry)));
        }
    }
    Ok(FolderResult {
        archive: archive.path().display().to_string(),
        folder: folder.to_owned(),
        total,
        items,
        next_cursor: page.next_cursor(total)?,
    })
}

const fn is_separator(c: char) -> bool {
    c == '/' || c == '\\'
}

/// Returns where the part of `path` below `folder` starts, or `None` when `path` is not below
/// it; an empty `folder` is the top level.
fn content_start(path: &str, folder: &str) -> Option<usize> {
    if folder.is_empty() {
        return Some(0);
    }
    let head = path.as_bytes().get(..folder.len())?;
    let same = head.iter().zip(folder.bytes()).all(|(&a, b)| {
        a.eq_ignore_ascii_case(&b) || (is_separator(a.into()) && is_separator(b.into()))
    });
    let separator = path.as_bytes().get(folder.len()).copied();
    (same && separator.is_some_and(|byte| is_separator(byte.into()))).then_some(folder.len() + 1)
}

/// A folder name ordered and compared ignoring ASCII case.
#[derive(Debug, Clone, Copy)]
struct FolderKey<'a>(&'a str);

impl Ord for FolderKey<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0
            .bytes()
            .map(|byte| byte.to_ascii_lowercase())
            .cmp(other.0.bytes().map(|byte| byte.to_ascii_lowercase()))
    }
}

impl PartialOrd for FolderKey<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for FolderKey<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for FolderKey<'_> {}

#[cfg(test)]
mod tests {
    use super::content_start;

    #[test]
    fn content_starts_after_the_folder_ignoring_case_and_separator_kind() {
        assert_eq!(
            content_start(r"Data\Libs\a.xml", "data/libs"),
            Some(10),
            "below"
        );
        assert_eq!(content_start(r"Data\Libs\a.xml", ""), Some(0), "top level");
        assert_eq!(
            content_start(r"Data\LibsX\a.xml", "Data/Libs"),
            None,
            "a longer name"
        );
        assert_eq!(
            content_start(r"Data\Libs", "Data/Libs"),
            None,
            "the folder itself"
        );
        assert_eq!(
            content_start("Dä", "D\u{e4}x"),
            None,
            "shorter than the folder"
        );
    }
}
