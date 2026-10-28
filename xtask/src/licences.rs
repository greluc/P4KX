// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Checks that `LICENSE-MIT.md` and `LICENSE-APACHE.md` carry the same words as the plain
//! `LICENSE-MIT` and `LICENSE-APACHE`.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

const LICENCES: [&str; 2] = ["LICENSE-MIT", "LICENSE-APACHE"];

/// Returns one message per Markdown licence whose wording differs from its plain file.
///
/// # Errors
///
/// When a licence file cannot be read.
pub(crate) fn check(root: &Path) -> Result<Vec<String>> {
    let mut violations = Vec::new();
    for name in LICENCES {
        let plain = read(root, name)?;
        let markdown = read(root, &format!("{name}.md"))?;
        if !same_words(&plain, &markdown) {
            violations.push(format!(
                "{name}.md differs in wording from {name}; Markdown may add only heading marks and layout"
            ));
        }
    }
    Ok(violations)
}

fn read(root: &Path, name: &str) -> Result<String> {
    fs::read_to_string(root.join(name)).with_context(|| format!("cannot read {name}"))
}

/// Whether both texts hold the same whitespace-separated words, ignoring the `#` heading marks
/// of Markdown.
fn same_words(plain: &str, markdown: &str) -> bool {
    words(plain).eq(words(markdown))
}

fn words(text: &str) -> impl Iterator<Item = &str> {
    text.split_whitespace()
        .filter(|word| !word.bytes().all(|byte| byte == b'#'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_and_heading_marks_are_ignored() {
        assert!(same_words(
            "MIT License\n\n   Copyright (c) 2026\n  the P4KX\n",
            "# MIT License\n\nCopyright (c) 2026 the P4KX\n"
        ));
    }

    #[test]
    fn changed_word_is_detected() {
        assert!(!same_words("Copyright (c) 2026", "Copyright (c) 2025"));
    }

    #[test]
    fn missing_or_extra_word_is_detected() {
        assert!(!same_words("a b c", "a b"));
        assert!(!same_words("a b", "a b c"));
    }

    #[test]
    fn list_numbering_counts_as_wording() {
        assert!(same_words("1. Definitions.", "1. Definitions."));
        assert!(!same_words("1. Definitions.", "Definitions."));
    }

    #[test]
    fn real_licences_match() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("xtask lives in the workspace");
        assert_eq!(
            check(root).expect("licence files exist"),
            Vec::<String>::new()
        );
    }
}
