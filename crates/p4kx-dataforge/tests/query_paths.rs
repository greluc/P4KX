//! Paths into nested data: dot paths through inline structs and strong pointers, `[n]`, `[*]`
//! and `[TypeName]` in `select` and `where`.

mod ships;

use std::error::Error;

use p4kx_dataforge::DataForge;
use ships::{database, rows};

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
