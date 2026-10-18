use std::collections::HashMap;

use p4kx_archive::{Archive, Entry};
use thiserror::Error;

/// Requested Entry paths that name no Entry of the Archive.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("no Entry named {}", .0.join(", "))]
pub struct UnknownEntries(pub Vec<String>);

/// Returns the Entries named by `requested`, in Archive order.
///
/// Paths match with either `/` or `\` as separator and ignore ASCII case.
///
/// # Errors
///
/// Returns [`UnknownEntries`] with every requested path that names no Entry.
pub fn select_entries<'a>(
    archive: &'a Archive,
    requested: &[impl AsRef<str>],
) -> Result<Vec<&'a Entry>, UnknownEntries> {
    let mut wanted: HashMap<String, bool> = requested
        .iter()
        .map(|path| (match_key(path.as_ref()), false))
        .collect();
    let selected: Vec<&Entry> = archive
        .entries()
        .iter()
        .filter(|entry| {
            wanted
                .get_mut(&match_key(entry.path()))
                .map(|found| *found = true)
                .is_some()
        })
        .collect();
    let unknown: Vec<String> = requested
        .iter()
        .map(AsRef::as_ref)
        .filter(|path| wanted.get(&match_key(path)) == Some(&false))
        .map(str::to_owned)
        .collect();
    if unknown.is_empty() {
        Ok(selected)
    } else {
        Err(UnknownEntries(unknown))
    }
}

fn match_key(path: &str) -> String {
    path.replace('/', "\\").to_ascii_lowercase()
}
