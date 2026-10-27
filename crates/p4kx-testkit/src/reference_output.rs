// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::path::PathBuf;

/// The environment variable that names a folder extracted by unp4k.
pub const REFERENCE_UNP4K_VAR: &str = "P4KX_REF_UNP4K";

/// The environment variable that names a folder of `CryXml` converted to XML by unforge.
pub const REFERENCE_UNFORGE_VAR: &str = "P4KX_REF_UNFORGE";

/// The environment variable that names a folder of `DataForge` JSON written by `StarBreaker`.
pub const REFERENCE_STARBREAKER_VAR: &str = "P4KX_REF_STARBREAKER";

/// Returns the folder of unp4k output named by `P4KX_REF_UNP4K`, `None` when the variable is
/// unset or empty, so that the differential test returns early.
#[must_use]
pub fn reference_unp4k_path() -> Option<PathBuf> {
    path_of(REFERENCE_UNP4K_VAR)
}

/// Returns the folder of unforge output named by `P4KX_REF_UNFORGE`, `None` when the variable is
/// unset or empty.
#[must_use]
pub fn reference_unforge_path() -> Option<PathBuf> {
    path_of(REFERENCE_UNFORGE_VAR)
}

/// Returns the folder of `StarBreaker` output named by `P4KX_REF_STARBREAKER`, `None` when the
/// variable is unset or empty.
#[must_use]
pub fn reference_starbreaker_path() -> Option<PathBuf> {
    path_of(REFERENCE_STARBREAKER_VAR)
}

fn path_of(variable: &str) -> Option<PathBuf> {
    std::env::var_os(variable)
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
}
