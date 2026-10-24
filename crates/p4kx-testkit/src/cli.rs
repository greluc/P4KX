// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use assert_cmd::Command;
use assert_cmd::cargo::CargoError;

use crate::test_key::test_key;

/// Returns a command that runs the `p4kx` binary with [`test_key`] as its Archive key, so that
/// the encrypted Entries of synthetic Archives read.
///
/// The look for a running game is switched off (`P4KX_GAME_CHECK=off`), so that a game on the
/// developer's machine changes no output. The RSI Launcher log folder (`P4KX_LAUNCHER_LOGS`) is a
/// folder that does not exist, so that no public patch comes from the developer's own launcher
/// log; a test that needs log lines sets the variable again.
///
/// # Errors
///
/// Returns a [`CargoError`] when the binary cannot be found.
pub fn p4kx() -> Result<Command, CargoError> {
    let mut command = Command::cargo_bin("p4kx")?;
    command.env("P4KX_ARCHIVE_KEY", test_key().to_hex());
    command.env("P4KX_GAME_CHECK", "off");
    command.env(
        "P4KX_LAUNCHER_LOGS",
        std::env::temp_dir().join("p4kx-test-no-launcher-logs"),
    );
    Ok(command)
}
