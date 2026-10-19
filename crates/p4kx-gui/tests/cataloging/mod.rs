//! Helpers of the Basetool catalog tests: a synthetic Archive with manufacturers and their
//! texts, opened headless.

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use p4kx_export::Language as TextLanguage;
use p4kx_gui::{CatalogExport, CatalogExportStatus, Language, Model};
use p4kx_testkit::{
    ArchiveBuilder, DataForgeBuilder, GlobalIniBuilder, TestEntry, TestInstance, TestRecord,
    TestStruct, TestType, TestValue,
};
use tempfile::TempDir;

pub(crate) type TestResult = Result<(), Box<dyn Error>>;

fn manufacturer(number: u32, class: &str, description: &str) -> TestRecord {
    TestRecord::new(
        0x1000_0000_0000_4000_8000_0000_0000_0000 + u128::from(number),
        "SCItemManufacturer",
        format!("SCItemManufacturer.{class}"),
        format!("libs/foundry/records/scitemmanufacturer/{class}.xml"),
    )
    .value("Code", TestValue::text(class))
    .value(
        "Localization",
        TestValue::Struct(
            TestInstance::new("SCItemLocalization")
                .value(
                    "Name",
                    TestValue::text(format!("@manufacturer_Name{class}")),
                )
                .value("Description", TestValue::text(description)),
        ),
    )
}

fn structures() -> DataForgeBuilder {
    DataForgeBuilder::new()
        .version(8)
        .structure(
            TestStruct::new("SCItemLocalization")
                .field("Name", TestType::Locale)
                .field("Description", TestType::Locale),
        )
        .structure(
            TestStruct::new("SCItemManufacturer")
                .field("Code", TestType::String)
                .field(
                    "Localization",
                    TestType::Class("SCItemLocalization".to_owned()),
                ),
        )
}

/// Two manufacturers and a development leftover; the German file lacks the name of DRAK.
pub(crate) fn makers() -> ArchiveBuilder {
    let forge = structures()
        .record(manufacturer(1, "ACAS", "@manufacturer_Desc"))
        .record(manufacturer(2, "DRAK", "@manufacturer_Desc"))
        .record(manufacturer(3, "Test_Maker", "@manufacturer_Desc"));
    let english = GlobalIniBuilder::new()
        .text("manufacturer_NameACAS", "Ace Astrogation")
        .text("manufacturer_NameDRAK", "Drake Interplanetary")
        .text("manufacturer_Desc", "Ships.");
    let german = GlobalIniBuilder::new()
        .text("manufacturer_NameACAS", "Ace Astrogation")
        .text("manufacturer_Desc", "Schiffe.");
    archive(&forge, &english, &german)
}

/// `count` manufacturers, enough to keep a catalog busy for a while.
pub(crate) fn many(count: u32) -> ArchiveBuilder {
    let forge = (0..count).fold(structures(), |builder, number| {
        builder.record(manufacturer(
            number,
            &format!("M{number:05}"),
            "@manufacturer_Desc",
        ))
    });
    let texts = GlobalIniBuilder::new().text("manufacturer_Desc", "Ships.");
    archive(&forge, &texts, &texts)
}

/// `count` manufacturers sharing a description of `length` characters in both languages.
pub(crate) fn wordy(count: u32, length: usize) -> ArchiveBuilder {
    let forge = (0..count).fold(structures(), |builder, number| {
        builder.record(manufacturer(
            number,
            &format!("W{number:05}"),
            "@manufacturer_Desc",
        ))
    });
    let long = "x".repeat(length);
    let texts = GlobalIniBuilder::new().text("manufacturer_Desc", &long);
    archive(&forge, &texts, &texts)
}

fn archive(
    forge: &DataForgeBuilder,
    english: &GlobalIniBuilder,
    german: &GlobalIniBuilder,
) -> ArchiveBuilder {
    ArchiveBuilder::new()
        .entry(TestEntry::zstd("Data\\Game2.dcb", forge.build()))
        .entry(TestEntry::zstd(
            TextLanguage::English.global_ini_path(),
            english.build(),
        ))
        .entry(TestEntry::zstd(
            TextLanguage::German.global_ini_path(),
            german.build(),
        ))
}

/// Writes the Archive of `builder`, opens it and returns the folder `out` next to it, created
/// empty.
pub(crate) fn opened(
    builder: &ArchiveBuilder,
) -> Result<(TempDir, Model, PathBuf), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    builder.write_to(&path)?;
    let mut model = Model::new(Language::BritishEnglish);
    model.open(path, || {});
    wait_until(&mut model, |model| model.browser().is_some())?;
    let out = dir.path().join("out");
    fs::create_dir(&out)?;
    Ok((dir, model, out))
}

/// Polls `model` until `done` holds.
pub(crate) fn wait_until(model: &mut Model, done: impl Fn(&Model) -> bool) -> TestResult {
    let deadline = Instant::now() + Duration::from_secs(120);
    while !done(model) {
        if Instant::now() > deadline {
            return Err("the window did not settle in time".into());
        }
        std::thread::sleep(Duration::from_millis(5));
        model.poll();
        model.poll_catalog();
        model.poll_record_export();
        if let Some(browser) = model.browser_mut() {
            browser.poll();
        }
    }
    Ok(())
}

/// Waits until the Basetool catalog ended and returns where it stands.
pub(crate) fn ended(model: &mut Model) -> Result<CatalogExportStatus<'_>, Box<dyn Error>> {
    wait_until(model, |model| {
        model.catalog().is_some_and(|run| !run.is_running())
    })?;
    model
        .catalog()
        .map(CatalogExport::status)
        .ok_or_else(|| "no Basetool catalog".into())
}

/// Returns the names of the files in `dir`, sorted.
pub(crate) fn files_in(dir: &Path) -> Result<Vec<String>, Box<dyn Error>> {
    let mut names = fs::read_dir(dir)?
        .map(|item| item.map(|item| item.file_name().to_string_lossy().into_owned()))
        .collect::<Result<Vec<_>, _>>()?;
    names.sort();
    Ok(names)
}
