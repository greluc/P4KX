// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The legal notice of `p4kx --help` and `p4kx --version`.

macro_rules! legal_notice {
    () => {
        "Unofficial Star Citizen fan project, not affiliated with or endorsed by the Cloud \
         Imperium group of companies. Ships no game data and reads only your own installation. \
         You are responsible for complying with the Star Citizen EULA and Terms of Service."
    };
}

/// Shown above the usage of `--help`.
pub(crate) const LEGAL_NOTICE: &str = legal_notice!();

/// Shown by `--version` and `-V`: the version, then the legal notice.
pub(crate) const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "\n", legal_notice!());
