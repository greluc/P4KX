// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};

const HEADER: [&str; 2] = [
    "// SPDX-License-Identifier: MIT OR Apache-2.0",
    "// SPDX-FileCopyrightText: 2026 the P4KX contributors",
];

/// Lines the header occupies, including the blank line after it.
pub(crate) const HEADER_LINES: usize = 3;

/// Returns one message per tracked Rust source file that does not start with the SPDX header.
pub(crate) fn check(root: &Path) -> Result<Vec<String>> {
    let mut violations = Vec::new();
    for relative in tracked_rust_files(root)? {
        let path = root.join(&relative);
        let text =
            fs::read_to_string(&path).with_context(|| format!("cannot read {}", path.display()))?;
        if !has_header(&text) {
            violations.push(format!("missing SPDX header: {relative}"));
        }
    }
    Ok(violations)
}

/// Whether `text` starts with the two header lines followed by a blank line.
pub(crate) fn has_header(text: &str) -> bool {
    let mut lines = text
        .split('\n')
        .map(|line| line.strip_suffix('\r').unwrap_or(line));
    lines.next() == Some(HEADER[0]) && lines.next() == Some(HEADER[1]) && lines.next() == Some("")
}

fn tracked_rust_files(root: &Path) -> Result<Vec<String>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["ls-files", "-z", "--", "*.rs"])
        .output()
        .context("cannot run git ls-files")?;
    if !output.status.success() {
        bail!("git ls-files failed with {}", output.status);
    }
    let listing = String::from_utf8(output.stdout).context("git ls-files is not UTF-8")?;
    Ok(listing
        .split('\0')
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::has_header;

    #[test]
    fn accepts_the_header_before_module_docs() {
        let text = format!(
            "{}\n{}\n\n//! Docs.\n\nfn main() {{}}\n",
            super::HEADER[0],
            super::HEADER[1]
        );
        assert!(has_header(&text));
    }

    #[test]
    fn accepts_windows_line_endings() {
        let text = format!(
            "{}\r\n{}\r\n\r\nfn main() {{}}\r\n",
            super::HEADER[0],
            super::HEADER[1]
        );
        assert!(has_header(&text));
    }

    #[test]
    fn rejects_a_file_without_the_header() {
        assert!(!has_header("//! Docs.\n\nfn main() {}\n"));
        assert!(!has_header(""));
    }

    #[test]
    fn rejects_a_header_without_the_blank_line_or_with_other_text() {
        let no_blank = format!(
            "{}\n{}\nfn main() {{}}\n",
            super::HEADER[0],
            super::HEADER[1]
        );
        assert!(!has_header(&no_blank));
        let wrong = "// SPDX-License-Identifier: MIT\n// SPDX-FileCopyrightText: 2026 the P4KX contributors\n\n";
        assert!(!has_header(wrong));
    }
}
