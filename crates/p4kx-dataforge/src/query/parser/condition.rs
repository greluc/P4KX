use crate::query::lexer::{Keyword, Symbol, Token};
use crate::query::parser::Parser;
use crate::query::{
    Comparison, Condition, Expected, Literal, LiteralValue, MAX_CONDITION_DEPTH, Operator,
    QueryError, QueryErrorKind, Span,
};

impl Parser<'_> {
    /// Parses conditions joined by `or`; `depth` counts the enclosing parentheses and `not`.
    pub(super) fn condition(&mut self, depth: usize) -> Result<Condition, QueryError> {
        let mut terms = Vec::new();
        loop {
            match self.term(depth)? {
                Condition::Or(inner) => terms.extend(inner),
                term => terms.push(term),
            }
            if self.token != Token::Keyword(Keyword::Or) {
                return Ok(joined(terms, Condition::Or));
            }
            self.advance()?;
        }
    }

    /// Parses conditions joined by `and`.
    fn term(&mut self, depth: usize) -> Result<Condition, QueryError> {
        let mut factors = Vec::new();
        loop {
            match self.factor(depth)? {
                Condition::And(inner) => factors.extend(inner),
                factor => factors.push(factor),
            }
            if self.token != Token::Keyword(Keyword::And) {
                return Ok(joined(factors, Condition::And));
            }
            self.advance()?;
        }
    }

    /// Parses `not`, a condition in parentheses or a comparison.
    fn factor(&mut self, depth: usize) -> Result<Condition, QueryError> {
        match self.token {
            Token::Keyword(Keyword::Not) => {
                let depth = self.deeper(depth)?;
                self.advance()?;
                Ok(Condition::Not(Box::new(self.factor(depth)?)))
            }
            Token::Symbol(Symbol::Open) => {
                let depth = self.deeper(depth)?;
                self.advance()?;
                let inner = self.condition(depth)?;
                if self.token != Token::Symbol(Symbol::Close) {
                    return Err(self.expected(Expected::CloseParenthesis));
                }
                self.advance()?;
                Ok(inner)
            }
            Token::Name(_) | Token::Quoted(_) => self.comparison(),
            _ => Err(self.expected(Expected::Condition)),
        }
    }

    /// Returns the depth inside the current token, or an error when it is deeper than allowed.
    fn deeper(&self, depth: usize) -> Result<usize, QueryError> {
        depth
            .checked_add(1)
            .filter(|deeper| *deeper <= MAX_CONDITION_DEPTH)
            .ok_or_else(|| {
                QueryError::new(
                    QueryErrorKind::ConditionTooDeep {
                        limit: MAX_CONDITION_DEPTH,
                    },
                    self.span,
                )
            })
    }

    fn comparison(&mut self) -> Result<Condition, QueryError> {
        let path = self.path()?;
        let (operator, operator_span) = self.operator()?;
        let literal = self.literal(operator)?;
        Ok(Condition::Compare(Comparison {
            path,
            operator,
            operator_span,
            literal,
        }))
    }

    fn operator(&mut self) -> Result<(Operator, Span), QueryError> {
        let operator = match self.token {
            Token::Symbol(Symbol::Equal) => Operator::Equal,
            Token::Symbol(Symbol::NotEqual) => Operator::NotEqual,
            Token::Symbol(Symbol::Less) => Operator::Less,
            Token::Symbol(Symbol::LessOrEqual) => Operator::LessOrEqual,
            Token::Symbol(Symbol::Greater) => Operator::Greater,
            Token::Symbol(Symbol::GreaterOrEqual) => Operator::GreaterOrEqual,
            Token::Name(word) if word.eq_ignore_ascii_case("contains") => Operator::Contains,
            Token::Name(word) if word.eq_ignore_ascii_case("like") => Operator::Like,
            Token::Name(word) if word.eq_ignore_ascii_case("starts") => {
                let (_, start) = self.advance()?;
                if !matches!(self.token, Token::Name(word) if word.eq_ignore_ascii_case("with")) {
                    return Err(self.expected(Expected::With));
                }
                let (_, end) = self.advance()?;
                return Ok((Operator::StartsWith, start.to(end)));
            }
            _ => return Err(self.expected(Expected::Operator)),
        };
        let (_, span) = self.advance()?;
        Ok((operator, span))
    }

    fn literal(&mut self, operator: Operator) -> Result<Literal, QueryError> {
        let text_only = operator.matches_text();
        let fits = match self.token {
            Token::Text(_) => true,
            Token::Number(_) => !text_only,
            Token::Name(word) => !text_only && bool_of(word).is_some(),
            _ => false,
        };
        if !fits {
            return Err(self.expected(if text_only {
                Expected::Text
            } else {
                Expected::Value
            }));
        }
        let (token, span) = self.advance()?;
        let value = match token {
            Token::Text(text) => LiteralValue::Text(text.into_owned()),
            Token::Number(number) => LiteralValue::Number(number.to_owned()),
            Token::Name(word) => LiteralValue::Bool(bool_of(word).unwrap_or_default()),
            _ => LiteralValue::Text(String::new()),
        };
        Ok(Literal { value, span })
    }
}

/// Returns the single condition, or `join` of several.
fn joined(mut parts: Vec<Condition>, join: fn(Vec<Condition>) -> Condition) -> Condition {
    if parts.len() == 1
        && let Some(part) = parts.pop()
    {
        return part;
    }
    join(parts)
}

const fn bool_of(word: &str) -> Option<bool> {
    if word.eq_ignore_ascii_case("true") {
        Some(true)
    } else if word.eq_ignore_ascii_case("false") {
        Some(false)
    } else {
        None
    }
}
