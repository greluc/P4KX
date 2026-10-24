// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The CHF files of a real Archive round-trip through JSON byte-exactly.

use std::error::Error;
use std::io::Read;

use p4kx_assembly::character::Chf;
use p4kx_extraction::Archive;
use p4kx_testkit::real_archive_path;

type TestResult = Result<(), Box<dyn Error>>;

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn every_chf_of_the_archive_round_trips_through_json_byte_exactly() -> TestResult {
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;
    let mut checked = 0_usize;
    let mut failures = Vec::new();
    for entry in archive.entries() {
        if !entry.path().to_ascii_lowercase().ends_with(".chf") {
            continue;
        }
        let mut bytes = Vec::new();
        archive.open_entry(entry)?.read_to_end(&mut bytes)?;
        let written = Chf::read(&bytes)
            .and_then(|chf| chf.to_json())
            .and_then(|json| Chf::from_json(json.as_bytes()))
            .and_then(|chf| chf.write());
        match written {
            Ok(written) if written == bytes => checked += 1,
            Ok(_) => failures.push(format!("{}: differs", entry.path())),
            Err(error) => failures.push(format!("{}: {error}", entry.path())),
        }
    }
    assert!(checked > 0, "the Archive holds CHF files");
    assert!(
        failures.is_empty(),
        "{checked} round-tripped, failures: {failures:#?}"
    );
    Ok(())
}
