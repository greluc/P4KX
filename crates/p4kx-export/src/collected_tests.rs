// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::Guid;

use crate::CatalogFailure;
use crate::collected::Picked;

#[test]
fn a_ship_whose_base_data_cannot_be_read_becomes_a_failure() {
    let failure = CatalogFailure {
        guid: Guid::from_stored([1; 16]),
        record_path: "libs/foundry/records/entities/spaceships/x".to_owned(),
        reason: "not readable".to_owned(),
    };
    let picked = Picked::of_ship(Err(failure.clone()), |_| std::process::abort());
    assert!(matches!(picked, Picked::Failed(found) if found == failure));
}
