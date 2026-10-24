// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Parsing paths with `[n]`, `[*]` and `[TypeName]`: their canonical form, spans and errors.

use std::error::Error;

use p4kx_dataforge::query::{
    Expected, MAX_PATH_SEGMENTS, Path, Query, QueryError, QueryErrorKind, Segment,
};

type TestResult = Result<(), Box<dyn Error>>;

fn printed(text: &str) -> Result<String, Box<dyn Error>> {
    Ok(Query::parse(text)?.to_string())
}

fn kind(text: &str) -> Option<QueryErrorKind> {
    Query::parse(text)
        .err()
        .as_ref()
        .map(QueryError::kind)
        .cloned()
}

#[test]
fn brackets_print_without_spaces_after_the_field() -> TestResult {
    assert_eq!(
        printed("from Ship select Components [ Weapon ] . damage , tags[ 0 ],tags [*]")?,
        "from Ship select Components[Weapon].damage, tags[0], tags[*]",
    );
    assert_eq!(
        printed("from Ship select a[`select`].b[007][*] where a[*].`x y` > 1")?,
        "from Ship select a[`select`].b[7][*] where a[*].`x y` > 1",
    );
    Ok(())
}

#[test]
fn segments_carry_their_kind_and_span() -> TestResult {
    let query = Query::parse("from Ship select parts[2][*][Gun].mass")?;
    let Some(path) = query.select().first() else {
        return Err("one path".into());
    };
    let segments: Vec<String> = path
        .tail()
        .iter()
        .map(|segment| {
            let range = segment.span().range();
            match segment {
                Segment::Index { index, .. } => format!("index {index} {range:?}"),
                Segment::All(_) => format!("all {range:?}"),
                Segment::Type { name, .. } => format!("type {} {range:?}", name.as_str()),
                Segment::Field(name) => format!("field {} {range:?}", name.as_str()),
                _ => "?".to_owned(),
            }
        })
        .collect();

    assert_eq!(
        segments,
        [
            "index 2 22..25",
            "all 25..28",
            "type Gun 28..33",
            "field mass 34..38"
        ],
    );
    assert_eq!(path.span().range(), 17..38, "the whole path");
    assert!(path.is_multi_valued(), "[*] and [Gun] select several");
    Ok(())
}

#[test]
fn a_path_with_fields_and_indices_only_is_single_valued() -> TestResult {
    let query = Query::parse("from Ship select parts[2].mass, parts[*].mass, parts[Gun]")?;
    let multi: Vec<bool> = query.select().iter().map(Path::is_multi_valued).collect();

    assert_eq!(multi, [false, true, true]);
    Ok(())
}

#[test]
fn an_index_is_a_whole_number_that_fits_32_bits() {
    for text in ["-1", "1.5", "2e3", "4294967296", "99999999999999999999999"] {
        let query = format!("from Ship select parts[{text}]");
        let error = Query::parse(&query).err();
        assert!(
            matches!(
                error.as_ref().map(QueryError::kind),
                Some(QueryErrorKind::InvalidIndex)
            ),
            "{query}: {error:?}"
        );
        assert_eq!(
            error.map(|error| error.span().range()),
            Some(23..23 + text.len()),
            "{query}"
        );
    }
    assert!(Query::parse("from Ship select parts[4294967295]").is_ok());
}

#[test]
fn brackets_need_an_index_a_star_or_a_type_and_a_closing_bracket() {
    assert_eq!(
        kind("from Ship select parts[]"),
        Some(QueryErrorKind::Expected {
            expected: Expected::Index,
            found: p4kx_dataforge::query::Found::Symbol("]"),
        })
    );
    assert_eq!(
        kind("from Ship select parts[1"),
        Some(QueryErrorKind::Expected {
            expected: Expected::CloseBracket,
            found: p4kx_dataforge::query::Found::End,
        })
    );
    assert_eq!(
        kind("from Ship select [1]"),
        Some(QueryErrorKind::Expected {
            expected: Expected::FieldName,
            found: p4kx_dataforge::query::Found::Symbol("["),
        })
    );
    assert!(matches!(
        kind("from Ship select parts.[1]"),
        Some(QueryErrorKind::Expected {
            expected: Expected::FieldName,
            ..
        })
    ));
    assert!(matches!(
        kind("from Ship select parts[select]"),
        Some(QueryErrorKind::Expected {
            expected: Expected::Index,
            ..
        })
    ));
}

#[test]
fn a_path_has_a_bounded_number_of_segments() {
    let fits = format!("from Ship select a{}", "[*]".repeat(MAX_PATH_SEGMENTS));
    assert!(Query::parse(&fits).is_ok(), "{MAX_PATH_SEGMENTS} segments");

    let text = format!("from Ship select a{}.b", "[*]".repeat(MAX_PATH_SEGMENTS));
    let error = Query::parse(&text).err();

    assert_eq!(
        error.as_ref().map(QueryError::kind),
        Some(&QueryErrorKind::PathTooLong {
            limit: MAX_PATH_SEGMENTS
        }),
    );
    assert_eq!(
        error.map(|error| error.span().range()),
        Some(text.len() - 1..text.len()),
        "the first segment too many"
    );
}
