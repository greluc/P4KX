//! A synthetic DataForge with one Record of many fields, in an Archive, for the tests of keyboard
//! scrolling.

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use p4kx_testkit::{
    ArchiveBuilder, DataForgeBuilder, TestEntry, TestRecord, TestStruct, TestType, TestValue,
};

pub(crate) type TestResult = Result<(), Box<dyn Error>>;

/// The number of fields of the Record.
pub(crate) const FIELDS: i32 = 200;

const GUID: u128 = 0x7000_0000_0000_4000_8000_0000_0000_0001;

/// Returns the name of the field at `index`.
pub(crate) fn name(index: i32) -> String {
    format!("f{index:03}")
}

/// Returns a DataForge whose only Record holds every field with `value`.
pub(crate) fn database(value: i32) -> DataForgeBuilder {
    let wide = (0..FIELDS).fold(TestStruct::new("Wide"), |wide, index| {
        wide.field(name(index), TestType::Int32)
    });
    let record = (0..FIELDS).fold(
        TestRecord::new(GUID, "Wide", "Wide.One", "libs/wide/record.xml"),
        |record, index| record.value(name(index), TestValue::Int32(value)),
    );
    DataForgeBuilder::new().structure(wide).record(record)
}

/// Writes an Archive with `database(value)` as its DataForge to `channel` of `dir`.
pub(crate) fn archive(dir: &Path, channel: &str, value: i32) -> Result<PathBuf, Box<dyn Error>> {
    let folder = dir.join(channel);
    fs::create_dir_all(&folder)?;
    let path = folder.join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::zstd("Data\\Game2.dcb", database(value).build()))
        .write_to(&path)?;
    Ok(path)
}
