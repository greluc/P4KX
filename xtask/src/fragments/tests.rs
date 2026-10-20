use std::fs;
use std::path::{Path, PathBuf};

use super::{check, parse, release, today, valid_date, valid_name, valid_version};

struct Repository(PathBuf);

impl Repository {
    fn new(test: &str, changelog: &str, fragments: &[(&str, &str)]) -> Self {
        let root =
            std::env::temp_dir().join(format!("xtask-fragments-{}-{test}", std::process::id()));
        let _ignored = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("changelog.d")).expect("create the directory");
        fs::write(root.join("CHANGELOG.md"), changelog).expect("write the changelog");
        for (name, text) in fragments {
            fs::write(root.join("changelog.d").join(name), text).expect("write a fragment");
        }
        Self(root)
    }

    fn path(&self) -> &Path {
        &self.0
    }

    fn changelog(&self) -> String {
        fs::read_to_string(self.0.join("CHANGELOG.md")).expect("read the changelog")
    }

    fn fragments(&self) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(self.0.join("changelog.d"))
            .expect("list the fragments")
            .map(|entry| {
                entry
                    .expect("entry")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        names.sort();
        names
    }
}

impl Drop for Repository {
    fn drop(&mut self) {
        let _ignored = fs::remove_dir_all(&self.0);
    }
}

const HEADER: &str = "# Changelog\n\n";

#[test]
fn names_are_an_issue_number_with_an_optional_suffix() {
    for name in ["345.md", "345-fixed.md", "7-a-b2.md"] {
        assert!(valid_name(name), "{name}");
    }
    for name in [
        "x.md", "345.txt", "345-.md", "-a.md", "345-A.md", "345_a.md", ".md", "345",
    ] {
        assert!(!valid_name(name), "{name}");
    }
}

#[test]
fn a_fragment_needs_a_known_section_and_a_list() {
    assert_eq!(
        parse("### Fixed\n\n- a\n  more\n- b\n"),
        Ok((4, "- a\n  more\n- b".to_owned()))
    );
    assert!(parse("").is_err(), "empty");
    assert!(parse("- a\n").is_err(), "no section");
    assert!(parse("### Fixes\n\n- a\n").is_err(), "unknown section");
    assert!(parse("### Fixed\n\n").is_err(), "no item");
    assert!(
        parse("### Fixed\n\n- a\n\n#### Sub\n").is_err(),
        "heading inside"
    );
    assert!(
        parse("### Fixed\n\ntext\n- a\n").is_err(),
        "text outside an item"
    );
}

#[test]
fn check_reports_bad_names_and_content() {
    let repository = Repository::new(
        "check",
        HEADER,
        &[
            ("1.md", "### Added\n\n- ok\n"),
            ("bad.md", "### Added\n\n- ok\n"),
            ("2.md", "nope\n"),
        ],
    );
    let violations = check(repository.path()).expect("check");
    assert_eq!(violations.len(), 2, "{violations:?}");
    assert!(
        violations[0].starts_with("changelog.d/2.md: must start"),
        "{violations:?}"
    );
    assert!(
        violations[1].starts_with("changelog.d/bad.md: the name"),
        "{violations:?}"
    );
}

#[test]
fn check_passes_without_the_folder_and_ignores_its_readme() {
    let repository = Repository::new("readme", HEADER, &[("README.md", "free text\n")]);
    assert_eq!(
        check(repository.path()).expect("check"),
        Vec::<String>::new()
    );
    fs::remove_dir_all(repository.path().join("changelog.d")).expect("remove");
    assert_eq!(
        check(repository.path()).expect("check"),
        Vec::<String>::new()
    );
}

#[test]
fn release_assembles_fragments_and_unreleased_in_order() {
    let changelog = "# Changelog\n\n## [Unreleased]\n\n### Fixed\n\n- old fix\n\n### Added\n\n- old add\n  more\n\n## [0.1.0]\n\n- First.\n";
    let repository = Repository::new(
        "release",
        changelog,
        &[
            ("10.md", "### Added\n\n- ten\n"),
            ("9-b.md", "### Security\n\n- nine b\n"),
            ("9.md", "### Fixed\n\n- nine\n"),
            ("README.md", "keep me\n"),
        ],
    );
    release(repository.path(), "0.3.0", "2026-11-01").expect("release");
    assert_eq!(
        repository.changelog(),
        "# Changelog\n\n## [Unreleased]\n\n## [0.3.0] - 2026-11-01\n\n### Added\n\n- old add\n  more\n- ten\n\n### Fixed\n\n- old fix\n- nine\n\n### Security\n\n- nine b\n\n## [0.1.0]\n\n- First.\n"
    );
    assert_eq!(repository.fragments(), vec!["README.md".to_owned()]);
}

#[test]
fn release_works_without_an_unreleased_body() {
    let repository = Repository::new(
        "empty-unreleased",
        "# Changelog\n\n## [Unreleased]\n",
        &[("1.md", "### Changed\n\n- one\n")],
    );
    release(repository.path(), "1.0.0", "2027-01-02").expect("release");
    assert_eq!(
        repository.changelog(),
        "# Changelog\n\n## [Unreleased]\n\n## [1.0.0] - 2027-01-02\n\n### Changed\n\n- one\n\n"
    );
}

type Case = (
    &'static str,
    String,
    Vec<(&'static str, &'static str)>,
    &'static str,
);

#[test]
fn release_refuses_and_keeps_everything_when_it_cannot_proceed() {
    let valid = ("1.md", "### Added\n\n- one\n");
    let unreleased = "## [Unreleased]\n\n";
    let cases: [Case; 5] = [
        (
            "invalid",
            unreleased.to_owned(),
            vec![("1.md", "nope\n")],
            "fix the fragments",
        ),
        (
            "exists",
            format!("{unreleased}## [0.3.0] - 2026-01-01\n"),
            vec![valid],
            "already",
        ),
        (
            "no-unreleased",
            "## [0.1.0]\n".to_owned(),
            vec![valid],
            "no `## [Unreleased]`",
        ),
        (
            "nothing",
            unreleased.to_owned(),
            vec![],
            "nothing to release",
        ),
        (
            "stray",
            "## [Unreleased]\n\n### Oops\n\n- x\n".to_owned(),
            vec![valid],
            "unknown section",
        ),
    ];
    for (test, changelog, fragments, message) in cases {
        let repository = Repository::new(test, &changelog, &fragments);
        let error = release(repository.path(), "0.3.0", "2026-11-01").expect_err(test);
        assert!(format!("{error:#}").contains(message), "{test}: {error:#}");
        assert_eq!(repository.changelog(), changelog, "{test}");
        assert_eq!(repository.fragments().len(), fragments.len(), "{test}");
    }
}

#[test]
fn versions_and_dates_are_validated() {
    assert!(valid_version("0.3.0") && valid_version("1.2.3-rc.1"));
    assert!(!valid_version("1.2") && !valid_version("v1.2.3") && !valid_version("1..3"));
    assert!(valid_date("2026-11-01") && !valid_date("2026-1-01") && !valid_date("20261101x"));
    assert!(valid_date(&today().expect("today")));
}
