//! Parsing and printing Queries: the canonical form is stable, and no input makes the parser or
//! the error marker panic.

mod generator;

use std::error::Error;

use generator::{Random, random_query};
use p4kx_dataforge::query::{
    Condition, Glob, MAX_QUERY_LEN, Name, Path, Query, QueryError, QueryErrorKind, Segment,
};

type TestResult = Result<(), Box<dyn Error>>;

/// Writes the segments of a path by kind and value, without spans.
fn segments(path: &Path) -> String {
    let mut parts = vec![path.head().as_str().to_owned()];
    parts.extend(path.tail().iter().map(|segment| match segment {
        Segment::Field(name) => format!("field {}", name.as_str()),
        Segment::Index { index, .. } => format!("index {index}"),
        Segment::All(_) => "all".to_owned(),
        Segment::Type { name, .. } => format!("type {}", name.as_str()),
        Segment::Follow { name, .. } => format!("follow {}", name.as_str()),
        _ => "?".to_owned(),
    }));
    parts.join(" ")
}

/// Writes the structure of a condition with every group in parentheses.
fn shape(condition: &Condition) -> String {
    let joined = |parts: &[Condition], joint: &str| {
        let parts: Vec<String> = parts.iter().map(shape).collect();
        format!("({joint} {})", parts.join(" "))
    };
    match condition {
        Condition::Compare(comparison) => format!(
            "[{} {:?} {:?}]",
            segments(comparison.path()),
            comparison.operator(),
            comparison.literal().value()
        ),
        Condition::Not(inner) => format!("(not {})", shape(inner)),
        Condition::And(parts) => joined(parts, "and"),
        Condition::Or(parts) => joined(parts, "or"),
        _ => "?".to_owned(),
    }
}

#[test]
fn printing_and_reparsing_a_parsed_query_is_stable() -> TestResult {
    let mut random = Random(0x9e37_79b9_7f4a_7c15);
    for _ in 0..5_000 {
        let text = random_query(&mut random);
        let parsed = Query::parse(&text).map_err(|error| format!("{text:?}: {error}"))?;
        let printed = parsed.to_string();
        let reparsed = Query::parse(&printed).map_err(|error| format!("{printed:?}: {error}"))?;

        assert_eq!(reparsed.to_string(), printed, "stable for {text:?}");
        assert_eq!(
            reparsed.select().iter().map(segments).collect::<Vec<_>>(),
            parsed.select().iter().map(segments).collect::<Vec<_>>(),
            "{text:?}"
        );
        assert_eq!(
            reparsed.from().map(Name::as_str),
            parsed.from().map(Name::as_str),
            "{text:?}"
        );
        assert_eq!(
            reparsed.at().map(Glob::as_str),
            parsed.at().map(Glob::as_str),
            "{text:?}"
        );
        assert_eq!(
            reparsed.condition().map(shape),
            parsed.condition().map(shape),
            "{text:?}"
        );
    }
    Ok(())
}

#[test]
fn the_canonical_form_is_spelt_out() -> TestResult {
    let query = Query::parse("FROM  Ship\nAt \"*/ships/*\"  SELECT crew ,`select` . x,`a b`")?;

    assert_eq!(
        query.to_string(),
        "from Ship at \"*/ships/*\" select crew, `select`.x, `a b`",
        "canonical form"
    );
    Ok(())
}

#[test]
fn a_field_named_like_the_referencing_keyword_is_printed_in_backticks() -> TestResult {
    let text = "from Ship referencing \"a\" select `referencing`, REFERENCING2";
    let query = Query::parse(text)?;

    assert_eq!(
        query.to_string(),
        text,
        "the keyword is quoted as a name, a longer name is not"
    );
    assert!(
        Query::parse("from Ship select referencing").is_err(),
        "the bare keyword is no name"
    );
    Ok(())
}

#[test]
fn spans_point_at_names_and_paths() -> TestResult {
    let text = "from Ship select crew.x, mass";
    let query = Query::parse(text)?;
    let spans: Vec<_> = query
        .select()
        .iter()
        .map(|path| path.span().range())
        .collect();

    assert_eq!(
        query.from().map(|name| name.span().range()),
        Some(5..9),
        "type"
    );
    assert_eq!(spans, [17..23, 25..29], "paths");
    Ok(())
}

/// Pieces random text for the parser is made of.
const PIECES: [&str; 49] = [
    "from", "at", "select", "where", "`", "\"", ".", ",", " ", "\n", "mass", "*", "ö", "\u{0}",
    "->", "[", "]", "😀", "\r\n", "\t", "x", "=", "!", "!=", "<", "<=", ">=", "(", ")", "-",
    "-1.5e3", "1e", "1.", "and", "or", "not", "contains", "starts", "with", "like", "true", "\"\"",
    "0", "[*]", "[0]", "[Item]", "[-1]", "[`a`]", ".x[",
];

#[test]
fn random_text_never_panics_and_errors_stay_inside_the_query() {
    let mut random = Random(0x2545_f491_4f6c_dd1d);
    for _ in 0..20_000 {
        let mut text = String::new();
        for _ in 0..random.below(12) {
            text.push_str(random.pick(&PIECES));
        }
        if random.chance(30) {
            text = random_query(&mut random).replacen(' ', random.pick(&PIECES), 1);
        }
        if let Err(error) = Query::parse(&text) {
            let span = error.span();
            assert!(span.start() <= span.end(), "{text:?}: {span:?}");
            assert!(span.end() <= text.len(), "{text:?}: {span:?}");
            assert!(text.is_char_boundary(span.start()), "{text:?}: {span:?}");
            assert!(text.is_char_boundary(span.end()), "{text:?}: {span:?}");
            let marked = error.marked(&text).to_string();
            assert!(marked.contains('^'), "{text:?}: {marked}");
        }
    }
}

#[test]
fn arbitrary_characters_never_panic() {
    let mut random = Random(0xdead_beef_cafe_f00d);
    for _ in 0..20_000 {
        let text: String = (0..random.below(24))
            .filter_map(|_| {
                let range = match random.below(4) {
                    0 => 0_usize..0x80,
                    1 => 0x80..0x800,
                    2 => 0x800..0x1_0000,
                    _ => 0x1_0000..0x11_0000,
                };
                let span = range.end - range.start;
                let code = u32::try_from(range.start + random.below(span)).ok()?;
                char::from_u32(code)
            })
            .collect();
        if let Err(error) = Query::parse(&text) {
            assert!(error.span().end() <= text.len(), "{text:?}: {error:?}");
            assert!(
                text.is_char_boundary(error.span().start()),
                "{text:?}: {error:?}"
            );
            assert!(!error.marked(&text).to_string().is_empty(), "{text:?}");
        }
    }
}

#[test]
fn a_marker_survives_a_span_that_does_not_fit_the_text() -> TestResult {
    let Err(error) = Query::parse("from `") else {
        return Err("an unterminated name is an error".into());
    };

    let marked = error.marked("x").to_string();

    assert!(marked.ends_with("  ^"), "{marked}");
    Ok(())
}

#[test]
fn an_overlong_query_is_rejected_before_parsing() {
    let text = format!("at \"{}\"", "a".repeat(MAX_QUERY_LEN));

    let error = Query::parse(&text).err();

    assert!(
        matches!(
            error.as_ref().map(QueryError::kind),
            Some(QueryErrorKind::TooLong { .. })
        ),
        "{error:?}"
    );
}
