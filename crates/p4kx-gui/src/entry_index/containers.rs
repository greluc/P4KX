use crate::sort::{compare_ignoring_case, file_name};
use p4kx_extraction::{Entry, EntryTree, FolderId};

/// The bytes an index keeps for each Entry besides the Entry itself: its id in path order, its
/// position in each sort order, its check mark and its search match.
pub(super) const INDEX_BYTES_PER_ENTRY: usize = 5 * size_of::<u32>() + 2;

const EXTENSION: &[u8] = b".socpak";

/// Returns whether the Entry at `path` opens as a container in the path tree: a `.socpak`,
/// ignoring case.
pub(crate) fn is_container(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes
        .len()
        .checked_sub(EXTENSION.len())
        .and_then(|start| bytes.get(start..))
        .is_some_and(|extension| extension.eq_ignore_ascii_case(EXTENSION))
}

/// Returns the containers among `sorted`, the Entries in path order, as their folder and id,
/// sorted by folder and then by name ignoring case.
pub(super) fn build(sorted: &[&Entry], tree: &EntryTree) -> Vec<(FolderId, u32)> {
    let mut found: Vec<(FolderId, u32)> = sorted
        .iter()
        .enumerate()
        .filter(|(_, entry)| is_container(entry.path()))
        .filter_map(|(id, entry)| Some((folder_of(tree, entry.path())?, u32::try_from(id).ok()?)))
        .collect();
    let name = |id: u32| {
        sorted
            .get(id as usize)
            .map_or("", |entry| file_name(entry.path()))
    };
    found.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then_with(|| compare_ignoring_case(name(a.1), name(b.1)))
    });
    found
}

/// Returns the containers of `containers`, as built by [`build`], that lie directly in `folder`.
pub(super) fn directly_in(containers: &[(FolderId, u32)], folder: FolderId) -> &[(FolderId, u32)] {
    let start = containers.partition_point(|&(other, _)| other < folder);
    let end = containers.partition_point(|&(other, _)| other <= folder);
    containers.get(start..end).unwrap_or_default()
}

fn folder_of(tree: &EntryTree, path: &str) -> Option<FolderId> {
    let parent = path
        .rfind(['\\', '/'])
        .map_or("", |end| path.get(..end).unwrap_or_default());
    tree.find_folder(parent)
}

#[cfg(test)]
mod tests {
    use super::is_container;

    #[test]
    fn a_socpak_is_a_container_in_any_case() {
        assert!(is_container("Data\\a\\room.socpak"), "lower case");
        assert!(is_container("Data\\a\\ROOM.SocPak"), "mixed case");
        assert!(
            !is_container("Data\\a\\room.socpak.xml"),
            "another extension"
        );
        assert!(!is_container("socpak"), "shorter than the extension");
        assert!(!is_container("Data\\level.pak"), "a pak");
    }
}
