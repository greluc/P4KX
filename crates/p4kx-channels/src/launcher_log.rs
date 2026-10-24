// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::path::{Path, PathBuf};

/// The only files read from the RSI Launcher's log folder.
const LOG_FILES: [&str; 2] = ["log.log", "log.old.log"];

const LAUNCH: &str = "Launching Star Citizen ";
const LAUNCH_FROM: &str = " from (";
const LAUNCH_END: &str = ")\"";
const INSTALLERS: [&str; 2] = ["Installing Star Citizen ", "Verifying Star Citizen "];
const INSTALL_AT: &str = " at ";
const INSTALL_END: &str = " (force DP";

/// Returns the library folders the RSI Launcher log in `log_folder` names, without duplicates.
///
/// Only the files in [`LOG_FILES`] are read; a missing or unreadable file is skipped.
#[must_use]
pub(crate) fn library_folders(log_folder: &Path) -> Vec<PathBuf> {
    let mut folders: Vec<PathBuf> = Vec::new();
    for name in LOG_FILES {
        let Ok(bytes) = std::fs::read(log_folder.join(name)) else {
            continue;
        };
        for line in String::from_utf8_lossy(&bytes).lines() {
            if let Some(folder) = library_folder(line)
                && !folders.contains(&folder)
            {
                folders.push(folder);
            }
        }
    }
    folders
}

fn library_folder(line: &str) -> Option<PathBuf> {
    if let Some(channel_folder) = launched_channel_folder(line) {
        return channel_folder.parent().map(Path::to_path_buf);
    }
    INSTALLERS.iter().find_map(|marker| {
        let rest = after(line, marker)?;
        let rest = after(rest, INSTALL_AT)?;
        let end = rest.find(INSTALL_END)?;
        unescape(rest.get(..end)?).map(PathBuf::from)
    })
}

fn launched_channel_folder(line: &str) -> Option<PathBuf> {
    let rest = after(line, LAUNCH)?;
    let rest = after(rest, LAUNCH_FROM)?;
    let end = rest.find(LAUNCH_END)?;
    unescape(rest.get(..end)?).map(PathBuf::from)
}

fn after<'a>(text: &'a str, marker: &str) -> Option<&'a str> {
    let start = text.find(marker)? + marker.len();
    text.get(start..)
}

fn unescape(json_fragment: &str) -> Option<String> {
    let path: String = serde_json::from_str(&format!("\"{json_fragment}\"")).ok()?;
    (!path.is_empty()).then_some(path)
}
