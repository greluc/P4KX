use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};

const FILE: &str = "THIRD-PARTY-NOTICES";
const MARKER: &str = "== GENERATED BELOW THIS LINE ==\n";

/// Returns a message when the generated part of `THIRD-PARTY-NOTICES` is not the current output
/// of `cargo about`.
///
/// # Errors
///
/// When the file cannot be read, has no marker line, or `cargo about` fails.
pub(crate) fn check(root: &Path) -> Result<Vec<String>> {
    let text = read(root)?;
    let generated = generate(root)?;
    Ok(if generated_part(&text)? == generated {
        Vec::new()
    } else {
        vec![format!(
            "{FILE} is out of date; run `cargo xtask notices --write` and commit the result"
        )]
    })
}

/// Rewrites the generated part of `THIRD-PARTY-NOTICES` from `cargo about`.
///
/// # Errors
///
/// When the file cannot be read or written, has no marker line, or `cargo about` fails.
pub(crate) fn write(root: &Path) -> Result<()> {
    let text = read(root)?;
    let generated = generate(root)?;
    let (head, _) = split(&text)?;
    fs::write(root.join(FILE), format!("{head}{MARKER}{generated}"))
        .with_context(|| format!("cannot write {FILE}"))
}

fn read(root: &Path) -> Result<String> {
    let text =
        fs::read_to_string(root.join(FILE)).with_context(|| format!("cannot read {FILE}"))?;
    Ok(text.replace("\r\n", "\n"))
}

fn split(text: &str) -> Result<(&str, &str)> {
    text.split_once(MARKER)
        .with_context(|| format!("{FILE} has no line `{}`", MARKER.trim_end()))
}

fn generated_part(text: &str) -> Result<&str> {
    split(text).map(|(_, generated)| generated)
}

fn generate(root: &Path) -> Result<String> {
    let target = std::env::temp_dir().join(format!("p4kx-notices-{}.txt", std::process::id()));
    let output = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned()))
        .args([
            "about",
            "generate",
            "--workspace",
            "--locked",
            "--fail",
            "-o",
        ])
        .arg(&target)
        .arg("about.hbs")
        .current_dir(root)
        .output()
        .context("cannot run cargo about; install it with `cargo install cargo-about --locked`")?;
    if !output.status.success() {
        bail!(
            "cargo about failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let generated = fs::read_to_string(&target).context("cannot read the cargo about output");
    let _ignored = fs::remove_file(&target);
    Ok(generated?.replace("\r\n", "\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_part_follows_the_marker() {
        let text = format!("head\n{MARKER}body\n");
        assert_eq!(
            generated_part(&text).ok(),
            Some("body\n"),
            "the text after the marker is the generated part"
        );
    }

    #[test]
    fn missing_marker_is_an_error() {
        assert!(
            generated_part("head\n").is_err(),
            "a file without the marker is rejected"
        );
    }
}
