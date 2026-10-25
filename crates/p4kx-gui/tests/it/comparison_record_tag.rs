// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! A Record whose tag alone differs shows a row of its own in the field changes.

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use p4kx_gui::{FieldName, FieldRow, FieldValue};
use p4kx_testkit::{
    ArchiveBuilder, DataForgeBuilder, TestEntry, TestRecord, TestStruct, TestType, TestValue,
};

use crate::comparison_record_fields::fields_of;
use crate::record_model::compared;
use crate::record_pair::{BOTH, TestResult};

const TAGGED: u128 = 0x7100_0000_0000_4000_8000_0000_0000_0001;
const GAINED: u128 = 0x7100_0000_0000_4000_8000_0000_0000_0002;

fn database(new: bool) -> DataForgeBuilder {
    let tag = |old: &str, new_tag: &str| {
        if new {
            new_tag.to_owned()
        } else {
            old.to_owned()
        }
    };
    DataForgeBuilder::new()
        .version(8)
        .structure(TestStruct::new("Ship").field("speed", TestType::Int32))
        .record(
            TestRecord::new(TAGGED, "Ship", "Ship.Tagged", "libs/ships/tagged.xml")
                .tag(tag("SystemsDesign", "Vehicles"))
                .value("speed", TestValue::Int32(1)),
        )
        .record(
            TestRecord::new(GAINED, "Ship", "Ship.Gained", "libs/ships/gained.xml")
                .tag(tag("", "Vehicles"))
                .value("speed", TestValue::Int32(1)),
        )
}

fn write(dir: &Path, channel: &str, new: bool) -> Result<PathBuf, Box<dyn Error>> {
    let folder = dir.join(channel);
    fs::create_dir_all(&folder)?;
    let path = folder.join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::stored("Data\\Game2.dcb", database(new).build()))
        .write_to(&path)?;
    Ok(path)
}

fn tag_row(old: Option<&str>, new: Option<&str>) -> FieldRow {
    let scalar = |text: &str| FieldValue::Scalar(text.into());
    FieldRow {
        name: FieldName::RecordTag,
        old: old.map(scalar),
        new: new.map(scalar),
    }
}

#[test]
fn a_changed_tag_is_a_row_of_its_own_and_a_gained_tag_has_no_old_value() -> TestResult {
    let dir = tempfile::tempdir()?;
    let (old, new) = (
        write(dir.path(), "LIVE", false)?,
        write(dir.path(), "PTU", true)?,
    );
    let mut model = compared((&old, &new), BOTH)?;

    let changed = fields_of(&mut model, "libs/ships/tagged")?;
    let gained = fields_of(&mut model, "libs/ships/gained")?;

    assert_eq!(
        changed.rows(),
        [tag_row(Some("SystemsDesign"), Some("Vehicles"))],
        "only the tag differs"
    );
    assert_eq!(
        gained.rows(),
        [tag_row(None, Some("Vehicles"))],
        "no tag before"
    );
    Ok(())
}
