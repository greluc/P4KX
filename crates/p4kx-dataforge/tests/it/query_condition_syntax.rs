//! Parsing and printing `where` conditions: precedence, word operators, literals and limits.

use std::error::Error;

use p4kx_dataforge::query::{
    Condition, Expected, Glob, MAX_CONDITION_DEPTH, Operator, Query, QueryError, QueryErrorKind,
};

type TestResult = Result<(), Box<dyn Error>>;

fn canonical(text: &str) -> Result<String, Box<dyn Error>> {
    Ok(Query::parse(text)?.to_string())
}

fn kind(text: &str) -> Option<QueryErrorKind> {
    Query::parse(text).err().map(|error| error.kind().clone())
}

#[test]
fn a_condition_prints_in_canonical_form() -> TestResult {
    assert_eq!(
        canonical("FROM Weapon SELECT damage WHERE damage>1000")?,
        "from Weapon select damage where damage > 1000",
        "canonical form"
    );
    assert_eq!(
        canonical("at \"*\" where a=-1.5e3 And b!=\"x\" or not c<=true")?,
        "at \"*\" where a = -1.5e3 and b != \"x\" or not c <= true",
        "operators and literals"
    );
    Ok(())
}

#[test]
fn and_binds_tighter_than_or_and_not_tighter_than_and() -> TestResult {
    let query = Query::parse("from T where a = 1 or b = 2 and not c = 3")?;

    let Some(Condition::Or(terms)) = query.condition() else {
        return Err(format!("an `or` at the top: {:?}", query.condition()).into());
    };
    assert!(
        matches!(terms.as_slice(), [Condition::Compare(_), Condition::And(factors)]
            if matches!(factors.as_slice(), [Condition::Compare(_), Condition::Not(_)])),
        "{terms:?}"
    );
    Ok(())
}

#[test]
fn parentheses_print_only_where_they_are_needed() -> TestResult {
    assert_eq!(
        canonical("from T where ((a = 1)) and (b = 2 or c = 3) and not (d = 4 and e = 5)")?,
        "from T where a = 1 and (b = 2 or c = 3) and not (d = 4 and e = 5)",
        "minimal parentheses"
    );
    assert_eq!(
        canonical("from T where a = 1 and (b = 2 and c = 3)")?,
        "from T where a = 1 and b = 2 and c = 3",
        "a nested `and` is flattened"
    );
    Ok(())
}

#[test]
fn word_operators_ignore_case_and_stay_field_names_elsewhere() -> TestResult {
    let query = Query::parse(
        "from T select contains, like where name CONTAINS \"x\" and `like` Starts With \"a\" \
         and starts like \"*a?\"",
    )?;

    assert_eq!(
        query.to_string(),
        "from T select contains, like where name contains \"x\" and like starts with \"a\" \
         and starts like \"*a?\"",
        "word operators"
    );
    let operators: Vec<Operator> = match query.condition() {
        Some(Condition::And(factors)) => factors
            .iter()
            .filter_map(|factor| match factor {
                Condition::Compare(comparison) => Some(comparison.operator()),
                _ => None,
            })
            .collect(),
        other => return Err(format!("an `and`: {other:?}").into()),
    };
    assert_eq!(
        operators,
        [Operator::Contains, Operator::StartsWith, Operator::Like],
        "operators"
    );
    Ok(())
}

#[test]
fn a_quoted_word_is_no_operator() {
    assert!(
        matches!(
            kind("from T where name `contains` \"x\""),
            Some(QueryErrorKind::Expected {
                expected: Expected::Operator,
                ..
            })
        ),
        "a quoted name is a name"
    );
}

#[test]
fn a_doubled_quote_stands_for_one_quote() -> TestResult {
    let query = Query::parse(r#"at "a""b\c" where name = """quoted""""#)?;

    assert_eq!(query.at().map(Glob::as_str), Some(r#"a"b\c"#), "glob");
    assert_eq!(
        query.to_string(),
        r#"at "a""b\c" where name = """quoted""""#,
        "printed doubled"
    );
    Ok(())
}

#[test]
fn true_and_false_are_literals_only_after_an_operator() -> TestResult {
    assert_eq!(
        canonical("from T where true = TRUE and false != False")?,
        "from T where true = true and false != false",
        "fields named true and false"
    );
    Ok(())
}

#[test]
fn a_comparison_needs_a_value() {
    assert!(
        matches!(
            kind("from T where a = b"),
            Some(QueryErrorKind::Expected {
                expected: Expected::Value,
                ..
            })
        ),
        "a name is no value"
    );
    assert!(
        matches!(
            kind("from T where a >"),
            Some(QueryErrorKind::Expected {
                expected: Expected::Value,
                ..
            })
        ),
        "the end is no value"
    );
    assert!(
        matches!(
            kind("from T where a contains 1"),
            Some(QueryErrorKind::Expected {
                expected: Expected::Text,
                ..
            })
        ),
        "matching needs a string"
    );
}

#[test]
fn condition_errors_carry_their_span() -> TestResult {
    let cases = [
        ("from T where", 12..12),
        ("from T where (a = 1", 19..19),
        ("from T where a = 1)", 18..19),
        ("from T where a = 1 b = 2", 19..20),
        ("from T where a ! 1", 15..16),
        ("from T where a = 1 where b = 2", 19..24),
        ("from T where a = 1 select b", 19..25),
        ("from T where a starts \"x\"", 22..25),
        ("from T where a = 1.", 18..19),
        ("from T where a = -", 17..18),
    ];
    for (text, span) in cases {
        let error = Query::parse(text).err().ok_or(text)?;
        assert_eq!(error.span().range(), span, "{text}: {error}");
    }
    Ok(())
}

#[test]
fn nesting_deeper_than_the_limit_is_rejected() -> TestResult {
    let depth = MAX_CONDITION_DEPTH;
    let fits = format!(
        "from T where {}a = 1{}",
        "(".repeat(depth),
        ")".repeat(depth)
    );
    let deep = format!("from T where {}a = 1", "not ".repeat(depth + 1));
    let parens = format!("from T where {}", "(".repeat(60_000));

    Query::parse(&fits)?;
    for text in [deep, parens] {
        let error = Query::parse(&text).err();
        assert!(
            matches!(
                error.as_ref().map(QueryError::kind),
                Some(QueryErrorKind::ConditionTooDeep { .. })
            ),
            "{error:?}"
        );
    }
    Ok(())
}
