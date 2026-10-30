// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The Build of an SCMDB dump as its file name states it, `<name>-<version>.json` with the
//! version `<game version>-<channel>.<change number>`, e.g. `4.10.2-live.12913470`.

use std::io;
use std::path::Path;

/// The dump of factions, standings and contracts.
pub(crate) const MERGED: &str = "merged";

/// The dump of crafting blueprints.
pub(crate) const BLUEPRINTS: &str = "crafting_blueprints";

/// The dump of mineable elements, compositions and cluster presets.
pub(crate) const MINING: &str = "mining_data";

const EXTENSION: &str = ".json";

/// Returns the file name of the dump `name` of `version`.
pub(crate) fn dump_file_name(name: &str, version: &str) -> String {
    format!("{name}-{version}{EXTENSION}")
}

/// Returns the version of the `merged` dump in `folder` that was made from the Build with
/// `branch` and `change_number`, `None` when the folder holds none.
///
/// # Errors
///
/// Returns the I/O error of reading the folder.
pub(crate) fn find_dump_version(
    folder: &Path,
    branch: &str,
    change_number: &str,
) -> io::Result<Option<String>> {
    let prefix = format!("{MERGED}-");
    for entry in std::fs::read_dir(folder)? {
        let name = entry?.file_name();
        let version = name
            .to_str()
            .and_then(|name| name.strip_prefix(&prefix))
            .and_then(|name| name.strip_suffix(EXTENSION));
        if let Some(version) = version
            && matches_build(version, branch, change_number)
        {
            return Ok(Some(version.to_owned()));
        }
    }
    Ok(None)
}

/// Returns whether the dump `version` names the change number and the game version a Build
/// manifest states; the branch ends in the game version, e.g. `sc-alpha-4.10.2`.
pub(crate) fn matches_build(version: &str, branch: &str, change_number: &str) -> bool {
    let Some((game_and_channel, change)) = version.rsplit_once('.') else {
        return false;
    };
    let Some((game, _channel)) = game_and_channel.rsplit_once('-') else {
        return false;
    };
    change == change_number
        && branch
            .rsplit_once('-')
            .is_some_and(|(_, branch_game)| branch_game == game)
}
