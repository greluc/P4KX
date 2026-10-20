use rayon::slice::ParallelSliceMut;

use super::ReferenceIndex;
use crate::DataForge;
use crate::strings::compare_paths;

const NO_FILE: u32 = u32::MAX;

/// The record files of a DataForge and which of them refer to which.
///
/// A record file is a record path ignoring ASCII case, the kind of separator and a trailing
/// `.xml`; the Records sharing one belong to one file. There is an edge from file A to file B,
/// A different from B, when a Record of A refers to a Record of B, so a chain of References
/// through Records of one file ends at the files they reach. Unresolved References give no
/// edge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileGraph<'f> {
    paths: Vec<&'f str>,
    edges: Vec<(u32, u32)>,
}

impl<'f> FileGraph<'f> {
    /// Returns the record path of each file in record-path order, as written by its first Record.
    #[must_use]
    pub fn paths(&self) -> &[&'f str] {
        &self.paths
    }

    /// Returns the edges as pairs of positions in [`paths`](Self::paths), sorted and without
    /// repeats.
    #[must_use]
    pub fn edges(&self) -> &[(u32, u32)] {
        &self.edges
    }

    /// Returns the record paths of the files that the file at `file` refers to.
    pub fn refers_to(&self, file: u32) -> impl Iterator<Item = &'f str> + '_ {
        self.edges
            .iter()
            .filter(move |(from, _)| *from == file)
            .filter_map(|(_, to)| self.paths.get(*to as usize).copied())
    }

    /// Returns the position of the file the record path `path` names, ignoring ASCII case and
    /// the kind of separator.
    #[must_use]
    pub fn position(&self, path: &str) -> Option<u32> {
        let at = self
            .paths
            .binary_search_by(|candidate| compare_paths(candidate, path))
            .ok()?;
        u32::try_from(at).ok()
    }
}

impl ReferenceIndex {
    /// Groups the References by the record files of their Records and returns the edges between
    /// different files; `forge` is the DataForge the index was built from.
    #[must_use]
    pub fn file_graph<'f>(&self, forge: &'f DataForge<'f>) -> FileGraph<'f> {
        let mut file_of = vec![NO_FILE; forge.record_count()];
        let mut paths: Vec<&'f str> = Vec::new();
        for &record in forge.path_order() {
            let Some(path) = forge.path_of(record) else {
                continue;
            };
            if paths
                .last()
                .is_none_or(|last| compare_paths(last, path).is_ne())
            {
                paths.push(path);
            }
            if let Some(slot) = file_of.get_mut(record as usize) {
                *slot = u32::try_from(paths.len().saturating_sub(1)).unwrap_or(NO_FILE);
            }
        }
        let file = |record: u32| file_of.get(record as usize).copied().unwrap_or(NO_FILE);
        let mut edges: Vec<(u32, u32)> = self
            .sources
            .iter()
            .zip(&self.targets)
            .filter(|(_, target)| **target != super::UNRESOLVED)
            .map(|(source, target)| (file(*source), file(*target)))
            .filter(|(from, to)| from != to && *from != NO_FILE && *to != NO_FILE)
            .collect();
        edges.par_sort_unstable();
        edges.dedup();
        FileGraph { paths, edges }
    }
}
