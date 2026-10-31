// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The Conflict policy for the files of a converted folder, `if-changed` comparing the
//! CRC-32C of the output with the file that would be written.

use std::error::Error;
use std::fs;
use std::path::Path;

use p4kx_extraction::{
    ConflictPolicy, ExtractOptions, ExtractionReport, FolderSource, convert_folder,
};
use p4kx_testkit::{CryXmlByteOrder, CryXmlSignature, TestElement};

type TestResult = Result<(), Box<dyn Error>>;

fn convert(
    input: &Path,
    out: &Path,
    policy: ConflictPolicy,
) -> Result<ExtractionReport, Box<dyn Error>> {
    let source = FolderSource::scan(input, None)?;
    let options = ExtractOptions {
        policy,
        ..ExtractOptions::default()
    };
    Ok(convert_folder(&source, None, true, out, &options)?)
}

#[test]
fn if_changed_keeps_the_files_that_match_and_writes_the_one_that_differs() -> TestResult {
    let dir = tempfile::tempdir()?;
    let input = dir.path().join("raw");
    fs::create_dir_all(&input)?;
    let ship = TestElement::new("Ship")
        .attribute("name", "Gladius")
        .to_cryxml(CryXmlSignature::CryXmlB, CryXmlByteOrder::Little);
    fs::write(input.join("ship.xml"), ship)?;
    fs::write(input.join("plain.bin"), b"plain data")?;
    let out = dir.path().join("out");
    let first = convert(&input, &out, ConflictPolicy::IfChanged)?;
    assert_eq!(first.extracted(), 2, "both written: {first:?}");
    fs::write(out.join("plain.bin"), b"PLAIN DATA")?;

    let second = convert(&input, &out, ConflictPolicy::IfChanged)?;

    assert_eq!(second.skipped(), ["ship.xml"], "the converted file matches");
    assert_eq!(second.extracted(), 1, "only the changed file is written");
    assert_eq!(fs::read(out.join("plain.bin"))?, b"plain data", "restored");
    Ok(())
}
