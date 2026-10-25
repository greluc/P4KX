// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::{BTreeSet, HashSet};
use std::ops::Bound;

/// The paths claimed so far by the files of a run, to find files that collide with an earlier one.
#[derive(Default)]
pub(crate) struct Collisions {
    files: HashSet<String>,
    folders: HashSet<String>,
    whole_folders: HashSet<String>,
    names: BTreeSet<String>,
    prefixes: BTreeSet<String>,
}

fn starts_with_first(set: &BTreeSet<String>, prefix: &str) -> bool {
    set.range::<str, _>((Bound::Included(prefix), Bound::Unbounded))
        .next()
        .is_some_and(|name| name.starts_with(prefix))
}

impl Collisions {
    /// Returns whether a name claimed as a prefix is a prefix of `key`, ending at an underscore.
    fn under_prefix(&self, key: &str) -> bool {
        key.match_indices('_')
            .filter_map(|(at, _)| key.get(..=at))
            .any(|prefix| self.prefixes.contains(prefix))
    }

    fn claim_prefix(&mut self, prefix: String) -> bool {
        let free = !starts_with_first(&self.names, &prefix)
            && !starts_with_first(&self.prefixes, &prefix)
            && !self.under_prefix(&prefix);
        self.prefixes.insert(prefix);
        free
    }

    /// Claims `path` for a file, with a trailing separator a whole folder whose files are not
    /// known yet, or with a trailing `*` every file whose name starts with `path`, which ends
    /// in an underscore; returns whether it is free, that is it collides with no earlier file by
    /// name, as a file where a folder is needed, inside or as a whole folder claimed earlier, or
    /// under an earlier prefix.
    ///
    /// Paths compare ignoring case, with `/` and `\` alike.
    pub(crate) fn claim(&mut self, path: &str) -> bool {
        let mut key = path.replace('/', "\\").to_lowercase();
        if let Some(prefix) = key.strip_suffix('*') {
            return self.claim_prefix(prefix.to_owned());
        }
        let whole = key.ends_with('\\');
        if whole {
            key.pop();
        }
        let under_prefix = self.under_prefix(&key);
        self.names.insert(key.clone());
        let parents: Vec<&str> = key
            .match_indices('\\')
            .filter_map(|(at, _)| key.get(..at))
            .collect();
        let free = !under_prefix
            && !self.files.contains(key.as_str())
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
    fn a_prefix_collides_with_files_under_it_before_and_after() {
        let mut collisions = Collisions::default();
        assert!(collisions.claim("Data\\a_layer0.png"), "an earlier file");
        assert!(!collisions.claim("Data\\a_*"), "the prefix over it");
        assert!(!collisions.claim("Data\\A_layer1.png"), "a later file");
        assert!(collisions.claim("Data\\ab.png"), "another stem");
        assert!(collisions.claim("Data\\a.png"), "the stem itself");
    }

    #[test]
    fn two_prefixes_collide_when_one_holds_the_other() {
        let mut collisions = Collisions::default();
        assert!(collisions.claim("Data\\a_*"), "the shorter stem");
        assert!(
            !collisions.claim("Data\\a_layer0_*"),
            "a longer one under it"
        );
        let mut reverse = Collisions::default();
        assert!(reverse.claim("Data\\a_layer0_*"), "the longer stem");
        assert!(!reverse.claim("Data\\a_*"), "a shorter one over it");
        assert!(reverse.claim("Data\\b_*"), "an unrelated stem");
    }

    #[test]
    fn a_whole_folder_collides_with_an_earlier_file_inside_it() {
        let mut collisions = Collisions::default();
        assert!(collisions.claim("Data\\bank\\1.wem"), "a loose file");
        assert!(!collisions.claim("Data\\bank\\"), "the folder holding it");
    }
}
