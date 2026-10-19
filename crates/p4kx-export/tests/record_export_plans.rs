//! Record exports that reuse the planned files of a DataForge and report their progress.

mod common;

use std::error::Error;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

use common::database;
use p4kx_export::{
    RecordExportOptions, RecordFilePlans, RecordForm, write_planned_record_export,
    write_record_export,
};
use p4kx_extraction::DataForge;
use p4kx_testkit::TestRecord;

type TestResult = Result<(), Box<dyn Error>>;
type Listing = Vec<(String, Vec<u8>)>;

fn listing(root: &Path) -> Result<Listing, Box<dyn Error>> {
    let mut files = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for item in fs::read_dir(dir)? {
            let path = item?.path();
            if path.is_dir() {
                pending.push(path);
            } else {
                let name = path
                    .strip_prefix(root)?
                    .to_string_lossy()
                    .replace('\\', "/");
                files.push((name, fs::read(&path)?));
            }
        }
    }
    files.sort();
    Ok(files)
}

#[test]
fn planned_exports_write_the_same_files_as_a_plain_export() -> TestResult {
    let bytes = database().build();
    let forge = DataForge::open(&bytes)?;
    let records: Vec<_> = forge.records().collect();
    let plans = RecordFilePlans::default();
    for form in RecordForm::ALL {
        let plain = tempfile::tempdir()?;
        let first = tempfile::tempdir()?;
        let second = tempfile::tempdir()?;
        let options = RecordExportOptions::default();

        let expected = write_record_export(&forge, &records, form, plain.path(), &options);
        let once =
            write_planned_record_export(&forge, &plans, &records, form, first.path(), &options);
        let again =
            write_planned_record_export(&forge, &plans, &records, form, second.path(), &options);

        assert_eq!(once.written(), expected.written(), "{form}: written");
        assert_eq!(again.written(), expected.written(), "{form}: written again");
        assert_eq!(listing(first.path())?, listing(plain.path())?, "{form}");
        assert_eq!(
            listing(second.path())?,
            listing(plain.path())?,
            "{form} again"
        );
    }
    Ok(())
}

#[test]
fn plans_of_another_dataforge_are_planned_again() -> TestResult {
    let small = database().build();
    let larger = database()
        .record(TestRecord::new(
            0x42,
            "Wheel",
            "Wheel.Extra",
            "libs/foundry/records/wheels/extra.xml",
        ))
        .build();
    let (small, larger) = (DataForge::open(&small)?, DataForge::open(&larger)?);
    let plans = RecordFilePlans::default();
    let warm = tempfile::tempdir()?;
    let small_records: Vec<_> = small.records().collect();
    let _warmed = write_planned_record_export(
        &small,
        &plans,
        &small_records,
        RecordForm::Json,
        warm.path(),
        &RecordExportOptions::default(),
    );
    let records: Vec<_> = larger.records().collect();
    let plain = tempfile::tempdir()?;
    let planned = tempfile::tempdir()?;

    let options = RecordExportOptions::default();
    let _plain = write_record_export(&larger, &records, RecordForm::Json, plain.path(), &options);
    let summary = write_planned_record_export(
        &larger,
        &plans,
        &records,
        RecordForm::Json,
        planned.path(),
        &options,
    );

    assert_eq!(summary.written(), 4, "every readable Record");
    assert_eq!(
        listing(planned.path())?,
        listing(plain.path())?,
        "the right files"
    );
    Ok(())
}

#[test]
fn every_form_reports_each_record_once_as_it_ends() -> TestResult {
    let bytes = database().build();
    let forge = DataForge::open(&bytes)?;
    let records: Vec<_> = forge.records().collect();
    for form in RecordForm::ALL {
        let dir = tempfile::tempdir()?;
        let ended = AtomicUsize::new(0);
        let progress = || {
            ended.fetch_add(1, Ordering::Relaxed);
        };
        let options = RecordExportOptions {
            progress: Some(&progress),
            ..RecordExportOptions::default()
        };

        let _summary = write_record_export(&forge, &records, form, dir.path(), &options);

        assert_eq!(
            ended.load(Ordering::Relaxed),
            records.len(),
            "{form}: written and failed Records"
        );
    }
    Ok(())
}

#[test]
fn plans_of_a_dataforge_with_as_many_records_elsewhere_are_planned_again() -> TestResult {
    let at = |path: &str| {
        p4kx_testkit::DataForgeBuilder::new()
            .structure(p4kx_testkit::TestStruct::new("Wheel"))
            .record(TestRecord::new(0x42, "Wheel", "Wheel.One", path))
            .build()
    };
    let (first, second) = (at("libs/a/one.xml"), at("libs/second/folder/one.xml"));
    let (first, second) = (DataForge::open(&first)?, DataForge::open(&second)?);
    let plans = RecordFilePlans::default();
    let options = RecordExportOptions::default();
    let warm = tempfile::tempdir()?;
    let first_records: Vec<_> = first.records().collect();
    let _warmed = write_planned_record_export(
        &first,
        &plans,
        &first_records,
        RecordForm::Json,
        warm.path(),
        &options,
    );
    let records: Vec<_> = second.records().collect();
    let planned = tempfile::tempdir()?;

    let summary = write_planned_record_export(
        &second,
        &plans,
        &records,
        RecordForm::Json,
        planned.path(),
        &options,
    );

    assert_eq!(summary.written(), 1, "the Record");
    let files: Vec<String> = listing(planned.path())?
        .into_iter()
        .map(|(name, _)| name)
        .collect();
    assert_eq!(
        files,
        ["libs/second/folder/one.json"],
        "its own record path"
    );
    Ok(())
}
