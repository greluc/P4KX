//! Parsing and printing Queries: the canonical form is stable, and no input makes the parser or
//! the error marker panic.

use std::error::Error;

use p4kx_dataforge::query::{MAX_QUERY_LEN, Name, Query, QueryError, QueryErrorKind, Text};

type TestResult = Result<(), Box<dyn Error>>;

/// A small deterministic generator, so a failure repeats with its seed.
struct Random(u64);

impl Random {
    const fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, bound: usize) -> usize {
        usize::try_from(self.next() % u64::try_from(bound).unwrap_or(1)).unwrap_or(0)
    }

    fn pick<'a>(&mut self, items: &[&'a str]) -> &'a str {
        items
            .get(self.below(items.len()))
            .copied()
            .unwrap_or_default()
    }

    fn chance(&mut self, percent: usize) -> bool {
        self.below(100) < percent
    }
}

const NAMES: [&str; 12] = [
    "mass",
    "Size",
    "_x1",
    "select",
    "FROM",
    "at",
    "Where",
    "a b",
    "Größe",
    "x.y",
    "\"q\"",
    "EntityClassDefinition",
];

const SPACES: [&str; 5] = [" ", "  ", "\n", "\t ", " \u{3000}"];

fn keyword(random: &mut Random, word: &str) -> String {
    match random.below(3) {
        0 => word.to_owned(),
        1 => word.to_ascii_uppercase(),
        _ => word
            .chars()
            .enumerate()
            .map(|(i, c)| if i == 0 { c.to_ascii_uppercase() } else { c })
            .collect(),
    }
}

fn name(random: &mut Random) -> String {
    let name = random.pick(&NAMES);
    let bare = name
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !["from", "at", "select", "where", "and", "or", "not"]
            .iter()
            .any(|keyword| keyword.eq_ignore_ascii_case(name));
    if bare && random.chance(70) {
        name.to_owned()
    } else {
        format!("`{name}`")
    }
}

/// Writes a random valid Query with random keyword case and whitespace.
fn random_query(random: &mut Random) -> String {
    let mut text = String::new();
    let has_from = random.chance(60);
    if has_from {
        text.push_str(&keyword(random, "from"));
        text.push_str(random.pick(&SPACES));
        text.push_str(&name(random));
    }
    if !has_from || random.chance(50) {
        let glob = random.pick(&["*/ships/*", "", "libs\\foundry", "?a*", "Größe ö"]);
        text.push_str(random.pick(&SPACES));
        text.push_str(&keyword(random, "at"));
        text.push_str(random.pick(&SPACES));
        text.push('"');
        text.push_str(glob);
        text.push('"');
    }
    if random.chance(70) {
        text.push_str(random.pick(&SPACES));
        text.push_str(&keyword(random, "select"));
        for path in 0..=random.below(3) {
            if path > 0 {
                text.push_str(random.pick(&SPACES));
                text.push(',');
            }
            text.push_str(random.pick(&SPACES));
            text.push_str(&name(random));
            for _ in 0..random.below(3) {
                text.push('.');
                text.push_str(&name(random));
            }
        }
    }
    text.push_str(random.pick(&SPACES));
    text
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
        assert_eq!(reparsed.select().len(), parsed.select().len(), "{text:?}");
        assert_eq!(
            reparsed.from().map(Name::as_str),
            parsed.from().map(Name::as_str),
            "{text:?}"
        );
        assert_eq!(
            reparsed.at().map(Text::as_str),
            parsed.at().map(Text::as_str),
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

#[test]
fn random_text_never_panics_and_errors_stay_inside_the_query() {
    let mut random = Random(0x2545_f491_4f6c_dd1d);
    let pieces = [
        "from", "at", "select", "where", "`", "\"", ".", ",", " ", "\n", "mass", "*", "ö", "\u{0}",
        "->", "[", "]", "😀", "\r\n", "\t", "x",
    ];
    for _ in 0..20_000 {
        let mut text = String::new();
        for _ in 0..random.below(12) {
            text.push_str(random.pick(&pieces));
        }
        if random.chance(30) {
            text = random_query(&mut random).replacen(' ', random.pick(&pieces), 1);
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
