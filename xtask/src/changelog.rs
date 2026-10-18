use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};

const FILE: &str = "CHANGELOG.md";

/// Returns a message when `CHANGELOG.md` has no notes for the workspace version.
pub(crate) fn check(root: &Path) -> Result<Vec<String>> {
    let version = workspace_version(root)?;
    let text = read(root)?;
    Ok(match section(&text, &version) {
        Some(_) => Vec::new(),
        None => vec![missing(&version)],
    })
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
    fs::write(output, format!("{notes}\n"))
        .with_context(|| format!("cannot write {}", output.display()))
}

fn missing(version: &str) -> String {
    format!("{FILE} has no notes under `## [{version}]`")
}

fn read(root: &Path) -> Result<String> {
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
    use super::section;

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
