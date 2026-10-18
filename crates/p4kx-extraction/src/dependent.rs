use std::collections::HashSet;

/// Splits the indices of `paths` into those whose files may be written in parallel and, in order,
/// those whose path collides with an earlier one by name or as a file where a folder is needed.
///
/// Paths compare ignoring case, with `/` and `\` alike.
pub(crate) fn split_dependent<'a>(
    paths: impl IntoIterator<Item = &'a str>,
) -> (Vec<usize>, Vec<usize>) {
    let mut files = HashSet::new();
    let mut folders = HashSet::new();
    let mut independent = Vec::new();
    let mut dependent = Vec::new();
    for (index, path) in paths.into_iter().enumerate() {
        let key = path.replace('/', "\\").to_lowercase();
        let parents: Vec<&str> = key
            .match_indices('\\')
            .filter_map(|(at, _)| key.get(..at))
            .collect();
        let free = !files.contains(key.as_str())
            && !folders.contains(key.as_str())
            && parents.iter().all(|parent| !files.contains(*parent));
        folders.extend(parents.iter().map(|parent| (*parent).to_owned()));
        files.insert(key);
        if free {
            independent.push(index);
        } else {
            dependent.push(index);
        }
    }
    (independent, dependent)
}
