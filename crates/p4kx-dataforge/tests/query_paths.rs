//! Paths into nested data: dot paths through inline structs and strong pointers, `[n]`, `[*]`
//! and `[TypeName]` in `select` and `where`.

mod ships;

use std::error::Error;

use p4kx_dataforge::DataForge;
use p4kx_dataforge::query::MAX_PATH_VALUES;
use p4kx_testkit::{TestRecord, TestValue};
use ships::{database, rows, schema};

type TestResult = Result<(), Box<dyn Error>>;

fn assert_rows(forge: &DataForge<'_>, text: &str, expected: &[&str]) -> TestResult {
    assert_eq!(rows(forge, text)?, expected, "{text}");
    Ok(())
}

#[test]
fn dot_paths_continue_into_inline_structs_and_strong_pointers() -> TestResult {
    let bytes = database();
    let forge = DataForge::open(&bytes)?;

    assert_rows(
        &forge,
        "from Ship select position.x, position.y, anchor.y, mass",
        &["alpha: 1.5 | -2 | 8 | 100", "beta: 0 | 0 | null | 50"],
    )
}

#[test]
fn an_index_selects_one_element_and_out_of_range_is_null() -> TestResult {
    let bytes = database();
    let forge = DataForge::open(&bytes)?;

    assert_rows(
        &forge,
        "from Ship select tags[0], tags[1], Components[1].name, hardpoints[1].y, Components[0].name",
        &[
            "alpha: fast | small | Bubble | 4 | Laser",
            "beta: slow | null | Wall | null | null",
        ],
    )
}

#[test]
fn a_star_gives_a_list_of_every_element_in_order() -> TestResult {
    let bytes = database();
    let forge = DataForge::open(&bytes)?;

    assert_rows(
        &forge,
        "from Ship select tags[*], Components[*].name, hardpoints[*].x",
        &[
            "alpha: [fast, small] | [Laser, Bubble, Cannon] | [1, 3]",
            "beta: [slow] | [null, Wall] | []",
        ],
    )
}

#[test]
fn a_type_selects_the_elements_of_that_type_or_a_subtype() -> TestResult {
    let bytes = database();
    let forge = DataForge::open(&bytes)?;

    assert_rows(
        &forge,
        "from Ship select Components[Weapon].damage, Components[Shield].hp, \
         Components[Component].name, Components[Weapon].name",
        &[
            "alpha: [250.5, 1200] | [900] | [Laser, Bubble, Cannon] | [Laser, Cannon]",
            "beta: [] | [10] | [Wall] | []",
        ],
    )
}

#[test]
fn a_comparison_over_several_values_holds_when_one_value_holds() -> TestResult {
    let bytes = database();
    let forge = DataForge::open(&bytes)?;
    let cases: [(&str, &[&str]); 8] = [
        ("Components[Weapon].damage > 1000", &["alpha: "]),
        ("not Components[Weapon].damage > 1000", &["beta: "]),
        ("tags[*] = \"slow\"", &["beta: "]),
        ("Components[*].name = \"Wall\"", &["beta: "]),
        ("Components[*].name != \"Wall\"", &["alpha: "]),
        ("not Components[*].name != \"Wall\"", &[]),
        ("tags[1] = \"small\"", &["alpha: "]),
        ("not tags[1] = \"small\"", &[]),
    ];
    for (condition, expected) in cases {
        assert_rows(&forge, &format!("from Ship where {condition}"), expected)?;
    }
    Ok(())
}

#[test]
fn without_from_each_type_resolves_the_path_itself() -> TestResult {
    let bytes = database();
    let forge = DataForge::open(&bytes)?;

    assert_rows(
        &forge,
        "at \"libs\" select position.x, tags[0], tags[*]",
        &[
            "alpha: 1.5 | fast | [fast, small]",
            "beta: 0 | slow | [slow]",
            "gamma: - | 7 | [7]",
        ],
    )?;
    assert_rows(&forge, "at \"libs\" where tags[*] = 7", &["gamma: "])
}

fn words(first: u32, second: u32) -> TestValue {
    TestValue::Raw([first.to_le_bytes(), second.to_le_bytes()].concat())
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
fn a_path_reaching_too_many_elements_is_an_error_value() -> TestResult {
    let many = MAX_PATH_VALUES + 1;
    let tags = (0..many).map(|index| TestValue::Int32(i32::try_from(index).unwrap_or(0)));
    let bytes = schema()
        .record(
            TestRecord::new(9, "Station", "Station.Big", "libs/stations/big.xml")
                .value("tags", TestValue::Array(tags.collect())),
        )
        .build();
    let forge = DataForge::open(&bytes)?;

    assert_rows(
        &forge,
        "from Station select tags[*], tags[65536]",
        &[&format!(
            "big: error: the path reaches more than {MAX_PATH_VALUES} array elements | 65536"
        )],
    )?;
    assert_rows(&forge, "from Station where tags[*] = 1", &[])?;
    assert_rows(&forge, "from Station where not tags[*] = 1", &[])?;
    assert_rows(&forge, "from Station where tags[65536] = 65536", &["big: "])
}
