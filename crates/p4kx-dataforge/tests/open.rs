//! Opening a version 6 DataForge and reading its header and tables.

use std::error::Error;

use p4kx_dataforge::{DataForge, DataForgeError, DataType, Guid, ValueKind};
use p4kx_testkit::{DataForgeBuilder, TestPool, TestRecord, TestStruct, TestType};

type TestResult = Result<(), Box<dyn Error>>;

const AVENGER: u128 = 0xcbee_34fd_6441_4000_b99f_924f_7f26_e19f;
const BOX: u128 = 0x50e8_a9f6_f956_4000_8ace_7c0c_a9a9_1f5d;
const CRATE: u128 = 0x0000_0001_0002_0003_0004_0000_0000_0005;

fn sample() -> DataForgeBuilder {
    DataForgeBuilder::new()
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
        .record(TestRecord::new(
            AVENGER,
            "Ship",
            "Ship.Avenger",
            "libs/foundry/records/ships/avenger.xml",
        ))
        .record(TestRecord::new(
            BOX,
            "Item",
            "Item.Box",
            "libs/foundry/records/items/boxes.xml",
        ))
        .record(TestRecord::new(
            CRATE,
            "Item",
            "Item.Crate",
            "libs/foundry/records/items/boxes.xml",
        ))
}

#[test]
fn reports_the_version_and_the_table_sizes() -> TestResult {
    let bytes = sample().build();

    let forge = DataForge::open(&bytes)?;

    assert_eq!(forge.version(), 6, "version");
    assert_eq!(forge.record_count(), 3, "Records");
    assert_eq!(forge.struct_count(), 2, "structs");
    assert_eq!(forge.enum_count(), 1, "enums");
    assert_eq!(forge.property_count(), 4, "properties");
    assert_eq!(forge.data_mapping_count(), 2, "data mappings");
    assert_eq!(forge.value_count(ValueKind::EnumOption), 3, "enum options");
    Ok(())
}

#[test]
fn lists_records_with_guid_type_and_record_path() -> TestResult {
    let bytes = sample().build();
    let forge = DataForge::open(&bytes)?;

    let records = forge
        .records()
        .map(|record| {
            Ok((
                record.guid().to_string(),
                record.type_name()?,
                record.name()?,
                record.record_path()?,
            ))
        })
        .collect::<Result<Vec<_>, DataForgeError>>()?;

    assert_eq!(
        records,
        [
            (
                "cbee34fd-6441-4000-b99f-924f7f26e19f".to_owned(),
                "Ship",
                "Ship.Avenger",
                "libs/foundry/records/ships/avenger"
            ),
            (
                "50e8a9f6-f956-4000-8ace-7c0ca9a91f5d".to_owned(),
                "Item",
                "Item.Box",
                "libs/foundry/records/items/boxes"
            ),
            (
                "00000001-0002-0003-0004-000000000005".to_owned(),
                "Item",
                "Item.Crate",
                "libs/foundry/records/items/boxes"
            ),
        ],
        "every Record in definition order"
    );
    Ok(())
}

#[test]
fn looks_up_a_record_by_guid() -> TestResult {
    let bytes = sample().build();
    let forge = DataForge::open(&bytes)?;

    let found = forge.record_by_guid("50E8A9F6-F956-4000-8ACE-7C0CA9A91F5D".parse()?);
    let missing = forge.record_by_guid("50e8a9f6-f956-4000-8ace-7c0ca9a91f5e".parse()?);

    assert_eq!(
        found.map(|record| record.name()).transpose()?,
        Some("Item.Box"),
        "the Record with that GUID"
    );
    assert!(missing.is_none(), "no Record has that GUID");
    Ok(())
}

#[test]
fn looks_up_the_records_at_a_record_path() -> TestResult {
    let bytes = sample().build();
    let forge = DataForge::open(&bytes)?;
    let names = |path: &str| {
        forge
            .records_at_path(path)
            .map(|record| record.name())
            .collect::<Result<Vec<_>, _>>()
    };

    assert_eq!(
        names("libs/foundry/records/items/boxes")?,
        ["Item.Box", "Item.Crate"],
        "both Records of the file, in definition order"
    );
    assert_eq!(
        names("Libs\\Foundry\\Records\\Ships\\Avenger.XML")?,
        ["Ship.Avenger"],
        "case, separator and the .xml extension do not matter"
    );
    assert!(
        names("libs/foundry/records/ships")?.is_empty(),
        "a folder is not a record path"
    );
    Ok(())
}

#[test]
fn enumerates_structs_with_parent_and_properties() -> TestResult {
    let bytes = sample().build();
    let forge = DataForge::open(&bytes)?;

    let ship = forge
        .structs()
        .find(|structure| structure.name().is_ok_and(|name| name == "Ship"))
        .ok_or("no struct Ship")?;
    let properties = ship
        .properties()
        .map(|property| Ok((property.name()?, property.data_type()?, property.is_array())))
        .collect::<Result<Vec<_>, DataForgeError>>()?;

    assert_eq!(
        ship.parent().map(|parent| parent.name()).transpose()?,
        Some("Item"),
        "parent struct"
    );
    assert_eq!(ship.size(), 4 + 4 + 4 + 8, "inherited and own fields");
    assert_eq!(
        properties,
        [
            ("crew", DataType::Int32, false),
            ("weapons", DataType::Reference, true)
        ],
        "own properties in declaration order"
    );
    let avenger = forge.records().next().ok_or("no Record")?;
    assert_eq!(avenger.struct_type()?.name()?, "Ship", "a Record's struct");
    assert_eq!(ship.instance_count(), 1, "one instance per Record");
    Ok(())
}

#[test]
fn properties_name_the_struct_or_enum_their_values_belong_to() -> TestResult {
    let bytes = sample()
        .structure(
            TestStruct::new("Hangar")
                .field("ship", TestType::Class("Ship".to_owned()))
                .field("best", TestType::ReferenceTo("Item".to_owned()))
                .field("any", TestType::Reference),
        )
        .build();
    let forge = DataForge::open(&bytes)?;
    let named = |name: &str| {
        forge
            .structs()
            .find(|structure| structure.name().is_ok_and(|found| found == name))
    };
    let targets = named("Hangar")
        .ok_or("no Hangar")?
        .properties()
        .map(|property| {
            Ok((
                property
                    .target_struct()
                    .map(|target| target.name())
                    .transpose()?,
                property
                    .target_enum()
                    .map(|target| target.name())
                    .transpose()?,
            ))
        })
        .collect::<Result<Vec<_>, DataForgeError>>()?;
    let size = named("Item")
        .ok_or("no Item")?
        .properties()
        .find_map(|property| property.target_enum())
        .map(|target| target.name())
        .transpose()?;

    assert_eq!(
        targets,
        [(Some("Ship"), None), (Some("Item"), None), (None, None)],
        "the struct of a class and of a typed Reference"
    );
    assert_eq!(size, Some("Size"), "the enum of an enum field");
    Ok(())
}

#[test]
fn enumerates_enums_with_their_options() -> TestResult {
    let bytes = sample().build();
    let forge = DataForge::open(&bytes)?;

    let enums = forge
        .enums()
        .map(|enumeration| {
            Ok((
                enumeration.name()?,
                enumeration.options().collect::<Result<Vec<_>, _>>()?,
            ))
        })
        .collect::<Result<Vec<_>, DataForgeError>>()?;

    assert_eq!(
        enums,
        [("Size", vec!["Small", "Medium", "Large"])],
        "enum options in order"
    );
    Ok(())
}

#[test]
fn reads_the_tables_behind_filled_value_pools() -> TestResult {
    let bytes = sample()
        .values(TestPool::Int8, 3)
        .values(TestPool::Bool, 5)
        .values(TestPool::Guid, 2)
        .values(TestPool::Reference, 7)
        .values(TestPool::EnumOption, 1)
        .build();

    let forge = DataForge::open(&bytes)?;

    assert_eq!(forge.value_count(ValueKind::Int8), 3, "int8 values");
    assert_eq!(forge.value_count(ValueKind::Bool), 5, "bool values");
    assert_eq!(forge.value_count(ValueKind::Guid), 2, "GUID values");
    assert_eq!(forge.value_count(ValueKind::Reference), 7, "References");
    assert_eq!(forge.value_count(ValueKind::Double), 0, "no doubles");
    assert_eq!(
        forge.value_count(ValueKind::EnumOption),
        4,
        "own and extra options"
    );
    assert_eq!(
        forge
            .records()
            .last()
            .map(|record| record.record_path())
            .transpose()?,
        Some("libs/foundry/records/items/boxes"),
        "text table found behind the pools"
    );
    Ok(())
}

#[test]
fn an_unknown_version_is_an_error() {
    for version in [0, 1, 5, 7, 9, 10, u32::MAX] {
        let bytes = sample().version(version).build();

        let result = DataForge::open(&bytes);

        assert!(
            matches!(result, Err(DataForgeError::UnsupportedVersion { version: v }) if v == version),
            "version {version} is rejected: {result:?}"
        );
    }
}

#[test]
fn every_truncation_is_an_error() {
    let bytes = sample().build();

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
fn trailing_bytes_are_an_error() {
    let mut bytes = sample().build();
    bytes.push(0);

    let result = DataForge::open(&bytes);

    assert!(
        matches!(result, Err(DataForgeError::InstanceSize { .. })),
        "the instance data must end the input: {result:?}"
    );
}

#[test]
fn a_record_of_an_unknown_struct_is_an_error() {
    let bytes = sample()
        .record(TestRecord::new(1, "Missing", "Missing.One", "a.xml"))
        .build();

    let result = DataForge::open(&bytes);

    assert!(
        matches!(result, Err(DataForgeError::IndexOutOfRange { .. })),
        "the struct index is checked: {result:?}"
    );
}

#[test]
fn guids_parse_and_print_in_text_form() -> TestResult {
    let guid: Guid = "CBEE34FD-6441-4000-B99F-924F7F26E19F".parse()?;

    assert_eq!(
        guid.to_string(),
        "cbee34fd-6441-4000-b99f-924f7f26e19f",
        "lower-case text"
    );
    for invalid in [
        "",
        "cbee34fd-6441-4000-b99f-924f7f26e19",
        "cbee34fd64414000b99f924f7f26e19f",
        "xbee34fd-6441-4000-b99f-924f7f26e19f",
        "cbee34fd-6441-4000-b99f-924f7f26e19ff",
    ] {
        assert!(invalid.parse::<Guid>().is_err(), "{invalid:?} is rejected");
    }
    Ok(())
}
