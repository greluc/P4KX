use std::fmt;

use crate::query::{QueryError, QueryErrorKind, Span};

/// A word with a meaning of its own in a Query; keywords ignore case.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Keyword {
    /// `from`, the type of the selected Records.
    From,
    /// `at`, the record-path glob of the selected Records.
    At,
    /// `select`, the columns.
    Select,
    /// `where`, reserved for filters.
    Where,
    /// `and`, reserved for filters.
    And,
    /// `or`, reserved for filters.
    Or,
    /// `not`, reserved for filters.
    Not,
}

impl Keyword {
    const ALL: [Self; 7] = [
        Self::From,
        Self::At,
        Self::Select,
        Self::Where,
        Self::And,
        Self::Or,
        Self::Not,
    ];

    /// Returns the keyword spelt `word` in any case.
    pub(crate) fn of(word: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|keyword| keyword.as_str().eq_ignore_ascii_case(word))
    }

    /// Returns the keyword in lower case.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::From => "from",
            Self::At => "at",
            Self::Select => "select",
            Self::Where => "where",
            Self::And => "and",
            Self::Or => "or",
            Self::Not => "not",
        }
    }
}

impl fmt::Display for Keyword {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// What a token of a Query is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Token<'q> {
    Keyword(Keyword),
    Name(&'q str),
    Text(&'q str),
    Comma,
    Dot,
    End,
}

/// Splits the text of a Query into tokens with their spans.
pub(crate) struct Lexer<'q> {
    text: &'q str,
    position: usize,
}

impl<'q> Lexer<'q> {
    pub(crate) const fn new(text: &'q str) -> Self {
        Self { text, position: 0 }
    }

    /// Returns the next token; after the last one, [`Token::End`] at the end of the text.
    pub(crate) fn next_token(&mut self) -> Result<(Token<'q>, Span), QueryError> {
        self.skip_whitespace();
        let start = self.position;
        let Some(c) = self.peek() else {
            return Ok((Token::End, Span::new(start, start)));
        };
        let token = match c {
            ',' => {
                self.bump(c);
                Token::Comma
            }
            '.' => {
                self.bump(c);
                Token::Dot
            }
            '"' => Token::Text(self.delimited('"', QueryErrorKind::UnterminatedString)?),
            '`' => {
                let name = self.delimited('`', QueryErrorKind::UnterminatedName)?;
                if name.is_empty() {
                    return Err(QueryError::new(
                        QueryErrorKind::EmptyName,
                        Span::new(start, self.position),
                    ));
                }
                Token::Name(name)
            }
            c if is_name_start(c) => {
                while let Some(c) = self.peek().filter(|c| is_name_char(*c)) {
                    self.bump(c);
                }
                let word = self.slice(start, self.position);
                Keyword::of(word).map_or(Token::Name(word), Token::Keyword)
            }
            c => {
                return Err(QueryError::new(
                    QueryErrorKind::UnexpectedCharacter(c),
                    Span::new(start, start + c.len_utf8()),
                ));
            }
        };
        Ok((token, Span::new(start, self.position)))
    }

    /// Reads the text between the delimiter at the current position and the next one.
    fn delimited(
        &mut self,
        delimiter: char,
        unterminated: QueryErrorKind,
    ) -> Result<&'q str, QueryError> {
        let start = self.position;
        self.bump(delimiter);
        let inner = self.position;
        while let Some(c) = self.peek() {
            self.bump(c);
            if c == delimiter {
                return Ok(self.slice(inner, self.position - delimiter.len_utf8()));
            }
        }
        Err(QueryError::new(
            unterminated,
            Span::new(start, self.position),
        ))
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek().filter(|c| c.is_whitespace()) {
            self.bump(c);
        }
    }

    fn peek(&self) -> Option<char> {
        self.text.get(self.position..)?.chars().next()
    }

    const fn bump(&mut self, c: char) {
        self.position += c.len_utf8();
    }

    fn slice(&self, start: usize, end: usize) -> &'q str {
        self.text.get(start..end).unwrap_or_default()
    }
}

/// Returns whether `name` can be written without backticks.
pub(crate) fn is_bare_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(is_name_start)
        && chars.all(is_name_char)
        && Keyword::of(name).is_none()
}

const fn is_name_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

const fn is_name_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}
