// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashSet;

/// The paths claimed so far by the files of a run, to find files that collide with an earlier one.
#[derive(Default)]
pub(crate) struct Collisions {
    files: HashSet<String>,
    folders: HashSet<String>,
    whole_folders: HashSet<String>,
}

impl Collisions {
    /// Claims `path` for a file, or with a trailing separator a whole folder whose files are not
    /// known yet; returns whether it is free, that is it collides with no earlier file by name, as
    /// a file where a folder is needed, or inside or as a whole folder claimed earlier.
    ///
    /// Paths compare ignoring case, with `/` and `\` alike.
    pub(crate) fn claim(&mut self, path: &str) -> bool {
        let mut key = path.replace('/', "\\").to_lowercase();
        let whole = key.ends_with('\\');
        if whole {
            key.pop();
        }
        let parents: Vec<&str> = key
            .match_indices('\\')
            .filter_map(|(at, _)| key.get(..at))
            .collect();
        let free = !self.files.contains(key.as_str())
            && !self.folders.contains(key.as_str())
            && parents.iter().all(|parent| {
                !self.files.contains(*parent) && !self.whole_folders.contains(*parent)
            });
        self.folders
            .extend(parents.iter().map(|parent| (*parent).to_owned()));
        if whole {
            self.folders.insert(key.clone());
            self.whole_folders.insert(key);
        } else {
            self.files.insert(key);
        }
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

#[cfg(test)]
mod tests {
    use super::Collisions;

    #[test]
    fn a_whole_folder_collides_with_files_inside_it_and_with_a_file_of_its_name() {
        let mut collisions = Collisions::default();
        assert!(collisions.claim("Data\\bank.bnk"), "the soundbank");
        assert!(collisions.claim("Data\\bank\\"), "its folder");
        assert!(!collisions.claim("data/BANK/1.ogg"), "a file inside it");
        assert!(!collisions.claim("Data\\bank"), "a file of its name");
        assert!(!collisions.claim("Data\\Bank/"), "the folder again");
        assert!(
            collisions.claim("Data\\banker\\1.ogg"),
            "a folder of a longer name"
        );
    }

    #[test]
    fn a_whole_folder_collides_with_an_earlier_file_inside_it() {
        let mut collisions = Collisions::default();
        assert!(collisions.claim("Data\\bank\\1.wem"), "a loose file");
        assert!(!collisions.claim("Data\\bank\\"), "the folder holding it");
    }
}
