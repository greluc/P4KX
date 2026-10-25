// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The legal notice of `p4kx-mcp --help` and `p4kx-mcp --version`.

macro_rules! legal_notice {
    () => {
        "Unofficial Star Citizen fan project, not affiliated with or endorsed by the Cloud \
         Imperium group of companies. Ships no game data and reads only your own installation. \
         You are responsible for complying with the Star Citizen EULA and Terms of Service."
    };
}

/// Shown above the usage of `--help`: the legal notice, then what the tools return.
pub(crate) const BEFORE_HELP: &str = concat!(
    legal_notice!(),
    " Content returned to the MCP client may be forwarded by it to third parties; game texts \
     and media must not be redistributed."
);

/// Shown by `--version` and `-V`: the version, then the legal notice.
pub(crate) const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "\n", legal_notice!());
