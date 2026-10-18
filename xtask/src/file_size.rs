use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

const MAX_LINES: usize = 400;
const SOURCE_DIRS: [&str; 2] = ["crates", "xtask"];

/// Returns one message per Rust source file longer than the line limit.
pub(crate) fn check(root: &Path) -> Result<Vec<String>> {
    let mut files = Vec::new();
    for dir in SOURCE_DIRS {
        collect_rust_files(&root.join(dir), &mut files)?;
    }
    let mut violations = Vec::new();
    for file in files {
        let text =
            fs::read_to_string(&file).with_context(|| format!("cannot read {}", file.display()))?;
        let lines = text.lines().count();
        if lines > MAX_LINES {
            let shown = file.strip_prefix(root).unwrap_or(&file);
            violations.push(format!(
                "file too long: {} has {lines} lines, the limit is {MAX_LINES}",
                shown.display()
            ));
        }
    }
    Ok(violations)
}

/// Adds every Rust source file below `dir` to `files`, skipping `target` folders.
pub(crate) fn collect_rust_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(dir).with_context(|| format!("cannot list {}", dir.display()))? {
        let path = entry?.path();
        if path.is_dir() {
            if path.file_name().is_some_and(|name| name != "target") {
                collect_rust_files(&path, files)?;
            }
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
    Ok(())
}
