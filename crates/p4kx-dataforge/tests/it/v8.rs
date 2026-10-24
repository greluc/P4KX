// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Opening a version 8 DataForge, whose Records carry a tag, next to an equivalent version 6 one.

use std::error::Error;

use p4kx_dataforge::{DataForge, DataForgeError, SUPPORTED_VERSIONS, ValueKind};
use p4kx_testkit::{DataForgeBuilder, TestRecord, TestStruct, TestType};

type TestResult = Result<(), Box<dyn Error>>;

const AVENGER: u128 = 0xcbee_34fd_6441_4000_b99f_924f_7f26_e19f;
const BOX: u128 = 0x50e8_a9f6_f956_4000_8ace_7c0c_a9a9_1f5d;
const CRATE: u128 = 0x0000_0001_0002_0003_0004_0000_0000_0005;

fn sample(version: u32) -> DataForgeBuilder {
    DataForgeBuilder::new()
        .version(version)
        .enumeration("Size", ["Small", "Medium", "Large"])
        .structure(
            TestStruct::new("Item")
                .field("mass", TestType::Single)
                .field("size", TestType::Enum("Size".to_owned())),
        )
        .structure(
            TestStruct::new("Ship")
                .parent("Item")
                .field("crew", TestType::Int32)
                .array("weapons", TestType::Reference),
        )
        .record(
            TestRecord::new(
                AVENGER,
                "Ship",
                "Ship.Avenger",
                "libs/foundry/records/ships/avenger.xml",
            )
            .tag("Ship"),
        )
        .record(
            TestRecord::new(
                BOX,
                "Item",
                "Item.Box",
                "libs/foundry/records/items/boxes.xml",
            )
            .tag("SystemsDesign"),
        )
        .record(TestRecord::new(
            CRATE,
            "Item",
            "Item.Crate",
            "libs/foundry/records/items/boxes.xml",
        ))
}

#[test]
fn reports_version_8_and_the_table_sizes() -> TestResult {
    let bytes = sample(8).build();

    let forge = DataForge::open(&bytes)?;

    assert_eq!(forge.version(), 8, "version");
    assert_eq!(forge.record_count(), 3, "Records");
    assert_eq!(forge.struct_count(), 2, "structs");
    assert_eq!(forge.enum_count(), 1, "enums");
    assert_eq!(forge.property_count(), 4, "properties");
    assert_eq!(forge.data_mapping_count(), 2, "data mappings");
    assert_eq!(forge.value_count(ValueKind::EnumOption), 3, "enum options");
    Ok(())
}

#[test]
fn reads_the_tag_of_every_record() -> TestResult {
    let v8 = sample(8).build();
    let v6 = sample(6).build();

    let tags = |bytes| -> Result<Vec<Option<String>>, DataForgeError> {
        DataForge::open(bytes)?
            .records()
            .map(|record| Ok(record.tag()?.map(str::to_owned)))
            .collect()
    };

    assert_eq!(
        tags(&v8)?,
        [
            Some("Ship".to_owned()),
            Some("SystemsDesign".to_owned()),
            Some(String::new())
        ],
        "version 8 stores a tag per Record"
    );
    assert_eq!(tags(&v6)?, [None, None, None], "version 6 has no tags");
    Ok(())
}

type Listing = Vec<(String, String, String, String)>;

fn listing(bytes: &[u8]) -> Result<Listing, DataForgeError> {
    DataForge::open(bytes)?
        .records()
        .map(|record| {
            Ok((
                record.guid().to_string(),
                record.type_name()?.to_owned(),
                record.name()?.to_owned(),
                record.record_path()?.to_owned(),
            ))
        })
        .collect()
}

#[test]
fn lists_the_same_records_as_an_equivalent_version_6_database() -> TestResult {
    let v8 = listing(&sample(8).build())?;
    let v6 = listing(&sample(6).build())?;

    let names: Vec<&str> = v8.iter().map(|record| record.2.as_str()).collect();
    assert_eq!(
        names,
        ["Ship.Avenger", "Item.Box", "Item.Crate"],
        "definition order"
    );
    assert_eq!(
        v8.first()
            .map(|record| (record.0.as_str(), record.1.as_str(), record.3.as_str())),
        Some((
            "cbee34fd-6441-4000-b99f-924f7f26e19f",
            "Ship",
            "libs/foundry/records/ships/avenger"
        )),
        "GUID, type and record path of the first Record"
    );
    assert_eq!(v8, v6, "the same Records as version 6");
    Ok(())
}

#[test]
fn looks_up_records_like_an_equivalent_version_6_database() -> TestResult {
    for version in SUPPORTED_VERSIONS {
        let bytes = sample(version).build();
        let forge = DataForge::open(&bytes)?;
        let by_guid = |guid: &str| -> Result<Option<String>, Box<dyn Error>> {
            Ok(forge
                .record_by_guid(guid.parse()?)
                .map(|record| record.name())
                .transpose()?
                .map(str::to_owned))
        };
        let at_path = |path: &str| {
            forge
                .records_at_path(path)
                .map(|record| record.name())
                .collect::<Result<Vec<_>, _>>()
        };

        assert_eq!(
            by_guid("50E8A9F6-F956-4000-8ACE-7C0CA9A91F5D")?.as_deref(),
            Some("Item.Box"),
            "version {version}: the Record with that GUID"
        );
        assert_eq!(
            by_guid("50e8a9f6-f956-4000-8ace-7c0ca9a91f5e")?,
            None,
            "version {version}: no Record has that GUID"
        );
        assert_eq!(
            at_path("Libs\\Foundry\\Records\\Items\\Boxes.XML")?,
            ["Item.Box", "Item.Crate"],
            "version {version}: both Records of the file"
        );
        assert!(
            at_path("libs/foundry/records/items")?.is_empty(),
            "version {version}: a folder is not a record path"
        );
    }
    Ok(())
}

#[test]
fn every_truncation_of_a_version_8_database_is_an_error() {
    let bytes = sample(8).build();

    for len in 0..bytes.len() {
        let result = DataForge::open(bytes.get(..len).unwrap_or_default());

        assert!(
            result.is_err(),
            "{len} of {} bytes are rejected",
            bytes.len()
        );
    }
}

#[test]
fn a_version_6_layout_labelled_version_8_is_an_error() {
    let mut bytes = sample(6).build();
    if let Some(version) = bytes.get_mut(4) {
        *version = 8;
    }

    let result = DataForge::open(&bytes);

    assert!(
        result.is_err(),
        "the longer Record rows do not fit: {result:?}"
    );
}

#[test]
fn an_invalid_tag_is_an_error_of_its_record() -> TestResult {
    let mut bytes = sample(8).build();
    let forge = DataForge::open(&bytes)?;
    let records = 120
        + forge.struct_count() * 16
        + forge.property_count() * 12
        + forge.enum_count() * 8
        + forge.data_mapping_count() * 8;
    let tag = records + 36 + 8;
    if let Some(field) = bytes.get_mut(tag..tag + 4) {
        field.copy_from_slice(&(u32::MAX - 1).to_le_bytes());
    }

    let forge = DataForge::open(&bytes)?;
    let tags: Vec<_> = forge.records().map(|record| record.tag()).collect();

    assert!(
        matches!(
            tags.as_slice(),
            [
                Ok(Some("Ship")),
                Err(DataForgeError::InvalidString { table: "blob", .. }),
                Ok(Some(""))
            ]
        ),
        "only the broken tag fails: {tags:?}"
    );
    assert_eq!(
        forge
            .records()
            .nth(1)
            .map(|record| record.name())
            .transpose()?,
        Some("Item.Box"),
        "the Record is still listed"
    );
    Ok(())
}
