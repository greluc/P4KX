// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::fmt;

use thiserror::Error;

use crate::query::lexer::Keyword;
use crate::query::parser::floor_char_boundary;
use crate::query::{Expected, Found, LiteralKind, MAX_QUERY_LEN, Operator, Span};
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
    #[error(
        "the Query selects no Records; start it with `from <type>`, `at \"<glob>\"` or `referencing \"<record>\"`"
    )]
    MissingSelection,
    /// `from` names a struct the DataForge does not declare.
    #[error("unknown type `{name}`{}", Suggestion(.suggestion.as_deref()))]
    UnknownType {
        /// The name as written.
        name: String,
        /// The closest declared struct name.
        suggestion: Option<String>,
    },
    /// `referencing` names a GUID or record path no Record has.
    #[error("no Record has the GUID or record path `{text}`")]
    UnknownReferencedRecord {
        /// The text as written.
        text: String,
    },
    /// `referencing` was compiled without the index of References it reads.
    #[error("the index of References is missing, so `referencing` cannot select Records")]
    MissingReferenceIndex,
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
    /// A path ends at a value that is no single scalar: a struct, an array or a link.
    #[error(
        "field `{name}`{} holds {}; a path has to end at a scalar value",
        Owner(.owner.as_deref()), Holds { data_type: *.data_type, array: *.array }
    )]
    UnsupportedField {
        /// The field as written from its name on, e.g. `parts[0]`.
        name: String,
        /// The struct declaring it, or `None` without `from`.
        owner: Option<String>,
        /// The type of its values.
        data_type: DataType,
        /// Whether it holds an array.
        array: bool,
    },
    /// A path continues with `.` behind a scalar value.
    #[error("field `{name}`{} holds a {data_type} value and has no field `{next}`", Owner(.owner.as_deref()))]
    ScalarHasNoFields {
        /// The field as written from its name on.
        name: String,
        /// The struct declaring it, or `None` without `from`.
        owner: Option<String>,
        /// The type of its value.
        data_type: DataType,
        /// The name written behind it.
        next: String,
    },
    /// A path continues with `.` behind an array without selecting elements.
    #[error(
        "field `{name}`{} holds {}; select its elements with `[n]`, `[*]` or `[TypeName]` before the field `{next}`",
        Owner(.owner.as_deref()), Holds { data_type: *.data_type, array: true }
    )]
    ArrayHasNoFields {
        /// The field as written from its name on.
        name: String,
        /// The struct declaring it, or `None` without `from`.
        owner: Option<String>,
        /// The type of its elements.
        data_type: DataType,
        /// The name written behind it.
        next: String,
    },
    /// A path continues with `.` behind a Reference or a weak pointer instead of `->`.
    #[error(
        "field `{name}`{} holds {}; follow it with `->{next}` to reach the field `{next}` of what it links to",
        Owner(.owner.as_deref()), Holds { data_type: *.data_type, array: false }
    )]
    LinkHasNoFields {
        /// The field as written from its name on.
        name: String,
        /// The struct declaring it, or `None` without `from`.
        owner: Option<String>,
        /// [`DataType::Reference`] or [`DataType::WeakPointer`].
        data_type: DataType,
        /// The name written behind it.
        next: String,
    },
    /// `->` behind a value that is neither a Reference nor a weak pointer.
    #[error(
        "field `{name}`{} holds {}, which is no Reference; `->` follows References and weak pointers only",
        Owner(.owner.as_deref()), Holds { data_type: *.data_type, array: false }
    )]
    NotALink {
        /// The field as written from its name on.
        name: String,
        /// The struct declaring it, or `None` without `from`.
        owner: Option<String>,
        /// The type of its value.
        data_type: DataType,
    },
    /// `->` behind an array without selecting its elements.
    #[error(
        "field `{name}`{} holds {}; select its elements with `[n]` or `[*]` before `->`",
        Owner(.owner.as_deref()), Holds { data_type: *.data_type, array: true }
    )]
    ArrayIsNoLink {
        /// The field as written from its name on.
        name: String,
        /// The struct declaring it, or `None` without `from`.
        owner: Option<String>,
        /// The type of its elements.
        data_type: DataType,
    },
    /// `->` behind a Reference or weak pointer whose field declares no struct it links to.
    #[error(
        "field `{name}`{} holds {} without a declared type, so `->` cannot reach its field `{next}`",
        Owner(.owner.as_deref()), Holds { data_type: *.data_type, array: false }
    )]
    UntypedLink {
        /// The field as written from its name on.
        name: String,
        /// The struct declaring it, or `None` without `from`.
        owner: Option<String>,
        /// [`DataType::Reference`] or [`DataType::WeakPointer`].
        data_type: DataType,
        /// The name written behind `->`.
        next: String,
    },
    /// `[n]`, `[*]` or `[TypeName]` behind a value that is no array.
    #[error(
        "field `{name}`{} holds {}, which has no elements",
        Owner(.owner.as_deref()), Holds { data_type: *.data_type, array: false }
    )]
    NotAnArray {
        /// The field as written from its name on.
        name: String,
        /// The struct declaring it, or `None` without `from`.
        owner: Option<String>,
        /// The type of its value.
        data_type: DataType,
    },
    /// `[TypeName]` behind an array whose elements are no structs.
    #[error(
        "field `{name}`{} holds {}; `[TypeName]` selects elements of struct arrays only",
        Owner(.owner.as_deref()), Holds { data_type: *.data_type, array: true }
    )]
    ElementsHaveNoType {
        /// The field as written from its name on.
        name: String,
        /// The struct declaring it, or `None` without `from`.
        owner: Option<String>,
        /// The type of its elements.
        data_type: DataType,
    },
    /// `[TypeName]` names a struct no element of the array can have.
    #[error(
        "the elements of field `{name}` are `{element}`, and `{type_name}` neither derives from it nor is a base of it"
    )]
    UnrelatedType {
        /// The field as written from its name on.
        name: String,
        /// The struct the field declares for its elements.
        element: String,
        /// The struct named in the brackets.
        type_name: String,
    },
    /// An index that is no whole number from 0 to `u32::MAX`.
    #[error("an index is a whole number from 0 to {}", u32::MAX)]
    InvalidIndex,
    /// A path with more than [`MAX_PATH_SEGMENTS`](crate::query::MAX_PATH_SEGMENTS) segments
    /// behind its first field.
    #[error("the path has more than {limit} segments behind its first field")]
    PathTooLong {
        /// The most segments allowed.
        limit: usize,
    },
    /// A path with more than [`MAX_PATH_REFERENCES`](crate::query::MAX_PATH_REFERENCES) `->`.
    #[error("the path follows more than {limit} References with `->`")]
    TooManyReferences {
        /// The most `->` allowed.
        limit: usize,
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
    /// A number too large in magnitude to compare with values of the field's type.
    #[error("the number is too large in magnitude to compare with {0} values")]
    NumberOutOfRange(DataType),
    /// A string compared with a GUID field that is no GUID.
    #[error("the string is not a GUID of the form xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx")]
    InvalidGuid,
    /// The schema of the DataForge cannot be read.
    #[error("the DataForge schema cannot be read: {0}")]
    DataForge(DataForgeError),
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
            (None, false) => write!(f, "a {} value", self.data_type),
            (None, true) => write!(f, "an array of {} values", self.data_type),
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
