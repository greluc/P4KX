//! Queries over the Records of a DataForge, defined in `docs/query.md`.
//!
//! [`Query::parse`] reads the text into a [`Query`], [`Query::compile`] checks it against one
//! DataForge, and [`CompiledQuery::run`] selects the Records in parallel, keeps those that meet
//! the condition, and builds the rows page by page from the canonical Record tree.

mod compile;
mod condition;
mod error;
mod lexer;
mod matching;
mod operand;
mod parser;
mod predicate;
mod run;
mod schema;
mod span;
mod suggest;
mod syntax;

pub use compile::CompiledQuery;
pub use condition::{Comparison, Condition, Literal, LiteralValue, MAX_CONDITION_DEPTH, Operator};
pub use error::{Expected, Found, LiteralKind, Marked, QueryError, QueryErrorKind};
pub use lexer::Keyword;
pub use run::{Pages, QueryRun, Row};
pub use span::Span;
pub use syntax::{Glob, Name, Path, Query, Segment};

/// The longest Query text in bytes.
pub const MAX_QUERY_LEN: usize = 65_536;
