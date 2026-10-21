use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};

const LOG_FILES: [&str; 2] = ["log.old.log", "log.log"];
const MAX_LOG_BYTES: u64 = 64 << 20;
const LOGS_OVERRIDE: &str = "P4KX_LAUNCHER_LOGS";
const LAUNCHER_FOLDER: &str = "rsilauncher";
const LAUNCHER_LOGS: &str = "logs";
const MARKER: &str = "(SC ";

/// Returns the folder of the RSI Launcher's logs: `P4KX_LAUNCHER_LOGS` when set, otherwise
/// `logs` in the launcher's application-data folder.
pub(crate) fn standard_log_folder() -> Option<PathBuf> {
    std::env::var_os(LOGS_OVERRIDE)
        .map(PathBuf::from)
        .or_else(|| dirs::config_dir().map(|dir| dir.join(LAUNCHER_FOLDER).join(LAUNCHER_LOGS)))
}

/// Returns the public patch the launcher logged for the Build with `change_number`, read from
/// the two log files in `log_folder`, the newest line winning.
pub(crate) fn find(log_folder: &Path, change_number: &str) -> Option<String> {
    if change_number.is_empty() {
        return None;
    }
    let mut found = None;
    for name in LOG_FILES {
        let Ok(file) = File::open(log_folder.join(name)) else {
            continue;
        };
        let mut reader = BufReader::new(file.take(MAX_LOG_BYTES));
        let mut line = Vec::new();
        loop {
            line.clear();
            match reader.read_until(b'\n', &mut line) {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    if let Some(patch) = patch_in(&String::from_utf8_lossy(&line), change_number) {
                        found = Some(patch.to_owned());
                    }
                }
            }
        }
    }
    found
}

/// Returns the version of the first `(SC <CHANNEL> <version>-<tag>.<change>)` of `line` whose
/// change is `change_number`, e.g. `4.7.2` for `(SC LIVE 4.7.2-live.11674325)`.
fn patch_in<'a>(line: &'a str, change_number: &str) -> Option<&'a str> {
    if !line.contains(change_number) {
        return None;
    }
    let mut rest = line;
    while let Some(start) = rest.find(MARKER) {
        rest = &rest[start + MARKER.len()..];
        let Some(end) = rest.find(')') else { break };
        if let Some(patch) = patch_of(&rest[..end], change_number) {
            return Some(patch);
        }
    }
    None
}

fn patch_of<'a>(label: &'a str, change_number: &str) -> Option<&'a str> {
    let (_channel, build) = label.split_once(' ')?;
    let (version, tail) = build.split_once('-')?;
    let (_tag, change) = tail.rsplit_once('.')?;
    let numeric = version.starts_with(|c: char| c.is_ascii_digit())
        && version.chars().all(|c| c.is_ascii_digit() || c == '.')
        && !version.ends_with('.');
    (change == change_number && numeric).then_some(version)
}
