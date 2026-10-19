use p4kx_extraction::{Archive, Filter};

use crate::archives::Opened;
use crate::error::ToolError;
use crate::page::Page;
use crate::shapes::{
    ArchiveInfo, BuildShape, EntryShape, FolderItem, FolderResult, IssueShape, SearchResult,
};

const MAX_SHOWN_ISSUES: usize = 50;

/// Returns the summary of `archive`.
pub(crate) fn info(archive: &Archive) -> ArchiveInfo {
    let entries = archive.entries();
    let (encrypted_count, compressed_size, size) =
        entries
            .iter()
            .fold((0, 0_u64, 0_u64), |(encrypted, compressed, size), entry| {
                (
                    encrypted + usize::from(entry.is_encrypted()),
                    compressed.saturating_add(entry.compressed_size()),
                    size.saturating_add(entry.uncompressed_size()),
                )
            });
    ArchiveInfo {
        archive: archive.path().display().to_string(),
        channel: p4kx_channels::channel_name(archive.path()),
        build: archive.build().map(BuildShape::from),
        entry_count: entries.len(),
        encrypted_count,
        issue_count: archive.issues().len(),
        issues: archive
            .issues()
            .iter()
            .take(MAX_SHOWN_ISSUES)
            .map(|(index, issue)| IssueShape {
                path: archive
                    .entries()
                    .get(*index)
                    .map(|entry| entry.path().to_owned())
                    .unwrap_or_default(),
                issue: issue.to_string(),
            })
            .collect(),
        compressed_size,
        size,
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
/// then its Entries in path order. Case and separator kind of `folder` do not matter.
pub(crate) fn list(opened: &Opened, folder: &str, page: Page) -> Result<FolderResult, ToolError> {
    let archive = opened.archive();
    let tree = opened.tree();
    let (folders, files) = tree
        .find_folder(folder)
        .map(|id| (tree.folder(id).children(), tree.files(id)))
        .unwrap_or_default();
    let total = folders.len() + files.len();
    let folder_page = page.slice(folders);
    let file_offset = page.offset().saturating_sub(folders.len()).min(files.len());
    let file_page = files
        .get(file_offset..)
        .unwrap_or_default()
        .iter()
        .take(page.limit() - folder_page.len());
    let items = folder_page
        .iter()
        .map(|&child| {
            let child = tree.folder(child);
            FolderItem::Folder {
                path: child.path().to_owned(),
                entry_count: child.entry_count(),
            }
        })
        .chain(
            file_page
                .filter_map(|&id| tree.entry(archive, id))
                .map(|entry| FolderItem::Entry(EntryShape::from(entry))),
        )
        .collect();
    Ok(FolderResult {
        archive: archive.path().display().to_string(),
        folder: folder.trim_matches(['/', '\\']).to_owned(),
        total,
        items,
        next_cursor: page.next_cursor(total)?,
    })
}
