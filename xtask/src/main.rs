//! Repository checks run by CI: `cargo xtask layers`, `cargo xtask file-size`, or both with
//! `cargo xtask`.

mod file_size;
mod layers;

use std::io::{self, Write};
use std::path::PathBuf;
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
            violations.extend(file_size::check(&root)?);
            Ok(violations)
        }
        Some("layers") => layers::check(&root),
        Some("file-size") => file_size::check(&root),
        Some(other) => bail!("unknown check `{other}`; use `layers` or `file-size`"),
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
