// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::fmt;

use crate::query::lexer::{Keyword, quote};
use crate::query::{Path, Span};

/// The deepest nesting of parentheses and `not` in a condition.
pub const MAX_CONDITION_DEPTH: usize = 32;

/// The condition of `where`, which a Record has to meet to be selected.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Condition {
    /// A value compared with a literal.
    Compare(Comparison),
    /// The condition does not hold.
    Not(Box<Self>),
    /// Every condition holds; never directly holds another `And`.
    And(Vec<Self>),
    /// One of the conditions holds; never directly holds another `Or`.
    Or(Vec<Self>),
}

/// A value at a path compared with a literal, e.g. `damage > 1000`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comparison {
    pub(crate) path: Path,
    pub(crate) operator: Operator,
    pub(crate) operator_span: Span,
    pub(crate) literal: Literal,
}

impl Comparison {
    /// Returns the path of the compared value.
    #[must_use]
    pub const fn path(&self) -> &Path {
        &self.path
    }

    /// Returns how the value is compared.
    #[must_use]
    pub const fn operator(&self) -> Operator {
        self.operator
    }

    /// Returns where the operator stands in the Query.
    #[must_use]
    pub const fn operator_span(&self) -> Span {
        self.operator_span
    }

    /// Returns the literal the value is compared with.
    #[must_use]
    pub const fn literal(&self) -> &Literal {
        &self.literal
    }

    /// Returns where the comparison stands in the Query, from the path to the literal.
    #[must_use]
    pub const fn span(&self) -> Span {
        self.path.span.to(self.literal.span)
    }
}

/// How a [`Comparison`] compares.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Operator {
    /// `=`.
    Equal,
    /// `!=`.
    NotEqual,
    /// `<`.
    Less,
    /// `<=`.
    LessOrEqual,
    /// `>`.
    Greater,
    /// `>=`.
    GreaterOrEqual,
    /// `contains`, the text occurs in the value, ignoring ASCII case.
    Contains,
    /// `starts with`, the value starts with the text, ignoring ASCII case.
    StartsWith,
    /// `like`, the value matches the glob, ignoring ASCII case.
    Like,
}

impl Operator {
    /// Returns whether the operator matches text rather than comparing values.
    #[must_use]
    pub const fn matches_text(self) -> bool {
        matches!(self, Self::Contains | Self::StartsWith | Self::Like)
    }

    /// Returns the operator as the canonical form writes it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Equal => "=",
            Self::NotEqual => "!=",
            Self::Less => "<",
            Self::LessOrEqual => "<=",
            Self::Greater => ">",
            Self::GreaterOrEqual => ">=",
            Self::Contains => "contains",
            Self::StartsWith => "starts with",
            Self::Like => "like",
        }
    }
}

/// A literal of a [`Comparison`] with its span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Literal {
    pub(crate) value: LiteralValue,
    pub(crate) span: Span,
}

impl Literal {
    /// Returns the value as written.
    #[must_use]
    pub const fn value(&self) -> &LiteralValue {
        &self.value
    }

    /// Returns where the literal stands in the Query.
    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }
}

/// The value of a [`Literal`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum LiteralValue {
    /// A number as written, e.g. `-1.5e3`.
    Number(String),
    /// A string without its quotes, a doubled quote read as one.
    Text(String),
    /// `true` or `false`.
    Bool(bool),
}

impl fmt::Display for Condition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Compare(comparison) => fmt::Display::fmt(comparison, f),
            Self::Not(inner) => {
                write!(f, "{} ", Keyword::Not)?;
                grouped(f, inner, matches!(**inner, Self::And(_) | Self::Or(_)))
            }
            Self::And(factors) => joined(f, factors, Keyword::And, |factor| {
                matches!(factor, Self::Or(_))
            }),
            Self::Or(terms) => joined(f, terms, Keyword::Or, |_| false),
        }
    }
}

fn joined(
    f: &mut fmt::Formatter<'_>,
    parts: &[Condition],
    keyword: Keyword,
    needs_parentheses: impl Fn(&Condition) -> bool,
) -> fmt::Result {
    for (index, part) in parts.iter().enumerate() {
        if index > 0 {
            write!(f, " {keyword} ")?;
        }
        grouped(f, part, needs_parentheses(part))?;
    }
    Ok(())
}

fn grouped(f: &mut fmt::Formatter<'_>, condition: &Condition, parentheses: bool) -> fmt::Result {
    if parentheses {
        write!(f, "({condition})")
    } else {
        fmt::Display::fmt(condition, f)
    }
}

impl fmt::Display for Comparison {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} {}", self.path, self.operator, self.literal)
    }
}

impl fmt::Display for Operator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Display for Literal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.value {
            LiteralValue::Number(text) => f.write_str(text),
            LiteralValue::Text(text) => quote(f, text),
            LiteralValue::Bool(value) => write!(f, "{value}"),
        }
    }
}
