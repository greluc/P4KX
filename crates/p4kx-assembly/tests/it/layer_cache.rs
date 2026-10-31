// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The layer files shared by the libraries an [`ArchiveModels`] reads.

use p4kx_assembly::{ArchiveModels, ModelProvider};
use p4kx_extraction::Archive;
use p4kx_testkit::layered_entries;

use crate::assembly_support::{TestResult, fixture};

const MODEL: &str = "Data\\Objects\\Test\\layered_ship.cgf";
const LIBRARY: &str = "Objects/Test/layered";

#[test]
fn the_libraries_of_one_provider_share_one_cache_of_layer_files() -> TestResult {
    let fixture = fixture(layered_entries())?;
    let archive = Archive::open(&fixture.path)?;
    let models = ArchiveModels::new(&archive);
    assert!(models.layer_cache().is_empty(), "nothing is read yet");

    let first = models.library(MODEL, LIBRARY);
    let kept = models.layer_cache().len();
    let second = models.library(MODEL, LIBRARY);

    assert_eq!(kept, 4, "base, wear, the lost one and the loop are kept");
    assert_eq!(
        models.layer_cache().len(),
        kept,
        "the second load adds none"
    );
    assert_eq!(first, second, "and reads the same surfaces");
    Ok(())
}
