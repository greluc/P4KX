use std::fmt;

use thiserror::Error;

use crate::query::lexer::Keyword;
use crate::query::parser::floor_char_boundary;
use crate::query::{MAX_QUERY_LEN, Operator, Span};
use crate::{DataForgeError, DataType};

/// A Query that cannot be parsed or compiled, with the span of the cause in the Query text.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{kind}")]
pub struct QueryError {
    kind: QueryErrorKind,
    span: Span,
}

impl QueryError {
    pub(crate) const fn new(kind: QueryErrorKind, span: Span) -> Self {
        Self { kind, span }
    }

    /// Returns what is wrong.
    #[must_use]
    pub const fn kind(&self) -> &QueryErrorKind {
        &self.kind
    }

    /// Returns where in the Query text the cause stands.
    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }

    /// Returns the message followed by the line of `query` holding the span and a line of `^`
    /// under the span; `query` is the text the error was reported for.
    #[must_use]
    pub const fn marked<'e>(&'e self, query: &'e str) -> Marked<'e> {
        Marked { error: self, query }
    }
}

/// Why a Query cannot be parsed or compiled.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum QueryErrorKind {
    /// The Query is longer than [`MAX_QUERY_LEN`] bytes.
    #[error("the Query is {len} bytes long, more than {MAX_QUERY_LEN}")]
    TooLong {
        /// The length in bytes.
        len: usize,
    },
    /// A character that starts no token.
    #[error("unexpected character `{}`", .0.escape_debug())]
    UnexpectedCharacter(char),
    /// A string without its closing `"`.
    #[error("the string has no closing `\"`")]
    UnterminatedString,
    /// A quoted name without its closing backtick.
    #[error("the quoted name has no closing backtick")]
    UnterminatedName,
    /// A pair of backticks with nothing between them.
    #[error("a quoted name cannot be empty")]
    EmptyName,
    /// A token that cannot stand where it stands.
    #[error("expected {expected}, found {found}")]
    Expected {
        /// What may stand there.
        expected: Expected,
        /// What stands there.
        found: Found,
    },
    /// A clause given twice.
    #[error("`{0}` is given twice")]
    DuplicateClause(Keyword),
    /// A clause after a clause that has to follow it.
    #[error("`{clause}` has to come before `{after}`")]
    ClauseOutOfOrder {
        /// The misplaced clause.
        clause: Keyword,
        /// The clause it has to precede.
        after: Keyword,
    },
    /// Neither `from` nor `at` is given.
    #[error("the Query selects no Records; start it with `from <type>` or `at \"<glob>\"`")]
    MissingSelection,
    /// `from` names a struct the DataForge does not declare.
    #[error("unknown type `{name}`{}", Suggestion(.suggestion.as_deref()))]
    UnknownType {
        /// The name as written.
        name: String,
        /// The closest declared struct name.
        suggestion: Option<String>,
    },
    /// A path names a field that its struct, or without `from` any struct, does not declare.
    #[error("unknown field `{name}`{}{}", Owner(.owner.as_deref()), Suggestion(.suggestion.as_deref()))]
    UnknownField {
        /// The name as written.
        name: String,
        /// The struct searched, or `None` when every struct was.
        owner: Option<String>,
        /// The closest declared field name.
        suggestion: Option<String>,
    },
    /// A path names a field that holds no single scalar value.
    #[error(
        "field `{name}`{} holds {}; only fields with one scalar value can be selected",
        Owner(.owner.as_deref()), Holds { data_type: *.data_type, array: *.array }
    )]
    UnsupportedField {
        /// The field name.
        name: String,
        /// The struct declaring it, or `None` without `from`.
        owner: Option<String>,
        /// The type of its values.
        data_type: DataType,
        /// Whether it holds an array.
        array: bool,
    },
    /// A path continues behind a field that holds a scalar value.
    #[error("field `{name}`{} holds a {data_type} value and has no field `{next}`", Owner(.owner.as_deref()))]
    ScalarHasNoFields {
        /// The scalar field.
        name: String,
        /// The struct declaring it, or `None` without `from`.
        owner: Option<String>,
        /// The type of its value.
        data_type: DataType,
        /// The name written behind it.
        next: String,
    },
    /// Parentheses and `not` nest deeper than [`MAX_CONDITION_DEPTH`](crate::query::MAX_CONDITION_DEPTH).
    #[error("the condition nests parentheses and `not` deeper than {limit}")]
    ConditionTooDeep {
        /// The deepest nesting allowed.
        limit: usize,
    },
    /// A comparison whose operator cannot compare the field's values with the literal.
    #[error(
        "`{operator}` cannot compare field `{name}`{}, which holds {data_type} values, with {literal}",
        Owner(.owner.as_deref())
    )]
    IncompatibleComparison {
        /// The compared field.
        name: String,
        /// The struct declaring it, or `None` without `from`.
        owner: Option<String>,
        /// The type of its values.
        data_type: DataType,
        /// The operator.
        operator: Operator,
        /// What the literal is.
        literal: LiteralKind,
    },
    /// A number that has no value in the compared type.
    #[error("the number is out of the range of {0} values")]
    NumberOutOfRange(DataType),
    /// A string compared with a GUID field that is no GUID.
    #[error("the string is not a GUID of the form xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx")]
    InvalidGuid,
    /// The schema of the DataForge cannot be read.
    #[error("the DataForge schema cannot be read: {0}")]
    DataForge(DataForgeError),
}

/// What the parser expected at a position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Expected {
    /// `from` or `at` at the start.
    Selection,
    /// A struct name after `from`.
    TypeName,
    /// A string after `at`.
    Glob,
    /// A field name in a path.
    FieldName,
    /// What may follow `from <type>`.
    AfterFrom,
    /// What may follow `at "<glob>"`.
    AfterAt,
    /// What may follow a path.
    AfterPath,
    /// A comparison, `not` or `(` where a condition starts.
    Condition,
    /// A comparison operator after the path of a comparison.
    Operator,
    /// `with` after `starts`.
    With,
    /// A number, a string, `true` or `false` after a comparison operator.
    Value,
    /// A string after a matching operator.
    Text,
    /// What may follow a condition in parentheses.
    CloseParenthesis,
    /// What may follow the condition of `where`.
    AfterCondition,
}

impl fmt::Display for Expected {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Selection => "`from` or `at`",
            Self::TypeName => "a type name",
            Self::Glob => "a record-path glob in double quotes",
            Self::FieldName => "a field name",
            Self::AfterFrom => "`at`, `select`, `where` or the end of the Query",
            Self::AfterAt => "`select`, `where` or the end of the Query",
            Self::AfterPath => "`.`, `,`, `where` or the end of the Query",
            Self::Condition => "a comparison, `not` or `(`",
            Self::Operator => {
                "`=`, `!=`, `<`, `<=`, `>`, `>=`, `contains`, `starts with` or `like`"
            }
            Self::With => "`with`",
            Self::Value => "a number, a string, `true` or `false`",
            Self::Text => "a string in double quotes",
            Self::CloseParenthesis => "`and`, `or` or `)`",
            Self::AfterCondition => "`and`, `or` or the end of the Query",
        })
    }
}

/// What the literal of a comparison is, for [`QueryErrorKind::IncompatibleComparison`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum LiteralKind {
    /// A number without fraction or exponent.
    Integer,
    /// A number with a fraction or an exponent.
    Fraction,
    /// A string.
    Text,
    /// `true` or `false`.
    Bool,
}

impl fmt::Display for LiteralKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Integer => "an integer",
            Self::Fraction => "a number with a fraction or exponent",
            Self::Text => "a string",
            Self::Bool => "a boolean",
        })
    }
}

/// The token the parser found instead of an expected one.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Found {
    /// The end of the Query.
    End,
    /// A keyword.
    Keyword(Keyword),
    /// A name.
    Name(String),
    /// A string.
    Text,
    /// A `,`.
    Comma,
    /// A `.`.
    Dot,
    /// A number.
    Number,
    /// An operator or a parenthesis.
    Symbol(&'static str),
}

impl fmt::Display for Found {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::End => f.write_str("the end of the Query"),
            Self::Keyword(keyword) => write!(f, "the keyword `{keyword}`"),
            Self::Name(name) => write!(f, "the name `{name}`"),
            Self::Text => f.write_str("a string"),
            Self::Comma => f.write_str("`,`"),
            Self::Dot => f.write_str("`.`"),
            Self::Number => f.write_str("a number"),
            Self::Symbol(symbol) => write!(f, "`{symbol}`"),
        }
    }
}

struct Suggestion<'a>(Option<&'a str>);

impl fmt::Display for Suggestion<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0
            .map_or(Ok(()), |name| write!(f, "; did you mean `{name}`?"))
    }
}

struct Owner<'a>(Option<&'a str>);

impl fmt::Display for Owner<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.map_or(Ok(()), |owner| write!(f, " of `{owner}`"))
    }
}

struct Holds {
    data_type: DataType,
    array: bool,
}

impl fmt::Display for Holds {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let nested = match self.data_type {
            DataType::Class => Some(("a struct", "structs")),
            DataType::StrongPointer => Some(("a strong pointer", "strong pointers")),
            DataType::WeakPointer => Some(("a weak pointer", "weak pointers")),
            DataType::Reference => Some(("a Reference", "References")),
            _ => None,
        };
        match (nested, self.array) {
            (Some((one, _)), false) => f.write_str(one),
            (Some((_, many)), true) => write!(f, "an array of {many}"),
            (None, _) => write!(f, "an array of {} values", self.data_type),
        }
    }
}

/// A [`QueryError`] with the line of the Query it marks; see [`QueryError::marked`].
#[derive(Debug, Clone, Copy)]
pub struct Marked<'e> {
    error: &'e QueryError,
    query: &'e str,
}

impl fmt::Display for Marked<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let span = self.error.span;
        let start = floor_char_boundary(self.query, span.start());
        let line_start = self
            .query
            .get(..start)
            .and_then(|head| head.rfind('\n'))
            .map_or(0, |newline| newline.saturating_add(1));
        let line_end = self
            .query
            .get(start..)
            .and_then(|tail| tail.find('\n'))
            .map_or(self.query.len(), |newline| start.saturating_add(newline));
        let line = self.query.get(line_start..line_end).unwrap_or_default();
        let before = self.query.get(line_start..start).unwrap_or_default();
        let end = floor_char_boundary(self.query, span.end().clamp(start, line_end));
        let width = self
            .query
            .get(start..end)
            .map_or(0, |text| text.chars().count());
        writeln!(f, "{}", self.error.kind)?;
        f.write_str("  ")?;
        for c in line.chars() {
            write!(
                f,
                "{}",
                if c.is_control() && c != '\t' {
                    '\u{fffd}'
                } else {
                    c
                }
            )?;
        }
        f.write_str("\n  ")?;
        for c in before.chars() {
            f.write_str(if c == '\t' { "\t" } else { " " })?;
        }
        for _ in 0..width.max(1) {
            f.write_str("^")?;
        }
        Ok(())
    }
}
