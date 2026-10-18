mod condition;

use crate::query::lexer::{Keyword, Lexer, Symbol, Token};
use crate::query::syntax::{Glob, Name, Path, Query, Segment};
use crate::query::{
    Expected, Found, MAX_PATH_SEGMENTS, MAX_QUERY_LEN, QueryError, QueryErrorKind, Span,
};

impl Query {
    /// Parses the text of a Query.
    ///
    /// Parsing checks the syntax only; [`Query::compile`] checks the names against a DataForge.
    ///
    /// # Errors
    ///
    /// Returns a [`QueryError`] with the span of the first token that breaks the grammar of
    /// `docs/query.md`, or when the text is longer than [`MAX_QUERY_LEN`] bytes.
    pub fn parse(text: &str) -> Result<Self, QueryError> {
        if text.len() > MAX_QUERY_LEN {
            return Err(QueryError::new(
                QueryErrorKind::TooLong { len: text.len() },
                Span::new(floor_char_boundary(text, MAX_QUERY_LEN), text.len()),
            ));
        }
        Parser::new(text)?.query()
    }
}

/// The clauses in the order they have to appear.
const CLAUSES: [Keyword; 4] = [Keyword::From, Keyword::At, Keyword::Select, Keyword::Where];

struct Parser<'q> {
    lexer: Lexer<'q>,
    token: Token<'q>,
    span: Span,
}

impl<'q> Parser<'q> {
    fn new(text: &'q str) -> Result<Self, QueryError> {
        let mut lexer = Lexer::new(text);
        let (token, span) = lexer.next_token()?;
        Ok(Self { lexer, token, span })
    }

    fn advance(&mut self) -> Result<(Token<'q>, Span), QueryError> {
        let (next, span) = self.lexer.next_token()?;
        let token = std::mem::replace(&mut self.token, next);
        Ok((token, std::mem::replace(&mut self.span, span)))
    }

    fn query(mut self) -> Result<Query, QueryError> {
        let mut query = Query {
            from: None,
            at: None,
            select: Vec::new(),
            condition: None,
        };
        let start = self.span;
        if self.token == Token::Keyword(Keyword::From) {
            self.advance()?;
            query.from = Some(self.type_name()?);
            self.check_clause(Keyword::From, Expected::AfterFrom)?;
        }
        if self.token == Token::Keyword(Keyword::At) {
            self.advance()?;
            query.at = Some(self.glob()?);
            self.check_clause(Keyword::At, Expected::AfterAt)?;
        }
        if query.from.is_none() && query.at.is_none() {
            if self.token != Token::Keyword(Keyword::Select) {
                return Err(self.expected(Expected::Selection));
            }
            self.paths()?;
            return Err(match self.token {
                Token::Keyword(clause @ (Keyword::From | Keyword::At)) => QueryError::new(
                    QueryErrorKind::ClauseOutOfOrder {
                        clause,
                        after: Keyword::Select,
                    },
                    self.span,
                ),
                _ => QueryError::new(QueryErrorKind::MissingSelection, start),
            });
        }
        if self.token == Token::Keyword(Keyword::Select) {
            query.select = self.paths()?;
            self.check_clause(Keyword::Select, Expected::AfterPath)?;
        }
        if self.token == Token::Keyword(Keyword::Where) {
            self.advance()?;
            query.condition = Some(self.condition(0)?);
            self.check_clause(Keyword::Where, Expected::AfterCondition)?;
        }
        Ok(query)
    }

    /// Parses `select` and its comma-separated paths.
    fn paths(&mut self) -> Result<Vec<Path>, QueryError> {
        self.advance()?;
        let mut paths = vec![self.path()?];
        while self.token == Token::Comma {
            self.advance()?;
            paths.push(self.path()?);
        }
        Ok(paths)
    }

    /// Checks that the clause `done` is followed by the end or by a later clause.
    fn check_clause(&self, done: Keyword, expected: Expected) -> Result<(), QueryError> {
        let position = |keyword| CLAUSES.iter().position(|clause| *clause == keyword);
        match self.token {
            Token::End => Ok(()),
            Token::Keyword(keyword) if position(keyword) > position(done) => Ok(()),
            Token::Keyword(keyword) if keyword == done => Err(QueryError::new(
                QueryErrorKind::DuplicateClause(keyword),
                self.span,
            )),
            Token::Keyword(keyword) if position(keyword).is_some() => {
                let after = CLAUSES
                    .into_iter()
                    .find(|clause| position(*clause) > position(keyword))
                    .unwrap_or(done);
                Err(QueryError::new(
                    QueryErrorKind::ClauseOutOfOrder {
                        clause: keyword,
                        after,
                    },
                    self.span,
                ))
            }
            _ => Err(self.expected(expected)),
        }
    }

    fn type_name(&mut self) -> Result<Name, QueryError> {
        match self.token {
            Token::Name(_) | Token::Quoted(_) => self.name(Expected::TypeName),
            _ => Err(self.expected(Expected::TypeName)),
        }
    }

    fn glob(&mut self) -> Result<Glob, QueryError> {
        if !matches!(self.token, Token::Text(_)) {
            return Err(self.expected(Expected::Glob));
        }
        let (token, span) = self.advance()?;
        Ok(Glob {
            text: match token {
                Token::Text(text) => text.into_owned(),
                _ => String::new(),
            },
            span,
        })
    }

    fn path(&mut self) -> Result<Path, QueryError> {
        let head = self.field()?;
        let start = head.span();
        let mut tail = Vec::new();
        loop {
            let segment = match self.token {
                Token::Dot => {
                    self.advance()?;
                    Segment::Field(self.field()?)
                }
                Token::Symbol(Symbol::OpenBracket) => self.bracket()?,
                _ => break,
            };
            if tail.len() == MAX_PATH_SEGMENTS {
                return Err(QueryError::new(
                    QueryErrorKind::PathTooLong {
                        limit: MAX_PATH_SEGMENTS,
                    },
                    segment.span(),
                ));
            }
            tail.push(segment);
        }
        let end = tail.last().map_or(start, Segment::span);
        Ok(Path {
            head,
            tail,
            span: start.to(end),
        })
    }

    /// Parses `[n]`, `[*]` or `[TypeName]`, starting at `[`.
    fn bracket(&mut self) -> Result<Segment, QueryError> {
        let (_, open) = self.advance()?;
        let selector = match self.token {
            Token::Symbol(Symbol::Star) => {
                self.advance()?;
                None
            }
            Token::Number(text) => {
                let (_, span) = self.advance()?;
                let index = Some(text)
                    .filter(|text| text.bytes().all(|byte| byte.is_ascii_digit()))
                    .and_then(|text| text.parse::<u32>().ok())
                    .ok_or_else(|| QueryError::new(QueryErrorKind::InvalidIndex, span))?;
                Some(Ok(index))
            }
            Token::Name(_) | Token::Quoted(_) => Some(Err(self.name(Expected::Index)?)),
            _ => return Err(self.expected(Expected::Index)),
        };
        if self.token != Token::Symbol(Symbol::CloseBracket) {
            return Err(self.expected(Expected::CloseBracket));
        }
        let (_, close) = self.advance()?;
        let span = open.to(close);
        Ok(match selector {
            None => Segment::All(span),
            Some(Ok(index)) => Segment::Index { index, span },
            Some(Err(name)) => Segment::Type { name, span },
        })
    }

    fn field(&mut self) -> Result<Name, QueryError> {
        match self.token {
            Token::Name(_) | Token::Quoted(_) => self.name(Expected::FieldName),
            _ => Err(self.expected(Expected::FieldName)),
        }
    }

    fn name(&mut self, expected: Expected) -> Result<Name, QueryError> {
        let (token, span) = self.advance()?;
        match token {
            Token::Name(text) | Token::Quoted(text) => Ok(Name {
                text: text.to_owned(),
                span,
            }),
            _ => Err(QueryError::new(
                QueryErrorKind::Expected {
                    expected,
                    found: found(&token),
                },
                span,
            )),
        }
    }

    fn expected(&self, expected: Expected) -> QueryError {
        QueryError::new(
            QueryErrorKind::Expected {
                expected,
                found: found(&self.token),
            },
            self.span,
        )
    }
}

fn found(token: &Token<'_>) -> Found {
    match token {
        Token::Keyword(keyword) => Found::Keyword(*keyword),
        Token::Name(name) | Token::Quoted(name) => Found::Name((*name).to_owned()),
        Token::Text(_) => Found::Text,
        Token::Number(_) => Found::Number,
        Token::Symbol(symbol) => Found::Symbol(symbol.as_str()),
        Token::Comma => Found::Comma,
        Token::Dot => Found::Dot,
        Token::End => Found::End,
    }
}

/// Returns the largest char boundary of `text` at or before `offset`.
pub(crate) fn floor_char_boundary(text: &str, offset: usize) -> usize {
    (0..=offset.min(text.len()))
        .rev()
        .find(|offset| text.is_char_boundary(*offset))
        .unwrap_or(0)
}
