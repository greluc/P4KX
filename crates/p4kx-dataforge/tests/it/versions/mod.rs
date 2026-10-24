// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Runs the same checks over a synthetic DataForge in the layouts of versions 6 and 8.

use std::error::Error;

use p4kx_dataforge::{DataForge, RecordTree};
use p4kx_testkit::DataForgeBuilder;

/// The result of a test.
pub(crate) type TestResult = Result<(), Box<dyn Error>>;

/// Builds `builder` as version 6 and as version 8 and runs `check` on the tree of the first
/// Record of each.
pub(crate) fn in_both_versions(
    builder: &DataForgeBuilder,
    check: impl Fn(&RecordTree<'_>) -> TestResult,
) -> TestResult {
    for version in [6, 8] {
        let bytes = builder.clone().version(version).build();
        let forge = DataForge::open(&bytes)?;
        let tree = forge.records().next().ok_or("no Record")?.tree()?;
        check(&tree).map_err(|error| format!("version {version}: {error}"))?;
    }
    Ok(())
}
