//! Repository checks run by CI: `cargo xtask layers`, `cargo xtask lints`,
//! `cargo xtask file-size`, `cargo xtask changelog`, or all of them with `cargo xtask`.
//!
//! `cargo xtask release-notes <tag> <file>` writes the `CHANGELOG.md` notes of a Release tag.

mod changelog;
mod file_size;
mod layers;
mod lints;
mod metadata;

use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result, bail};

fn main() -> ExitCode {
    match run() {
        Ok(violations) if violations.is_empty() => ExitCode::SUCCESS,
        Ok(violations) => {
            report(&violations);
            ExitCode::FAILURE
        }
        Err(error) => {
            report(&[format!("error: {error:#}")]);
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<Vec<String>> {
    let root = workspace_root()?;
    let command = std::env::args().nth(1);
    match command.as_deref() {
        None => {
            let mut violations = layers::check(&root)?;
            violations.extend(lints::check(&root)?);
            violations.extend(file_size::check(&root)?);
            violations.extend(changelog::check(&root)?);
            Ok(violations)
        }
        Some("layers") => layers::check(&root),
        Some("lints") => lints::check(&root),
        Some("file-size") => file_size::check(&root),
        Some("changelog") => changelog::check(&root),
        Some("release-notes") => {
            let mut args = std::env::args().skip(2);
            let (Some(tag), Some(output)) = (args.next(), args.next()) else {
                bail!("usage: cargo xtask release-notes <tag> <file>");
            };
            changelog::write_notes(&root, &tag, Path::new(&output))?;
            Ok(Vec::new())
        }
        Some(other) => bail!(
            "unknown check `{other}`; use `layers`, `lints`, `file-size`, `changelog` or `release-notes`"
        ),
    }
}

fn workspace_root() -> Result<PathBuf> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .map(PathBuf::from)
        .context("xtask has no parent directory")
}

fn report(lines: &[String]) {
    let mut stderr = io::stderr().lock();
    for line in lines {
        let _ignored = writeln!(stderr, "{line}");
    }
}
