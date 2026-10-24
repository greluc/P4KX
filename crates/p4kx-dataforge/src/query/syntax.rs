// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::fmt;

use crate::query::lexer::{Keyword, is_bare_name, quote};
use crate::query::{Condition, Span};

/// A parsed Query: which Records it selects and which values it lists as columns.
///
/// It prints in its canonical form (`docs/query.md`); parsing that form gives the same Query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Query {
    pub(crate) from: Option<Name>,
    pub(crate) at: Option<Glob>,
    pub(crate) referencing: Option<Referent>,
    pub(crate) select: Vec<Path>,
    pub(crate) condition: Option<Condition>,
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

    /// Returns the GUID or record path of `referencing`, whose referrers are selected.
    #[must_use]
    pub const fn referencing(&self) -> Option<&Referent> {
        self.referencing.as_ref()
    }

    /// Returns the paths of `select`, one per column after the record path.
    #[must_use]
    pub fn select(&self) -> &[Path] {
        &self.select
    }

    /// Returns the condition of `where` a Record has to meet.
    #[must_use]
    pub const fn condition(&self) -> Option<&Condition> {
        self.condition.as_ref()
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

/// The GUID or record path of `referencing` with its span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Referent {
    pub(crate) text: String,
    pub(crate) span: Span,
}

impl Referent {
    /// Returns the text without quotes.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// Returns where the text stands in the Query, quotes included.
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

    /// Returns whether the path can reach several values, through `[*]` or `[TypeName]`.
    #[must_use]
    pub fn is_multi_valued(&self) -> bool {
        self.tail.iter().any(Segment::is_multi_valued)
    }

    /// Returns the path with names as written but without backticks, for messages.
    pub(crate) fn written(&self) -> String {
        let mut text = self.head.text.clone();
        for segment in &self.tail {
            text.push_str(&segment.written());
        }
        text
    }
}

/// One step of a [`Path`] behind its first field.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Segment {
    /// `.` and a field by name.
    Field(Name),
    /// `[n]`, the element at index `n`, counted from 0.
    Index {
        /// The index.
        index: u32,
        /// Where the brackets stand in the Query.
        span: Span,
    },
    /// `[*]`, every element.
    All(Span),
    /// `[TypeName]`, the elements whose struct is the named one or derives from it.
    Type {
        /// The struct name.
        name: Name,
        /// Where the brackets stand in the Query.
        span: Span,
    },
    /// `->` and a field by name: the field of the Record or instance a Reference or weak pointer
    /// links to.
    Follow {
        /// Where the `->` stands in the Query.
        arrow: Span,
        /// The field name.
        name: Name,
    },
}

impl Segment {
    /// Returns where the segment stands in the Query: the name of a field, `->` with its name, the
    /// brackets of the others.
    #[must_use]
    pub const fn span(&self) -> Span {
        match self {
            Self::Field(name) => name.span,
            Self::Follow { arrow, name } => arrow.to(name.span),
            Self::Index { span, .. } | Self::All(span) | Self::Type { span, .. } => *span,
        }
    }

    /// Returns whether the segment can select several values: `[*]` and `[TypeName]`.
    #[must_use]
    pub const fn is_multi_valued(&self) -> bool {
        matches!(self, Self::All(_) | Self::Type { .. })
    }

    /// Returns the segment with its name as written but without backticks, for messages.
    pub(crate) fn written(&self) -> String {
        match self {
            Self::Field(name) => format!(".{}", name.text),
            Self::Index { index, .. } => format!("[{index}]"),
            Self::All(_) => "[*]".to_owned(),
            Self::Type { name, .. } => format!("[{}]", name.text),
            Self::Follow { name, .. } => format!("->{}", name.text),
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
        if let Some(referent) = &self.referencing {
            let separator = if self.from.is_some() || self.at.is_some() {
                " "
            } else {
                ""
            };
            write!(f, "{separator}{} {referent}", Keyword::Referencing)?;
        }
        if let Some((first, rest)) = self.select.split_first() {
            let separator =
                if self.from.is_some() || self.at.is_some() || self.referencing.is_some() {
                    " "
                } else {
                    ""
                };
            write!(f, "{separator}{} {first}", Keyword::Select)?;
            for path in rest {
                write!(f, ", {path}")?;
            }
        }
        if let Some(condition) = &self.condition {
            write!(f, " {} {condition}", Keyword::Where)?;
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
        quote(f, &self.text)
    }
}

impl fmt::Display for Referent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        quote(f, &self.text)
    }
}

impl fmt::Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.head, f)?;
        for segment in &self.tail {
            match segment {
                Segment::Field(name) => write!(f, ".{name}")?,
                Segment::Index { index, .. } => write!(f, "[{index}]")?,
                Segment::All(_) => f.write_str("[*]")?,
                Segment::Type { name, .. } => write!(f, "[{name}]")?,
                Segment::Follow { name, .. } => write!(f, "->{name}")?,
            }
        }
        Ok(())
    }
}
