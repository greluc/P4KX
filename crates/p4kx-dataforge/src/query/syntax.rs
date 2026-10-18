use std::fmt;

use crate::query::Span;
use crate::query::lexer::{Keyword, is_bare_name};

/// A parsed Query: which Records it selects and which values it lists as columns.
///
/// It prints in its canonical form (`docs/query.md`); parsing that form gives the same Query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Query {
    pub(crate) from: Option<Name>,
    pub(crate) at: Option<Glob>,
    pub(crate) select: Vec<Path>,
}

impl Query {
    /// Returns the type of `from`, the Records of which and of its subtypes are selected.
    #[must_use]
    pub const fn from(&self) -> Option<&Name> {
        self.from.as_ref()
    }

    /// Returns the record-path glob of `at`.
    #[must_use]
    pub const fn at(&self) -> Option<&Glob> {
        self.at.as_ref()
    }

    /// Returns the paths of `select`, one per column after the record path.
    #[must_use]
    pub fn select(&self) -> &[Path] {
        &self.select
    }
}

/// A type or field name with its span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Name {
    pub(crate) text: String,
    pub(crate) span: Span,
}

impl Name {
    /// Returns the name without backticks.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// Returns where the name stands in the Query.
    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }
}

/// The record-path glob of `at` with its span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Glob {
    pub(crate) text: String,
    pub(crate) span: Span,
}

impl Glob {
    /// Returns the glob without quotes.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// Returns where the glob stands in the Query, quotes included.
    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }
}

/// A dot path to a value inside a Record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Path {
    pub(crate) head: Name,
    pub(crate) tail: Vec<Segment>,
    pub(crate) span: Span,
}

impl Path {
    /// Returns the field the path starts with.
    #[must_use]
    pub const fn head(&self) -> &Name {
        &self.head
    }

    /// Returns the segments behind the first field, in order.
    #[must_use]
    pub fn tail(&self) -> &[Segment] {
        &self.tail
    }

    /// Returns where the path stands in the Query.
    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }
}

/// One step of a [`Path`] behind its first field.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Segment {
    /// `.` and a field by name.
    Field(Name),
}

impl Segment {
    /// Returns where the segment stands in the Query.
    #[must_use]
    pub const fn span(&self) -> Span {
        match self {
            Self::Field(name) => name.span,
        }
    }
}

impl fmt::Display for Query {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(from) = &self.from {
            write!(f, "{} {from}", Keyword::From)?;
        }
        if let Some(at) = &self.at {
            let separator = if self.from.is_some() { " " } else { "" };
            write!(f, "{separator}{} {at}", Keyword::At)?;
        }
        if let Some((first, rest)) = self.select.split_first() {
            let separator = if self.from.is_some() || self.at.is_some() {
                " "
            } else {
                ""
            };
            write!(f, "{separator}{} {first}", Keyword::Select)?;
            for path in rest {
                write!(f, ", {path}")?;
            }
        }
        Ok(())
    }
}

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if is_bare_name(&self.text) {
            f.write_str(&self.text)
        } else {
            write!(f, "`{}`", self.text)
        }
    }
}

impl fmt::Display for Glob {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\"{}\"", self.text)
    }
}

impl fmt::Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.head, f)?;
        for segment in &self.tail {
            match segment {
                Segment::Field(name) => write!(f, ".{name}")?,
            }
        }
        Ok(())
    }
}
