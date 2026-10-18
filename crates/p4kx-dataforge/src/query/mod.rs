//! Queries over the Records of a DataForge, defined in `docs/query.md`.
//!
//! [`Query::parse`] reads the text into a [`Query`], [`Query::compile`] checks it against one
//! DataForge, and [`CompiledQuery::run`] selects the Records in parallel and builds the rows page
//! by page from the canonical Record tree.

mod compile;
mod error;
mod lexer;
mod output;
mod parser;
mod run;
mod schema;
mod span;
mod suggest;
mod syntax;

pub use compile::CompiledQuery;
pub use error::{Expected, Found, Marked, QueryError, QueryErrorKind};
pub use lexer::Keyword;
pub use output::{RECORD_PATH_COLUMN, ResultFormat, ResultWriter};
pub use run::{Pages, QueryRun, Row};
pub use span::Span;
pub use syntax::{Glob, Name, Path, Query, Segment};

/// The longest Query text in bytes.
pub const MAX_QUERY_LEN: usize = 65_536;
