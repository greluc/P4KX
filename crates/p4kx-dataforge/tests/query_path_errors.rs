//! Paths that do not fit the schema are compile errors with the span of their cause.

mod ships;

use std::error::Error;

use p4kx_dataforge::DataForge;
use p4kx_dataforge::query::Query;
use ships::{database, rows};

type TestResult = Result<(), Box<dyn Error>>;

/// Returns the message and the marked text of the error `text` compiles to.
fn error(forge: &DataForge<'_>, text: &str) -> Result<(String, String), Box<dyn Error>> {
    let query = Query::parse(text)?;
    let Err(error) = query.compile(forge) else {
        return Err(format!("{text} compiles").into());
    };
    let marked = text
        .get(error.span().range())
        .unwrap_or_default()
        .to_owned();
    Ok((error.to_string(), marked))
}

#[test]
fn unknown_field_and_type_names_are_marked_with_a_suggestion() -> TestResult {
    let bytes = database();
    let forge = DataForge::open(&bytes)?;
    let cases = [
        (
            "from Ship select position.z",
            "unknown field `z` of `Vector`; did you mean `x`?",
            "z",
        ),
        (
            "from Ship select Components[Weapn].damage",
            "unknown type `Weapn`; did you mean `Weapon`?",
            "Weapn",
        ),
        (
            "from Ship select Components[Weapon].hp",
            "unknown field `hp` of `Weapon`",
            "hp",
        ),
        (
            "from Ship select Components[0].damage",
            "unknown field `damage` of `Component`",
            "damage",
        ),
        (
            "at \"libs\" select anchor.z",
            "unknown field `z` of `Vector`; did you mean `x`?",
            "z",
        ),
    ];
    for (text, message, marked) in cases {
        assert_eq!(
            error(&forge, text)?,
            (message.to_owned(), marked.to_owned()),
            "{text}"
        );
    }
    assert_eq!(
        rows(&forge, "from Ship select Components[Weapon].name")?.len(),
        2,
        "the names as declared compile"
    );
    Ok(())
}

#[test]
fn a_segment_that_does_not_fit_the_value_is_marked() -> TestResult {
    let bytes = database();
    let forge = DataForge::open(&bytes)?;
    let cases = [
        (
            "from Ship select Components.name",
            "field `Components` of `Ship` holds an array of strong pointers; select its elements \
             with `[n]`, `[*]` or `[TypeName]` before the field `name`",
            "name",
        ),
        (
            "from Ship select mass[0]",
            "field `mass` of `Ship` holds a Single value, which has no elements",
            "[0]",
        ),
        (
            "from Ship select position[*]",
            "field `position` of `Ship` holds a struct, which has no elements",
            "[*]",
        ),
        (
            "from Ship select tags[Weapon]",
            "field `tags` of `Ship` holds an array of String values; `[TypeName]` selects \
             elements of struct arrays only",
            "[Weapon]",
        ),
        (
            "from Ship select hardpoints[Weapon].x",
            "the elements of field `hardpoints` are `Vector`, and `Weapon` neither derives \
             from it nor is a base of it",
            "Weapon",
        ),
        (
            "from Ship select links[0].name",
            "field `links[0]` of `Ship` holds a Reference; follow it with `->name` to reach the \
             field `name` of what it links to",
            "name",
        ),
        (
            "from Ship select tags[0].x",
            "field `tags[0]` of `Ship` holds a String value and has no field `x`",
            "x",
        ),
    ];
    for (text, message, marked) in cases {
        assert_eq!(
            error(&forge, text)?,
            (message.to_owned(), marked.to_owned()),
            "{text}"
        );
    }
    Ok(())
}

#[test]
fn a_path_has_to_end_at_a_scalar() -> TestResult {
    let bytes = database();
    let forge = DataForge::open(&bytes)?;
    let cases = [
        (
            "from Ship select position",
            "field `position` of `Ship` holds a struct; a path has to end at a scalar value",
            "position",
        ),
        (
            "from Ship select Components[Weapon]",
            "field `Components[Weapon]` of `Ship` holds a strong pointer; a path has to end at \
             a scalar value",
            "[Weapon]",
        ),
        (
            "from Ship select tags",
            "field `tags` of `Ship` holds an array of String values; a path has to end at a \
             scalar value",
            "tags",
        ),
        (
            "from Ship select links[*]",
            "field `links[*]` of `Ship` holds a Reference; a path has to end at a scalar value",
            "[*]",
        ),
    ];
    for (text, message, marked) in cases {
        assert_eq!(
            error(&forge, text)?,
            (message.to_owned(), marked.to_owned()),
            "{text}"
        );
    }
    Ok(())
}

#[test]
fn a_comparison_names_the_whole_path_it_cannot_compare() -> TestResult {
    let bytes = database();
    let forge = DataForge::open(&bytes)?;

    assert_eq!(
        error(&forge, "from Ship where Components[*].name > 5")?,
        (
            "`>` cannot compare field `Components[*].name` of `Ship`, which holds String \
             values, with an integer"
                .to_owned(),
            "Components[*].name > 5".to_owned()
        ),
    );
    Ok(())
}
