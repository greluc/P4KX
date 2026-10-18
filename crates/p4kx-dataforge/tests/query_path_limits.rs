//! Paths over data that cannot be read and over more elements than a Record may reach: error
//! values at their place, typed and never silent.

mod ships;

use std::error::Error;

use p4kx_dataforge::DataForge;
use p4kx_dataforge::query::MAX_PATH_VALUES;
use p4kx_testkit::{TestInstance, TestRecord, TestValue};
use ships::{database, rows, schema};

type TestResult = Result<(), Box<dyn Error>>;

const TINY_STRUCT: u32 = 6;

fn assert_rows(forge: &DataForge<'_>, text: &str, expected: &[&str]) -> TestResult {
    assert_eq!(rows(forge, text)?, expected, "{text}");
    Ok(())
}

fn words(first: u32, second: u32) -> TestValue {
    TestValue::Raw([first.to_le_bytes(), second.to_le_bytes()].concat())
}

fn weapon(name: &str, modes: impl Iterator<Item = i32>) -> TestValue {
    TestValue::Struct(
        TestInstance::new("Weapon")
            .value("name", TestValue::text(name))
            .value(
                "modes",
                TestValue::Array(modes.map(TestValue::Int32).collect()),
            ),
    )
}

fn too_wide() -> String {
    format!("error: the paths of the Record reach more than {MAX_PATH_VALUES} array elements")
}

#[test]
fn a_step_that_cannot_be_read_gives_an_error_value_at_its_place() -> TestResult {
    let ship_struct = 4;
    let bytes = schema()
        .record(
            TestRecord::new(8, "Ship", "Ship.Broken", "libs/ships/broken.xml")
                .value("anchor", words(ship_struct, 0))
                .value("Components", words(3, 1000))
                .value("tags", words(2, 1_000_000)),
        )
        .record(
            TestRecord::new(9, "Ship", "Ship.Dangling", "libs/ships/dangling.xml")
                .value("anchor", words(0, 99)),
        )
        .build();
    let forge = DataForge::open(&bytes)?;

    assert_rows(
        &forge,
        "from Ship select anchor.x, Components[*].name, tags[0], hardpoints[0].x",
        &[
            "broken: error: the pointer refers to struct 4, which does not derive from struct 0, \
             the type of the field | [error: the array of 3 values from index 1000 lies outside \
             the strong pointer pool of 0 values] | error: the array of 2 values from index \
             1000000 lies outside the string pool of 0 values | null",
            "dangling: error: the pointer refers to instance 99 of struct 0, which does not exist \
             | [] | null | null",
        ],
    )?;
    assert_rows(
        &forge,
        "from Ship where anchor.x = 0 or tags[*] = \"\"",
        &[],
    )
}

#[test]
fn an_instance_shorter_than_its_declared_layout_is_an_error_value() -> TestResult {
    let bytes = schema()
        .record(
            TestRecord::new(8, "Ship", "Ship.Short", "libs/ships/short.xml")
                .value("anchor", TestValue::Struct(TestInstance::new("Tiny"))),
        )
        .build();
    let forge = DataForge::open(&bytes)?;

    assert_rows(
        &forge,
        "from Ship select anchor.x, anchor.y",
        &[&format!(
            "short: 0 | error: the instance of struct {TINY_STRUCT} has 4 bytes, but the path \
             reads up to byte 8"
        )],
    )?;
    assert_rows(&forge, "from Ship where not anchor.y = 1", &[])
}

#[test]
fn a_component_selector_reports_a_pointer_to_a_missing_struct() -> TestResult {
    let bytes = schema()
        .record(
            TestRecord::new(8, "Ship", "Ship.Odd", "libs/ships/odd.xml").value(
                "Components",
                TestValue::Array(vec![words(77, 0), weapon("Laser", 1..3)]),
            ),
        )
        .build();
    let forge = DataForge::open(&bytes)?;

    assert_rows(
        &forge,
        "from Ship select Components[Weapon].name, Components[*].name",
        &[
            "odd: [error: the pointer refers to instance 0 of struct 77, which does not exist, \
           Laser] | [error: the pointer refers to instance 0 of struct 77, which does not exist, \
           Laser]",
        ],
    )
}

#[test]
fn nested_arrays_are_flattened_in_order() -> TestResult {
    let bytes = schema()
        .record(
            TestRecord::new(8, "Ship", "Ship.Nest", "libs/ships/nest.xml").value(
                "Components",
                TestValue::Array(vec![
                    weapon("A", 1..3),
                    weapon("B", 3..4),
                    weapon("C", 0..0),
                ]),
            ),
        )
        .build();
    let forge = DataForge::open(&bytes)?;

    assert_rows(
        &forge,
        "from Ship select Components[*].modes[*], Components[Weapon].modes[0]",
        &["nest: [1, 2, 3] | [1, 3, null]"],
    )?;
    let ships = database();
    assert_rows(
        &DataForge::open(&ships)?,
        "from Ship select Components[*].modes[*]",
        &["alpha: []", "beta: [null]"],
    )?;
    assert_rows(
        &forge,
        "from Ship where Components[*].modes[*] = 3",
        &["nest: "],
    )
}

#[test]
fn the_budget_counts_every_level_of_nested_arrays() -> TestResult {
    let per_weapon = i32::try_from(MAX_PATH_VALUES / 64).unwrap_or(0);
    let components = |modes: i32| {
        TestValue::Array(
            (0..64)
                .map(|index| weapon(&format!("W{index}"), 0..modes))
                .collect(),
        )
    };
    let bytes = schema()
        .record(
            TestRecord::new(8, "Ship", "Ship.Fits", "libs/ships/fits.xml")
                .value("Components", components(per_weapon - 1)),
        )
        .record(
            TestRecord::new(9, "Ship", "Ship.Wide", "libs/ships/wide.xml")
                .value("Components", components(per_weapon)),
        )
        .build();
    let forge = DataForge::open(&bytes)?;

    let rows = rows(&forge, "from Ship select Components[*].modes[*]")?;

    assert!(
        rows.first()
            .is_some_and(|row| row.starts_with("fits: [0, 1")),
        "{rows:?}"
    );
    assert_eq!(
        rows.get(1),
        Some(&format!("wide: {}", too_wide())),
        "64 + 64 × 64 > 4096"
    );
    Ok(())
}

#[test]
fn every_path_of_a_record_shares_one_budget() -> TestResult {
    let many = MAX_PATH_VALUES + 1;
    let tags = |count: usize| {
        TestValue::Array(
            (0..count)
                .map(|index| TestValue::Int32(i32::try_from(index).unwrap_or(0)))
                .collect(),
        )
    };
    let bytes = schema()
        .record(
            TestRecord::new(8, "Station", "Station.Big", "libs/stations/big.xml")
                .value("tags", tags(many)),
        )
        .record(
            TestRecord::new(9, "Station", "Station.Half", "libs/stations/half.xml")
                .value("tags", tags(MAX_PATH_VALUES / 2 + 1)),
        )
        .build();
    let forge = DataForge::open(&bytes)?;

    let rows = rows(&forge, "from Station select tags[*], tags[4096], tags[*]")?;

    assert_eq!(
        rows.first(),
        Some(&format!("big: {} | 4096 | {}", too_wide(), too_wide())),
        "too many for one path"
    );
    assert!(
        rows.get(1)
            .is_some_and(|row| row.ends_with(&format!("| null | {}", too_wide()))),
        "two paths over half the budget each: {:?}",
        rows.get(1).map(String::len)
    );
    assert_rows(&forge, "from Station where tags[*] = 1", &["half: "])?;
    assert_rows(&forge, "from Station where not tags[*] = 1", &[])?;
    assert_rows(
        &forge,
        "from Station where tags[*] = -1 or tags[*] = 1",
        &[],
    )?;
    assert_rows(&forge, "from Station where tags[4096] = 4096", &["big: "])
}
