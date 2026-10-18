//! Parsing and printing Queries: the canonical form is stable, and no input makes the parser or
//! the error marker panic.

use std::error::Error;

use p4kx_dataforge::query::{
    Condition, Glob, MAX_QUERY_LEN, Name, Path, Query, QueryError, QueryErrorKind, Segment,
};

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

/// Writes a random path of up to four segments: dotted fields, indices, `[*]` and types, with
/// random whitespace around the brackets and dots.
fn path(random: &mut Random) -> String {
    let mut text = name(random);
    for _ in 0..random.below(5) {
        let gap = if random.chance(20) {
            random.pick(&SPACES)
        } else {
            ""
        };
        text.push_str(gap);
        let inner = match random.below(4) {
            0 => {
                text.push('.');
                text.push_str(gap);
                text.push_str(&name(random));
                continue;
            }
            1 => random.pick(&["0", "7", "007", "4294967295"]).to_owned(),
            2 => "*".to_owned(),
            _ => name(random),
        };
        for part in ["[", gap, &inner, gap, "]"] {
            text.push_str(part);
        }
    }
    text
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
        let glob = random.pick(&["*/ships/*", "", "libs\\foundry", "?a*", "Größe ö", "a\"\"b"]);
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
        for column in 0..=random.below(3) {
            if column > 0 {
                text.push_str(random.pick(&SPACES));
                text.push(',');
            }
            text.push_str(random.pick(&SPACES));
            text.push_str(&path(random));
        }
    }
    if random.chance(60) {
        text.push_str(random.pick(&SPACES));
        text.push_str(&keyword(random, "where"));
        text.push_str(random.pick(&SPACES));
        text.push_str(&random_condition(random, 0));
    }
    text.push_str(random.pick(&SPACES));
    text
}

/// Writes a random valid condition nested at most three levels.
fn random_condition(random: &mut Random, depth: usize) -> String {
    let space = |random: &mut Random| random.pick(&SPACES).to_owned();
    let mut text = String::new();
    for part in 0..=random.below(3) {
        if part > 0 {
            let joint = if random.chance(50) { "and" } else { "or" };
            text += &space(random);
            text += &keyword(random, joint);
            text += &space(random);
        }
        if random.chance(20) {
            text += &keyword(random, "not");
            text += &space(random);
        }
        if depth < 3 && random.chance(25) {
            text.push('(');
            text += &random_condition(random, depth + 1);
            text.push(')');
            continue;
        }
        text += &path(random);
        let operator = random.pick(&[
            "=",
            "!=",
            "<",
            "<=",
            ">",
            ">=",
            "contains",
            "starts with",
            "like",
        ]);
        let literal = if ["contains", "starts with", "like"].contains(&operator) {
            random.pick(&["\"x\"", "\"\"", "\"a\"\"b\"", "\"*?\""])
        } else {
            random.pick(&["0", "-12", "1.5", "2e-3", "1E+9", "\"s\"", "true", "FALSE"])
        };
        let operator = keyword(random, operator).replace(' ', &space(random));
        text += &space(random);
        text += &operator;
        text += &space(random);
        text += literal;
    }
    text
}

/// Writes the segments of a path by kind and value, without spans.
fn segments(path: &Path) -> String {
    let mut parts = vec![path.head().as_str().to_owned()];
    parts.extend(path.tail().iter().map(|segment| match segment {
        Segment::Field(name) => format!("field {}", name.as_str()),
        Segment::Index { index, .. } => format!("index {index}"),
        Segment::All(_) => "all".to_owned(),
        Segment::Type { name, .. } => format!("type {}", name.as_str()),
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
