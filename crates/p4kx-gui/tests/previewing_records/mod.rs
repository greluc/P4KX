//! A synthetic DataForge whose Records link to each other, opened headless, and helpers that
//! select Records and wait for their preview.

use std::error::Error;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use p4kx_gui::{
    Browser, Language, Model, RecordPreview, RecordPreviewStatus, RecordTree, RecordTreeStatus,
};
use p4kx_testkit::{
    ArchiveBuilder, DataForgeBuilder, TestEntry, TestInstance, TestRecord, TestStruct, TestType,
    TestValue,
};
use tempfile::TempDir;

pub(crate) type TestResult = Result<(), Box<dyn Error>>;

pub(crate) const MAKER: u128 = 0x7000_0000_0000_4000_8000_0000_0000_0001;
pub(crate) const ITEM: u128 = 0x7000_0000_0000_4000_8000_0000_0000_0002;
pub(crate) const SHIP: u128 = 0x7000_0000_0000_4000_8000_0000_0000_0003;
pub(crate) const GONE: u128 = 0x7000_0000_0000_4000_8000_0000_0000_00ff;

/// The record paths of the Records, as the tree finds them.
pub(crate) const MAKER_PATH: &str = "libs/makers/one";
pub(crate) const ITEM_PATH: &str = "libs/items/sample";
pub(crate) const SHIP_PATH: &str = "libs/ships/hauler";

/// A Maker, an Item that links to the Maker and to a Record that does not exist, holds owned
/// Nodes and a Hull with an unreadable label, and a Ship that links to the Item.
pub(crate) fn database() -> DataForgeBuilder {
    let node = |weight: i32| {
        TestValue::Struct(TestInstance::new("Node").value("weight", TestValue::Int32(weight)))
    };
    DataForgeBuilder::new()
        .structure(TestStruct::new("Maker").field("rank", TestType::Int32))
        .structure(
            TestStruct::new("Hull")
                .field("label", TestType::String)
                .field("armor", TestType::Int32),
        )
        .structure(TestStruct::new("Node").field("weight", TestType::Int32))
        .structure(
            TestStruct::new("Item")
                .field("maker", TestType::Reference)
                .field("spare", TestType::Reference)
                .array("nodes", TestType::Class("Node".to_owned()))
                .field("hull", TestType::Class("Hull".to_owned()))
                .field("speed", TestType::Int32),
        )
        .structure(TestStruct::new("Ship").field("cargo", TestType::Reference))
        .record(
            TestRecord::new(MAKER, "Maker", "Maker.One", "libs/makers/one.xml")
                .value("rank", TestValue::Int32(7)),
        )
        .record(
            TestRecord::new(ITEM, "Item", "Item.Sample", "libs/items/sample.xml")
                .value("maker", TestValue::Reference(MAKER))
                .value("spare", TestValue::Reference(GONE))
                .value("nodes", TestValue::Array(vec![node(1), node(2)]))
                .value(
                    "hull",
                    TestValue::Struct(
                        TestInstance::new("Hull")
                            .value("label", TestValue::Raw(u32::MAX.to_le_bytes().to_vec()))
                            .value("armor", TestValue::Int32(3)),
                    ),
                )
                .value("speed", TestValue::Int32(5)),
        )
        .record(
            TestRecord::new(SHIP, "Ship", "Ship.Hauler", "libs/ships/hauler.xml")
                .value("cargo", TestValue::Reference(ITEM)),
        )
}

/// Writes an Archive with `database` as its DataForge.
pub(crate) fn archive(database: &DataForgeBuilder) -> Result<(TempDir, PathBuf), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::stored("Data\\Libs\\config.xml", "config"))
        .entry(TestEntry::zstd("Data\\Game2.dcb", database.build()))
        .write_to(&path)?;
    Ok((dir, path))
}

/// Opens the Archive of [`database`] and loads its DataForge.
pub(crate) fn opened() -> Result<(TempDir, PathBuf, Model), Box<dyn Error>> {
    let (dir, path) = archive(&database())?;
    let mut model = Model::new(Language::BritishEnglish);
    model.open(path.clone(), || {});
    wait_until(&mut model, |model| model.browser().is_some())?;
    model.browser_mut().ok_or("no Archive")?.load_records(|| {});
    wait_until(&mut model, |model| {
        tree(model).is_ok_and(|tree| matches!(tree.status(), RecordTreeStatus::Ready(_)))
    })?;
    Ok((dir, path, model))
}

/// Polls `model` until `done` holds.
pub(crate) fn wait_until(model: &mut Model, done: impl Fn(&Model) -> bool) -> TestResult {
    let deadline = Instant::now() + Duration::from_secs(30);
    while !done(model) {
        if Instant::now() > deadline {
            return Err("the model did not settle in time".into());
        }
        std::thread::sleep(Duration::from_millis(5));
        model.poll();
        model.query_mut().poll();
        if let Some(browser) = model.browser_mut() {
            browser.poll();
        }
    }
    Ok(())
}

/// Returns the Record tree.
pub(crate) fn tree(model: &Model) -> Result<&RecordTree, Box<dyn Error>> {
    model
        .browser()
        .and_then(Browser::records)
        .ok_or_else(|| "no DataForge node".into())
}

/// Returns the record path of the Record selected in the tree.
pub(crate) fn selected_path(model: &Model) -> Option<String> {
    let browser = model.browser()?;
    let id = browser.selected_record()?;
    Some(browser.records()?.record(id)?.record_path().to_owned())
}

/// Selects the Record at `path` in the tree as a click on its row does.
pub(crate) fn select(model: &mut Model, path: &str) -> TestResult {
    let node = tree(model)?
        .find(path)
        .ok_or_else(|| format!("no node {path}"))?;
    model
        .browser_mut()
        .ok_or("no Archive")?
        .select_record_node(node, || {});
    Ok(())
}

/// Asks for the preview of the selected Record and waits until it was read.
pub(crate) fn previewed(model: &mut Model) -> Result<&RecordPreview, Box<dyn Error>> {
    model
        .browser_mut()
        .ok_or("no Archive")?
        .load_record_preview(|| {});
    wait_until(model, |model| {
        model
            .browser()
            .and_then(Browser::record_preview)
            .is_some_and(|preview| !matches!(preview.status(), RecordPreviewStatus::Loading))
    })?;
    model
        .browser()
        .and_then(Browser::record_preview)
        .ok_or_else(|| "no Record preview".into())
}
