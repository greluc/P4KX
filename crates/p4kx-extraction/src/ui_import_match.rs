use std::path::PathBuf;

use p4kx_archive::Archive;
use rayon::prelude::*;

const LANGUAGE_FOLDER: &str = "data\\localization\\english\\";
const UI_FOLDER: &str = "data\\ui\\";

pub(crate) fn folder_of(path: &str) -> &str {
    path.rfind(['\\', '/']).map_or("", |at| &path[..at])
}

fn normalised(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for part in path.split(['\\', '/']) {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            part => parts.push(part),
        }
    }
    parts.join("\\").to_ascii_lowercase()
}

fn swapped(name: &str) -> Option<String> {
    let (stem, extension) = name.rsplit_once('.')?;
    let other = match extension.to_ascii_lowercase().as_str() {
        "swf" => "gfx",
        "gfx" => "swf",
        _ => return None,
    };
    Some(format!("{stem}.{other}"))
}

/// Returns the URL as a relative path that stays inside the folder of the importing movie, or
/// `None` when it is empty, rooted, carries a drive or stream prefix, or climbs above that folder.
pub(super) fn contained(url: &str) -> Option<PathBuf> {
    let mut parts: Vec<&str> = Vec::new();
    for part in url.split(['\\', '/']) {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            part if part.contains([':', '\0']) => return None,
            part => parts.push(part),
        }
    }
    if parts.is_empty() || url.starts_with(['\\', '/']) {
        return None;
    }
    Some(parts.iter().collect())
}

/// Returns the indexes of up to `limit` Entries that a movie in `folder` may mean by the import
/// URL `url`, the best first: the exact path, the same tail below the UI folder or the English
/// localisation folder, the same file name there, then anywhere. A `.swf` and a `.gfx` of the
/// same name are alternatives, the one the URL names first.
pub(crate) fn candidates(archive: &Archive, url: &str, folder: &str, limit: usize) -> Vec<usize> {
    let relative = normalised(&format!("{folder}\\{url}"));
    let file = url.rsplit(['\\', '/']).next().unwrap_or(url);
    let other = swapped(file);
    let direct = normalised(url);
    let suffix = format!("\\{direct}");
    let mut found: Vec<(usize, String, usize)> = archive
        .entries()
        .par_iter()
        .enumerate()
        .filter_map(|(at, entry)| {
            let name = entry.path().rsplit(['\\', '/']).next()?;
            let twin = if name.eq_ignore_ascii_case(file) {
                0
            } else if other
                .as_deref()
                .is_some_and(|other| name.eq_ignore_ascii_case(other))
            {
                1
            } else {
                return None;
            };
            let path = normalised(entry.path());
            let suffixed = format!("\\{path}").ends_with(&suffix);
            let place = if path == relative || path == direct {
                0
            } else if suffixed && path.starts_with(UI_FOLDER) {
                1
            } else if suffixed && path.starts_with(LANGUAGE_FOLDER) {
                2
            } else if folder_of(&path) == LANGUAGE_FOLDER.trim_end_matches('\\') {
                3
            } else if path.starts_with(UI_FOLDER) {
                4
            } else {
                5
            };
            Some((twin + 2 * place, path, at))
        })
        .collect();
    found.sort_unstable_by(|left, right| (left.0, &left.1).cmp(&(right.0, &right.1)));
    found.into_iter().take(limit).map(|(_, _, at)| at).collect()
}
