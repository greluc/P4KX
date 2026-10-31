// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Which DataForge file of a folder is converted: a path of the game first, then `.dcb` files,
//! then any file that starts like a DataForge; the others are named in the report.

use std::error::Error;
use std::fs;
use std::path::Path;

use p4kx_extraction::{
    ConflictPolicy, Exclusion, ExtractOptions, ExtractionReport, FolderSource, RecordExportReport,
    convert_folder,
};
use p4kx_testkit::{DataForgeBuilder, TestRecord, TestStruct, TestType, TestValue};

type TestResult = Result<(), Box<dyn Error>>;

fn database(name: &str) -> Vec<u8> {
    DataForgeBuilder::new()
        .structure(TestStruct::new("Item").field("count", TestType::Int32))
        .record(
            TestRecord::new(1, "Item", name, format!("libs/items/{name}.xml"))
                .value("count", TestValue::Int32(7)),
        )
        .build()
}

fn put(root: &Path, path: &str, bytes: &[u8]) -> TestResult {
    let file = root.join(path);
    if let Some(folder) = file.parent() {
        fs::create_dir_all(folder)?;
    }
    Ok(fs::write(file, bytes)?)
}

fn convert(input: &Path, out: &Path) -> Result<ExtractionReport, Box<dyn Error>> {
    let source = FolderSource::scan(input, None)?;
    let options = ExtractOptions {
        policy: ConflictPolicy::Overwrite,
        ..ExtractOptions::default()
    };
    let selection = source.select(None, &Exclusion::default());
    Ok(convert_folder(&source, &selection, false, out, &options)?)
}

#[test]
fn a_path_of_the_game_beats_other_dataforge_files_and_the_rest_are_named() -> TestResult {
    let dir = tempfile::tempdir()?;
    let input = dir.path().join("raw");
    put(&input, "Data/Game.dcb", &database("Old"))?;
    put(&input, "Data/Game2.dcb", &database("New"))?;
    put(&input, "a.dcb", &database("Other"))?;

    let report = convert(&input, &dir.path().join("out"))?;

    assert_eq!(report.records_entry(), Some("Data\\Game2.dcb"));
    assert_eq!(report.dataforge_left_out(), ["Data\\Game.dcb"]);
    assert_eq!(report.records().map(RecordExportReport::written), Some(1));
    Ok(())
}

#[test]
fn a_data_folder_converted_on_its_own_finds_the_game_paths_too() -> TestResult {
    let dir = tempfile::tempdir()?;
    let input = dir.path().join("Data");
    put(&input, "Game2.dcb", &database("New"))?;
    put(&input, "z.dcb", &database("Other"))?;

    let report = convert(&input, &dir.path().join("out"))?;

    assert_eq!(report.records_entry(), Some("Game2.dcb"));
    assert!(report.dataforge_left_out().is_empty(), "z.dcb is not named");
    Ok(())
}

#[test]
fn without_a_game_path_the_first_dcb_file_by_path_wins_and_the_others_are_named() -> TestResult {
    let dir = tempfile::tempdir()?;
    let input = dir.path().join("raw");
    put(&input, "x/b.dcb", &database("B"))?;
    put(&input, "x/a.dcb", &database("A"))?;
    put(
        &input,
        "x/not.dcb",
        b"not a database at all, only some text of a fair length ...",
    )?;

    let report = convert(&input, &dir.path().join("out"))?;

    assert_eq!(report.records_entry(), Some("x\\a.dcb"));
    assert_eq!(report.dataforge_left_out(), ["x\\b.dcb"]);
    Ok(())
}

#[test]
fn a_renamed_dataforge_is_found_by_its_content_when_no_dcb_file_exists() -> TestResult {
    let dir = tempfile::tempdir()?;
    let input = dir.path().join("raw");
    put(&input, "kept/database.bin", &database("Renamed"))?;
    put(&input, "kept/other.bin", &[7; 4096])?;

    let report = convert(&input, &dir.path().join("out"))?;

    assert_eq!(report.records_entry(), Some("kept\\database.bin"));
    assert_eq!(report.converted(), Some(1));
    assert_eq!(
        report.not_converted(),
        1,
        "the other file has no conversion"
    );
    Ok(())
}
