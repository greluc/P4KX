//! Running `where` conditions over a synthetic DataForge: precedence, Kleene logic for missing
//! values, failed Records and the compile errors of comparisons.

mod common;

use std::error::Error;

use common::{ITEMS, TestResult, assert_selects, database, names};
use p4kx_dataforge::DataForge;
use p4kx_dataforge::query::{LiteralKind, Query, QueryErrorKind};
fn compile_error(forge: &DataForge<'_>, text: &str) -> Result<QueryErrorKind, Box<dyn Error>> {
    match Query::parse(text)?.compile(forge) {
        Ok(_) => Err(format!("{text} compiles").into()),
        Err(error) => Ok(error.kind().clone()),
    }
}

#[test]
fn and_binds_tighter_than_or_and_not_negates() -> TestResult {
    let bytes = database();
    let forge = DataForge::open(&bytes)?;

    assert_selects(
        &forge,
        &[
            (
                "from Item where (count = 10 or count = 3) and active = true",
                &["a"],
            ),
            (
                "from Item where count = 10 or count = 3 and active = false",
                &["a", "b"],
            ),
            ("from Item where not (count = 10 or count = 3)", &["c"]),
            ("from Item where not not count = 3", &["b"]),
        ],
    )
}

#[test]
fn a_missing_or_incomparable_value_never_matches() -> TestResult {
    let bytes = database();
    let forge = DataForge::open(&bytes)?;

    assert_selects(
        &forge,
        &[
            ("at \"libs\" where name = \"Avenger\"", &["x!", "a"]),
            ("at \"libs\" where name != \"Avenger\"", &["x!", "b", "c"]),
            (
                "at \"libs\" where not name = \"Avenger\"",
                &["x!", "b", "c"],
            ),
            ("at \"libs\" where name = 42", &["x!", "g"]),
            (
                "at \"libs\" where active = true or name = 42",
                &["x!", "g", "a", "c"],
            ),
            (
                "at \"libs\" where active = false or not name = 42",
                &["x!", "b"],
            ),
            (
                "at \"libs\" where not (active = true and mass > 0)",
                &["x!", "b", "c"],
            ),
        ],
    )
}

#[test]
fn a_record_whose_values_cannot_be_read_is_reported_as_failed() -> TestResult {
    let bytes = database();
    let forge = DataForge::open(&bytes)?;

    assert_selects(
        &forge,
        &[
            ("at \"libs\" where count > 5", &["x!", "a"]),
            ("at \"*/broken/*\" where count = 0", &["x!"]),
        ],
    )
}

#[test]
fn incompatible_comparisons_are_compile_errors_with_their_span() -> TestResult {
    let bytes = database();
    let forge = DataForge::open(&bytes)?;
    let cases = [
        ("from Item where count = \"5\"", LiteralKind::Text),
        ("from Item where count = 1.5", LiteralKind::Fraction),
        ("from Item where mass = true", LiteralKind::Bool),
        ("from Item where active = 1", LiteralKind::Integer),
        ("from Item where id < 1", LiteralKind::Integer),
        ("from Item where mass contains \"1\"", LiteralKind::Text),
        ("from Item where id like \"*\"", LiteralKind::Text),
        ("at \"libs\" where name = true", LiteralKind::Bool),
    ];
    for (text, kind) in cases {
        let error = Query::parse(text)?.compile(&forge).err().ok_or(text)?;
        assert_eq!(error.span().range(), 16..text.len(), "{text}: {error}");
        assert!(
            matches!(error.kind(), QueryErrorKind::IncompatibleComparison { literal, .. }
                if *literal == kind),
            "{text}: {error:?}"
        );
    }
    Ok(())
}
#[test]
fn literals_that_do_not_fit_the_field_are_marked() -> TestResult {
    let bytes = database();
    let forge = DataForge::open(&bytes)?;

    assert!(
        matches!(
            compile_error(&forge, "from Item where mass > 1e39")?,
            QueryErrorKind::NumberOutOfRange(_)
        ),
        "beyond f32"
    );
    assert!(
        matches!(
            compile_error(&forge, "from Item where power > 1e999")?,
            QueryErrorKind::NumberOutOfRange(_)
        ),
        "beyond f64"
    );
    assert!(
        matches!(
            compile_error(
                &forge,
                &format!("from Item where count = {}", "9".repeat(40))
            )?,
            QueryErrorKind::NumberOutOfRange(_)
        ),
        "beyond every integer"
    );
    assert!(
        matches!(
            compile_error(&forge, "from Item where id = \"1-2\"")?,
            QueryErrorKind::InvalidGuid
        ),
        "no GUID"
    );
    assert!(
        matches!(
            compile_error(&forge, "from Item where weight > 1")?,
            QueryErrorKind::UnknownField { .. }
        ),
        "unknown field"
    );
    Ok(())
}

#[test]
fn a_comparison_and_its_negation_split_every_record_with_the_value() -> TestResult {
    let bytes = database();
    let forge = DataForge::open(&bytes)?;
    let comparisons = [
        "count > 3",
        "mass <= 0.1",
        "power = 0",
        "power != 1.5",
        "name like \"*a*\"",
        "size >= \"Small\"",
        "active = false",
        "big < 2",
        "label contains \"_c\"",
    ];
    for left in comparisons {
        for right in comparisons {
            for joint in ["and", "or"] {
                let condition = format!("{left} {joint} {right}");
                let kept = names(&forge, &format!("from Item where {condition}"))?;
                let dropped = names(&forge, &format!("from Item where not ({condition})"))?;
                assert_eq!(kept.len() + dropped.len(), ITEMS.len(), "{condition}");
                assert!(
                    kept.iter().all(|name| !dropped.contains(name)),
                    "{condition}"
                );
            }
        }
    }
    Ok(())
}
