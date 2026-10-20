/// The process names of the game, compared ignoring case.
const GAME_PROCESSES: [&str; 2] = ["StarCitizen.exe", "StarCitizen"];

/// Returns whether `name` is the name of the Star Citizen game process, ignoring case.
#[must_use]
pub fn is_game_process(name: &str) -> bool {
    GAME_PROCESSES
        .iter()
        .any(|game| name.eq_ignore_ascii_case(game))
}

/// Returns whether any of `names` is the game process.
pub fn any_game_process<I, S>(names: I) -> bool
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    names.into_iter().any(|name| is_game_process(name.as_ref()))
}

/// Returns whether the output of `qprocess` lists the game process.
///
/// The image name is the last column of every line; the header and messages are in the language
/// of the system and never end in the name of the game.
#[must_use]
pub fn qprocess_lists_game(output: &str) -> bool {
    any_game_process(
        output
            .lines()
            .filter_map(|line| line.split_whitespace().last()),
    )
}

/// Returns whether `StarCitizen.exe` of any Channel is running.
///
/// Only the list of process names is read: `qprocess` on Windows, the process names below `/proc`
/// elsewhere. No process is opened, attached to or inspected, so the game's memory and handles
/// are never touched. The check takes a few hundred milliseconds on Windows.
#[must_use]
pub fn game_is_running() -> bool {
    platform::game_is_running()
}

#[cfg(windows)]
mod platform {
    use std::os::windows::process::CommandExt;
    use std::path::PathBuf;
    use std::process::Command;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    pub(super) fn game_is_running() -> bool {
        let root = std::env::var_os("SystemRoot")
            .map_or_else(|| PathBuf::from(r"C:\Windows"), PathBuf::from);
        Command::new(root.join("System32").join("qprocess.exe"))
            .arg("StarCitizen.exe")
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .is_ok_and(|output| {
                super::qprocess_lists_game(&String::from_utf8_lossy(&output.stdout))
            })
    }
}

#[cfg(not(windows))]
mod platform {
    pub(super) fn game_is_running() -> bool {
        let Ok(processes) = std::fs::read_dir("/proc") else {
            return false;
        };
        processes.filter_map(Result::ok).any(|process| {
            std::fs::read_to_string(process.path().join("comm"))
                .is_ok_and(|name| super::is_game_process(name.trim_end()))
        })
    }
}
