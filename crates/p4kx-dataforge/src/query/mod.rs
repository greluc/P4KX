//! Queries over the Records of a DataForge, defined in `docs/query.md`.
//!
//! [`Query::parse`] reads the text into a [`Query`], [`Query::compile`] checks it against one
//! DataForge, and [`CompiledQuery::run`] selects the Records in parallel, keeps those that meet
//! the condition, and builds the rows page by page from the Records' instances.

mod compile;
mod condition;
mod error;
mod expected;
mod lexer;
mod matching;
mod operand;
mod output;
mod parser;
mod path_plan;
mod predicate;
mod reader;
mod resolve;
mod run;
mod schema;
mod span;
mod suggest;
mod syntax;

pub use compile::CompiledQuery;
pub use condition::{Comparison, Condition, Literal, LiteralValue, MAX_CONDITION_DEPTH, Operator};
pub use error::{Marked, QueryError, QueryErrorKind};
pub use expected::{Expected, Found, LiteralKind};
pub use lexer::Keyword;
pub use output::{RECORD_PATH_COLUMN, ResultFormat, ResultWriter};
pub use run::{Pages, QueryRun, Row};
pub use span::Span;
pub use syntax::{Glob, Name, Path, Query, Segment};

/// The most segments a path has behind its first field: `.name`, `[n]`, `[*]` and `[TypeName]`.
pub const MAX_PATH_SEGMENTS: usize = 64;

/// The most array elements one path reaches in one Record; beyond them its value is an error.
pub const MAX_PATH_VALUES: usize = 65_536;

/// The longest Query text in bytes.
pub const MAX_QUERY_LEN: usize = 65_536;
