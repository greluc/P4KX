use std::collections::HashSet;

/// The paths claimed so far by the files of a run, to find files that collide with an earlier one.
#[derive(Default)]
pub(crate) struct Collisions {
    files: HashSet<String>,
    folders: HashSet<String>,
}

impl Collisions {
    /// Claims `path` for a file; returns whether it is free, that is it collides with no earlier
    /// file by name or as a file where a folder is needed.
    ///
    /// Paths compare ignoring case, with `/` and `\` alike.
    pub(crate) fn claim(&mut self, path: &str) -> bool {
        let key = path.replace('/', "\\").to_lowercase();
        let parents: Vec<&str> = key
            .match_indices('\\')
            .filter_map(|(at, _)| key.get(..at))
            .collect();
        let free = !self.files.contains(key.as_str())
            && !self.folders.contains(key.as_str())
            && parents.iter().all(|parent| !self.files.contains(*parent));
        self.folders
            .extend(parents.iter().map(|parent| (*parent).to_owned()));
        self.files.insert(key);
        free
    }
}

/// Splits the indices of `paths` into those whose files may be written in parallel and, in order,
/// those whose path collides with an earlier one by name or as a file where a folder is needed.
///
/// Paths compare ignoring case, with `/` and `\` alike.
pub(crate) fn split_dependent<'a>(
    paths: impl IntoIterator<Item = &'a str>,
) -> (Vec<usize>, Vec<usize>) {
    let mut collisions = Collisions::default();
    let mut independent = Vec::new();
    let mut dependent = Vec::new();
    for (index, path) in paths.into_iter().enumerate() {
        if collisions.claim(path) {
            independent.push(index);
        } else {
            dependent.push(index);
        }
    }
    (independent, dependent)
}
