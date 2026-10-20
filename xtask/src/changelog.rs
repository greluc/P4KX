use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::fragments;

pub(crate) const FILE: &str = "CHANGELOG.md";

/// Returns a message when `CHANGELOG.md` has no notes for the workspace version, repeats or
/// misorders a `### ` heading, or a fragment in `changelog.d/` is invalid.
pub(crate) fn check(root: &Path) -> Result<Vec<String>> {
    let version = workspace_version(root)?;
    let text = read(root)?;
    let mut violations = Vec::new();
    if section(&text, &version).is_none() {
        violations.push(missing(&version));
    }
    violations.extend(subsection_violations(&text));
    violations.extend(fragments::check(root)?);
    Ok(violations)
}

pub(crate) const ORDER: [&str; 6] = [
    "Added",
    "Changed",
    "Deprecated",
    "Removed",
    "Fixed",
    "Security",
];

/// Returns a message for every `### ` heading that repeats within one `## ` section or breaks the
/// Keep a Changelog order.
fn subsection_violations(text: &str) -> Vec<String> {
    let mut violations = Vec::new();
    let mut sections: Vec<(&str, Vec<&str>)> = Vec::new();
    for line in text.lines() {
        let line = line.trim_end();
        if line.starts_with("## ") {
            sections.push((line, Vec::new()));
        } else if let (Some(name), Some((_, headings))) =
            (line.strip_prefix("### "), sections.last_mut())
        {
            headings.push(name.trim());
        }
    }
    for (title, headings) in sections {
        let mut seen: Vec<&str> = Vec::new();
        for name in &headings {
            if seen.contains(name) {
                continue;
            }
            seen.push(name);
            let count = headings.iter().filter(|other| *other == name).count();
            if count > 1 {
                violations.push(format!("{FILE}: `{title}` has `### {name}` {count} times"));
            }
        }
        let mut last: Option<(usize, &str)> = None;
        for name in seen {
            let Some(rank) = ORDER.iter().position(|known| *known == name) else {
                continue;
            };
            match last {
                Some((previous, previous_name)) if rank < previous => violations.push(format!(
                    "{FILE}: `{title}` has `### {name}` after `### {previous_name}`"
                )),
                _ => last = Some((rank, name)),
            }
        }
    }
    violations
}

/// Writes the notes of the Release tagged `tag` to `output`.
///
/// # Errors
///
/// When `tag` is not `v` and the workspace version, or `CHANGELOG.md` has no notes for it.
pub(crate) fn write_notes(root: &Path, tag: &str, output: &Path) -> Result<()> {
    let version = workspace_version(root)?;
    if tag.strip_prefix('v') != Some(version.as_str()) {
        bail!("the tag `{tag}` does not match the workspace version {version}");
    }
    let text = read(root)?;
    let Some(notes) = section(&text, &version) else {
        bail!(missing(&version));
    };
    fs::write(output, with_notice(notes))
        .with_context(|| format!("cannot write {}", output.display()))
}

/// The legal notice every Release states below its notes.
const NOTICE: &str = "P4K-Extractor is an unofficial Star Citizen fan project, not affiliated with \
the Cloud Imperium group of companies and not endorsed by it. It ships no game data and reads only \
your own Star Citizen installation. You are responsible for complying with the Star Citizen End \
User Licence Agreement (EULA) and Terms of Service when you use it.\n\nAll content not authored by \
the P4K-Extractor contributors or its users is the property of its respective owners. Star \
Citizen®, Roberts Space Industries® and Cloud Imperium® are registered trademarks of Cloud \
Imperium Rights LLC.\n\nCopyright (c) 2026 KRT and the P4K-Extractor contributors";

/// Returns `notes` followed by the legal notice.
fn with_notice(notes: &str) -> String {
    format!("{notes}\n\n---\n\n{NOTICE}\n")
}

fn missing(version: &str) -> String {
    format!("{FILE} has no notes under `## [{version}]`")
}

pub(crate) fn read(root: &Path) -> Result<String> {
    fs::read_to_string(root.join(FILE)).with_context(|| format!("cannot read {FILE}"))
}

fn workspace_version(root: &Path) -> Result<String> {
    let manifest = fs::read_to_string(root.join("Cargo.toml")).context("cannot read Cargo.toml")?;
    let manifest: toml::Table = toml::from_str(&manifest).context("Cargo.toml is not TOML")?;
    manifest
        .get("workspace")
        .and_then(|workspace| workspace.get("package"))
        .and_then(|package| package.get("version"))
        .and_then(toml::Value::as_str)
        .map(str::to_owned)
        .context("Cargo.toml has no workspace.package.version")
}

/// Returns the trimmed text below the `## [version]` heading up to the next `## ` heading, or
/// `None` when the heading is missing or nothing stands below it.
fn section<'a>(text: &'a str, version: &str) -> Option<&'a str> {
    let heading = format!("## [{version}]");
    let mut offset = 0;
    let mut start = None;
    for line in text.split_inclusive('\n') {
        let end = offset + line.len();
        if let Some(start) = start {
            if line.starts_with("## ") {
                return non_empty(text.get(start..offset)?);
            }
        } else if line.trim_end() == heading || line.starts_with(&format!("{heading} ")) {
            start = Some(end);
        }
        offset = end;
    }
    non_empty(text.get(start?..)?)
}

fn non_empty(notes: &str) -> Option<&str> {
    let notes = notes.trim();
    (!notes.is_empty()).then_some(notes)
}

#[cfg(test)]
mod tests {
    use super::{section, subsection_violations, with_notice};

    const CHANGELOG: &str = "# Changelog\n\n## [0.2.0] - 2026-11-01\n\n### Added\n\n- Search.\n\n## [0.1.0] - 2026-10-20\n\n- First Release.\n";

    #[test]
    fn the_notes_of_a_version_end_at_the_next_heading() {
        assert_eq!(
            section(CHANGELOG, "0.2.0"),
            Some("### Added\n\n- Search."),
            "only 0.2.0"
        );
        assert_eq!(
            section(CHANGELOG, "0.1.0"),
            Some("- First Release."),
            "the last section runs to the end"
        );
    }

    #[test]
    fn a_repeated_heading_in_unreleased_is_reported() {
        let text = "## [Unreleased]\n\n### Fixed\n\n- a\n\n### Added\n\n### Fixed\n\n### Fixed\n";
        let violations = subsection_violations(text);
        assert!(
            violations
                .contains(&"CHANGELOG.md: `## [Unreleased]` has `### Fixed` 3 times".to_owned()),
            "{violations:?}"
        );
    }

    #[test]
    fn a_repeated_heading_in_a_released_section_is_reported() {
        let text =
            "## [Unreleased]\n\n### Added\n\n## [0.2.0] - 2026-11-01\n\n### Added\n\n### Added\n";
        assert_eq!(
            subsection_violations(text),
            vec!["CHANGELOG.md: `## [0.2.0] - 2026-11-01` has `### Added` 2 times".to_owned()],
        );
    }

    #[test]
    fn distinct_headings_pass() {
        assert_eq!(subsection_violations(CHANGELOG), Vec::<String>::new());
        let text = "## [Unreleased]\n\n### Added\n\n### Changed\n\n## [0.1.0]\n\n### Added\n\n### Fixed\n\n### Security\n";
        assert_eq!(subsection_violations(text), Vec::<String>::new());
    }

    #[test]
    fn headings_out_of_order_are_reported() {
        let text = "## [Unreleased]\n\n### Fixed\n\n### Added\n";
        assert_eq!(
            subsection_violations(text),
            vec!["CHANGELOG.md: `## [Unreleased]` has `### Added` after `### Fixed`".to_owned()],
        );
    }

    #[test]
    fn the_release_notes_end_with_the_legal_notice() {
        let notes = with_notice("### Added\n\n- Search.");
        assert!(
            notes.starts_with("### Added\n\n- Search.\n\n---\n\n"),
            "{notes}"
        );
        for needle in [
            "unofficial Star Citizen fan project",
            "not affiliated with the Cloud Imperium group of companies",
            "ships no game data",
            "Terms of Service",
            "Star Citizen®, Roberts Space Industries® and Cloud Imperium® are registered trademarks of Cloud Imperium Rights LLC.",
        ] {
            assert!(notes.contains(needle), "{needle}");
        }
    }

    #[test]
    fn a_missing_or_empty_section_has_no_notes() {
        assert_eq!(section(CHANGELOG, "0.3.0"), None, "missing");
        assert_eq!(section(CHANGELOG, "0.2"), None, "no prefix match");
        assert_eq!(
            section("## [0.4.0]\n\n## [0.3.0]\n- x\n", "0.4.0"),
            None,
            "empty"
        );
    }
}
