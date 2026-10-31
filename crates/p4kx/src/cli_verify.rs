// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::path::PathBuf;

use clap::Args;

/// Arguments of `p4kx verify`.
#[derive(Debug, Args)]
pub(crate) struct VerifyArgs {
    /// Path of the Data.p4k Archive.
    pub(crate) archive: PathBuf,
    /// Checks only the Entries the Filter matches, with the rules of `p4kx search`, instead of
    /// every Entry.
    #[arg(long, value_name = "FILTER")]
    pub(crate) filter: Option<String>,
    /// Leaves out every Entry this Filter matches, with the rules of the Filter; repeatable. The
    /// parts of a texture whose header it matches are left out too. A selection it leaves empty
    /// is invalid input.
    #[arg(long, value_name = "FILTER")]
    pub(crate) exclude: Vec<String>,
}
