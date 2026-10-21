//! Running `where` conditions over a synthetic DataForge: the comparison rules of every scalar type.

use crate::common::{TestResult, assert_selects, database};
use p4kx_dataforge::DataForge;

const FIRST_ID: &str = "00000000-0000-0000-0000-000000000001";
#[test]
fn integers_compare_by_value_across_the_whole_range() -> TestResult {
    let bytes = database();
    let forge = DataForge::open(&bytes)?;

    assert_selects(
        &forge,
        &[
            ("from Item where count > 3", &["a"]),
            ("from Item where count >= 3", &["a", "b"]),
            ("from Item where count < 0", &["c"]),
            ("from Item where count <= -7", &["c"]),
            ("from Item where count = 10", &["a"]),
            ("from Item where count != 10", &["b", "c"]),
            ("from Item where big = 18446744073709551615", &["a"]),
            ("from Item where big > 18446744073709551614", &["a"]),
            ("from Item where big > -1", &["a", "b", "c"]),
            (
                "from Item where count < 99999999999999999999",
                &["a", "b", "c"],
            ),
        ],
    )
}

#[test]
fn floats_compare_by_value_in_their_own_precision_and_nan_equals_nothing() -> TestResult {
    let bytes = database();
    let forge = DataForge::open(&bytes)?;

    assert_selects(
        &forge,
        &[
            ("from Item where mass = 0.1", &["a"]),
            ("from Item where mass = 1e-1", &["a"]),
            ("from Item where mass > 0", &["a", "b"]),
            ("from Item where mass >= -1", &["a", "b", "c"]),
            ("from Item where power = 0", &["c"]),
            ("from Item where power >= 0", &["a", "c"]),
            ("from Item where power < 2", &["a", "c"]),
            ("from Item where power != 1.5", &["b", "c"]),
            ("from Item where not power = 1.5", &["b", "c"]),
            ("from Item where not power < 2", &["b"]),
        ],
    )
}

#[test]
fn strings_compare_exactly_and_match_ignoring_ascii_case() -> TestResult {
    let bytes = database();
    let forge = DataForge::open(&bytes)?;

    assert_selects(
        &forge,
        &[
            ("from Item where name = \"Avenger\"", &["a"]),
            ("from Item where name = \"avenger\"", &[]),
            ("from Item where name = \"Say \"\"hi\"\"\"", &["c"]),
            ("from Item where name < \"B\"", &["a"]),
            ("from Item where name > \"Z\"", &["b"]),
            ("from Item where name contains \"VEN\"", &["a"]),
            ("from Item where name contains \"\"", &["a", "b", "c"]),
            ("from Item where name starts with \"A\"", &["a", "b"]),
            ("from Item where name like \"a*r?\"", &["b"]),
            ("from Item where name like \"*\"\"HI\"\"\"", &["c"]),
            ("from Item where name like \"a*\"", &["a", "b"]),
            ("from Item where label = \"@item_b\"", &["b"]),
            ("from Item where label like \"@ITEM_?\"", &["a", "b", "c"]),
        ],
    )
}

#[test]
fn enums_guids_and_bools_compare_by_name_text_and_order() -> TestResult {
    let bytes = database();
    let forge = DataForge::open(&bytes)?;

    assert_selects(
        &forge,
        &[
            ("from Item where size = \"Large\"", &["a"]),
            ("from Item where size != \"Large\"", &["b", "c"]),
            ("from Item where size = \"large\"", &[]),
            ("from Item where size < \"M\"", &["a"]),
            ("from Item where size starts with \"s\"", &["b", "c"]),
            (&format!("from Item where id = \"{FIRST_ID}\""), &["a"]),
            (
                &format!("from Item where id = \"{}\"", FIRST_ID.to_uppercase()),
                &["a"],
            ),
            (&format!("from Item where id > \"{FIRST_ID}\""), &["b", "c"]),
            (
                "from Item where id > \"f0000000-0000-0000-0000-000000000000\"",
                &["c"],
            ),
            ("from Item where active = true", &["a", "c"]),
            ("from Item where active != true", &["b"]),
            ("from Item where active < true", &["b"]),
        ],
    )
}
