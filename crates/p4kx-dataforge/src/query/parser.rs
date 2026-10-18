use crate::query::lexer::{Keyword, Lexer, Token};
use crate::query::syntax::{Name, Path, Query, Segment, Text};
use crate::query::{Expected, Found, MAX_QUERY_LEN, QueryError, QueryErrorKind, Span};

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
                Span::new(MAX_QUERY_LEN, text.len()),
            ));
        }
        Parser::new(text)?.query()
    }
}

/// The clauses in the order they have to appear.
const CLAUSES: [Keyword; 3] = [Keyword::From, Keyword::At, Keyword::Select];

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
            Token::Name(_) => self.name(),
            _ => Err(self.expected(Expected::TypeName)),
        }
    }

    fn glob(&mut self) -> Result<Text, QueryError> {
        match self.token {
            Token::Text(text) => {
                let (_, span) = self.advance()?;
                Ok(Text {
                    text: text.to_owned(),
                    span,
                })
            }
            _ => Err(self.expected(Expected::Glob)),
        }
    }

    fn path(&mut self) -> Result<Path, QueryError> {
        let head = self.field()?;
        let start = head.span();
        let mut tail = Vec::new();
        while self.token == Token::Dot {
            self.advance()?;
            tail.push(Segment::Field(self.field()?));
        }
        let end = tail.last().map_or(start, Segment::span);
        Ok(Path {
            head,
            tail,
            span: start.to(end),
        })
    }

    fn field(&mut self) -> Result<Name, QueryError> {
        match self.token {
            Token::Name(_) => self.name(),
            _ => Err(self.expected(Expected::FieldName)),
        }
    }

    fn name(&mut self) -> Result<Name, QueryError> {
        let (token, span) = self.advance()?;
        match token {
            Token::Name(text) => Ok(Name {
                text: text.to_owned(),
                span,
            }),
            _ => Err(QueryError::new(
                QueryErrorKind::Expected {
                    expected: Expected::FieldName,
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
        Token::Name(name) => Found::Name((*name).to_owned()),
        Token::Text(_) => Found::Text,
        Token::Comma => Found::Comma,
        Token::Dot => Found::Dot,
        Token::End => Found::End,
    }
}
