// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The animation names of a Mannequin animation database (`.adb`) and of the databases it nests.

use std::collections::HashSet;

use crate::markup::{self, Budget, Element};
use crate::{ModelFailure, ModelProvider};

/// The most bytes of one database that are read.
const MAX_DATABASE_BYTES: u64 = 16 << 20;

/// The most databases read for one Entity: the one it names and those nested in it.
const MAX_DATABASES: usize = 16;

/// The most animation names kept.
const MAX_NAMES: usize = 1 << 16;

/// The animation names of a database and what could not be read of its nested databases.
#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct Clips {
    /// The names in the order of their first use, each once, ignoring ASCII case.
    pub(super) names: Vec<String>,
    /// The nested databases that could not be read, each with why.
    pub(super) unread: Vec<(String, String)>,
}

/// Why the database an Entity names gives no clips.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum DatabaseProblem {
    Missing,
    Unreadable(String),
}

/// Reads the animation names of the database at `path`, a path below `Data`, and of the databases
/// its `SubADB` elements name, breadth first.
///
/// # Errors
///
/// Returns why the database at `path` itself cannot be read; a nested database that cannot be read
/// is listed in [`Clips::unread`].
pub(super) fn clips(models: &dyn ModelProvider, path: &str) -> Result<Clips, DatabaseProblem> {
    let mut clips = Clips::default();
    let mut seen_names: HashSet<String> = HashSet::new();
    let mut seen_files: HashSet<String> = HashSet::from([key(path)]);
    let mut queue = vec![path.to_owned()];
    let mut next = 0;
    while let Some(file) = queue.get(next).cloned() {
        let nested = match read(models, &file) {
            Ok(element) => collect(&element, &mut clips.names, &mut seen_names),
            Err(problem) if next == 0 => return Err(problem),
            Err(problem) => {
                clips.unread.push((file, describe(&problem)));
                Vec::new()
            }
        };
        for file in nested {
            if queue.len() < MAX_DATABASES && seen_files.insert(key(&file)) {
                queue.push(file);
            }
        }
        next += 1;
    }
    Ok(clips)
}

fn key(path: &str) -> String {
    path.replace('\\', "/").to_ascii_lowercase()
}

fn describe(problem: &DatabaseProblem) -> String {
    match problem {
        DatabaseProblem::Missing => "it is not in the Archive".to_owned(),
        DatabaseProblem::Unreadable(reason) => reason.clone(),
    }
}

fn read(models: &dyn ModelProvider, path: &str) -> Result<Element, DatabaseProblem> {
    let bytes = models
        .file(path, MAX_DATABASE_BYTES)
        .map_err(|failure| match failure {
            ModelFailure::Missing => DatabaseProblem::Missing,
            other => DatabaseProblem::Unreadable(other.to_string()),
        })?;
    markup::parse(&bytes, &mut Budget::new(bytes.len())).map_err(DatabaseProblem::Unreadable)
}

/// Adds the names of the `Animation` elements below `root` to `names`, and returns the files of
/// its `SubADB` elements.
fn collect(root: &Element, names: &mut Vec<String>, seen: &mut HashSet<String>) -> Vec<String> {
    let mut nested = Vec::new();
    let mut stack = vec![root];
    while let Some(element) = stack.pop() {
        if element.name.eq_ignore_ascii_case("Animation")
            && let Some(name) = element.attribute("name").filter(|name| !name.is_empty())
            && names.len() < MAX_NAMES
            && seen.insert(name.to_ascii_lowercase())
        {
            names.push(name.to_owned());
        }
        if element.name.eq_ignore_ascii_case("SubADB")
            && let Some(file) = element.attribute("File").filter(|file| !file.is_empty())
        {
            nested.push(file.to_owned());
        }
        stack.extend(element.children.iter().rev());
    }
    nested
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ProvidedModel;

    struct Files(Vec<(&'static str, &'static str)>);

    impl ModelProvider for Files {
        fn model(&self, _: &str, _: u32) -> Result<ProvidedModel, ModelFailure> {
            Err(ModelFailure::Missing)
        }

        fn library(&self, _: &str, _: &str) -> p4kx_model::MaterialStatus {
            p4kx_model::MaterialStatus::Unnamed
        }

        fn file(&self, path: &str, _: u64) -> Result<Vec<u8>, ModelFailure> {
            self.0
                .iter()
                .find(|(name, _)| name.eq_ignore_ascii_case(path))
                .map(|(_, text)| text.as_bytes().to_vec())
                .ok_or(ModelFailure::Missing)
        }
    }

    const ROOT: &str = r#"<AnimDB>
        <SubADBs><SubADB File="Animations/sub.adb"/><SubADB File="animations\gone.adb"/></SubADBs>
        <FragmentList>
          <Idle><Fragment><AnimLayer><Animation name="idle_01"/></AnimLayer></Fragment>
                <Fragment><AnimLayer><Animation name="IDLE_01"/><Animation name="walk"/></AnimLayer></Fragment></Idle>
        </FragmentList></AnimDB>"#;

    #[test]
    fn names_are_listed_once_in_order_of_use_with_those_of_nested_databases_after() {
        let files = Files(vec![
            ("a.adb", ROOT),
            (
                "Animations/sub.adb",
                r#"<AnimDB><FragmentList><X><Fragment><AnimLayer><Animation name="walk"/><Animation name="roar"/></AnimLayer></Fragment></X></FragmentList></AnimDB>"#,
            ),
        ]);

        let clips = clips(&files, "a.adb").expect("clips");

        assert_eq!(clips.names, ["idle_01", "walk", "roar"]);
        assert_eq!(clips.unread.len(), 1);
        assert_eq!(clips.unread[0].0, "animations\\gone.adb");
    }

    #[test]
    fn a_database_that_names_itself_is_read_once() {
        let files = Files(vec![(
            "a.adb",
            r#"<AnimDB><SubADBs><SubADB File="A.adb"/></SubADBs></AnimDB>"#,
        )]);

        let clips = clips(&files, "a.adb").expect("clips");

        assert_eq!(clips, Clips::default());
    }

    #[test]
    fn a_missing_or_unreadable_root_database_is_a_problem() {
        let files = Files(vec![("bad.adb", "not xml")]);

        assert_eq!(
            clips(&files, "none.adb").expect_err("missing"),
            DatabaseProblem::Missing
        );
        assert!(matches!(
            clips(&files, "bad.adb").expect_err("unreadable"),
            DatabaseProblem::Unreadable(_)
        ));
    }
}
