// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::borrow::Cow;
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
    /// `referencing`, the Record every selected Record refers to.
    Referencing,
    /// `select`, the columns.
    Select,
    /// `where`, the condition a Record has to meet.
    Where,
    /// `and`, both conditions hold.
    And,
    /// `or`, one of the conditions holds.
    Or,
    /// `not`, the condition does not hold.
    Not,
}

impl Keyword {
    const ALL: [Self; 8] = [
        Self::From,
        Self::At,
        Self::Referencing,
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
            Self::Referencing => "referencing",
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

/// A token of one or two punctuation characters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Symbol {
    Equal,
    NotEqual,
    Less,
    LessOrEqual,
    Greater,
    GreaterOrEqual,
    Open,
    Close,
    OpenBracket,
    CloseBracket,
    Star,
    Arrow,
}

impl Symbol {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Equal => "=",
            Self::NotEqual => "!=",
            Self::Less => "<",
            Self::LessOrEqual => "<=",
            Self::Greater => ">",
            Self::GreaterOrEqual => ">=",
            Self::Open => "(",
            Self::Close => ")",
            Self::OpenBracket => "[",
            Self::CloseBracket => "]",
            Self::Star => "*",
            Self::Arrow => "->",
        }
    }
}

/// What a token of a Query is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Token<'q> {
    Keyword(Keyword),
    Name(&'q str),
    Quoted(&'q str),
    Text(Cow<'q, str>),
    Number(&'q str),
    Symbol(Symbol),
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
            ',' => self.single(c, Token::Comma),
            '.' => self.single(c, Token::Dot),
            '(' => self.single(c, Token::Symbol(Symbol::Open)),
            ')' => self.single(c, Token::Symbol(Symbol::Close)),
            '[' => self.single(c, Token::Symbol(Symbol::OpenBracket)),
            ']' => self.single(c, Token::Symbol(Symbol::CloseBracket)),
            '*' => self.single(c, Token::Symbol(Symbol::Star)),
            '=' => self.single(c, Token::Symbol(Symbol::Equal)),
            '<' => self.with_equal(c, Symbol::Less, Symbol::LessOrEqual),
            '>' => self.with_equal(c, Symbol::Greater, Symbol::GreaterOrEqual),
            '-' if self.peek_second() == Some('>') => {
                self.bump(c);
                self.single('>', Token::Symbol(Symbol::Arrow))
            }
            '!' if self.peek_second() == Some('=') => {
                self.bump(c);
                self.single('=', Token::Symbol(Symbol::NotEqual))
            }
            '"' => Token::Text(self.string()?),
            '`' => {
                let name = self.quoted_name()?;
                if name.is_empty() {
                    return Err(QueryError::new(
                        QueryErrorKind::EmptyName,
                        Span::new(start, self.position),
                    ));
                }
                Token::Quoted(name)
            }
            c if c.is_ascii_digit()
                || (c == '-' && self.peek_second().is_some_and(|c| c.is_ascii_digit())) =>
            {
                Token::Number(self.number(start))
            }
            c if is_name_start(c) => {
                self.bump_while(is_name_char);
                let word = self.slice(start, self.position);
                Keyword::of(word).map_or(Token::Name(word), Token::Keyword)
            }
            c => {
                return Err(QueryError::new(
                    QueryErrorKind::UnexpectedCharacter(c),
                    Span::new(start, start.saturating_add(c.len_utf8())),
                ));
            }
        };
        Ok((token, Span::new(start, self.position)))
    }

    const fn single(&mut self, c: char, token: Token<'q>) -> Token<'q> {
        self.bump(c);
        token
    }

    fn with_equal(&mut self, c: char, alone: Symbol, with_equal: Symbol) -> Token<'q> {
        self.bump(c);
        if self.peek() == Some('=') {
            self.bump('=');
            Token::Symbol(with_equal)
        } else {
            Token::Symbol(alone)
        }
    }

    /// Reads a number: `-`, digits, a fraction and an exponent; a part without its digits is
    /// left for the next token.
    fn number(&mut self, start: usize) -> &'q str {
        if self.peek() == Some('-') {
            self.bump('-');
        }
        self.bump_while(|c| c.is_ascii_digit());
        if self.peek() == Some('.') && self.peek_second().is_some_and(|c| c.is_ascii_digit()) {
            self.bump('.');
            self.bump_while(|c| c.is_ascii_digit());
        }
        if matches!(self.peek(), Some('e' | 'E')) {
            let mark = self.position;
            self.bump('e');
            if let Some(sign @ ('+' | '-')) = self.peek() {
                self.bump(sign);
            }
            if self.peek().is_some_and(|c| c.is_ascii_digit()) {
                self.bump_while(|c| c.is_ascii_digit());
            } else {
                self.position = mark;
            }
        }
        self.slice(start, self.position)
    }

    /// Reads a string in double quotes, where a doubled quote stands for one quote.
    fn string(&mut self) -> Result<Cow<'q, str>, QueryError> {
        let start = self.position;
        self.bump('"');
        let inner = self.position;
        let mut doubled = false;
        while let Some(c) = self.peek() {
            self.bump(c);
            if c == '"' {
                if self.peek() == Some('"') {
                    self.bump('"');
                    doubled = true;
                    continue;
                }
                let text = self.slice(inner, self.position.saturating_sub(1));
                return Ok(if doubled {
                    Cow::Owned(text.replace("\"\"", "\""))
                } else {
                    Cow::Borrowed(text)
                });
            }
        }
        Err(QueryError::new(
            QueryErrorKind::UnterminatedString,
            Span::new(start, self.position),
        ))
    }

    /// Reads a name between backticks.
    fn quoted_name(&mut self) -> Result<&'q str, QueryError> {
        let start = self.position;
        self.bump('`');
        let inner = self.position;
        while let Some(c) = self.peek() {
            self.bump(c);
            if c == '`' {
                return Ok(self.slice(inner, self.position.saturating_sub(1)));
            }
        }
        Err(QueryError::new(
            QueryErrorKind::UnterminatedName,
            Span::new(start, self.position),
        ))
    }

    fn skip_whitespace(&mut self) {
        self.bump_while(char::is_whitespace);
    }

    fn bump_while(&mut self, keep: impl Fn(char) -> bool) {
        while let Some(c) = self.peek().filter(|c| keep(*c)) {
            self.bump(c);
        }
    }

    fn peek(&self) -> Option<char> {
        self.text.get(self.position..)?.chars().next()
    }

    fn peek_second(&self) -> Option<char> {
        self.text.get(self.position..)?.chars().nth(1)
    }

    const fn bump(&mut self, c: char) {
        self.position = self.position.saturating_add(c.len_utf8());
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

/// Writes `text` in double quotes with each quote doubled.
pub(crate) fn quote(f: &mut fmt::Formatter<'_>, text: &str) -> fmt::Result {
    f.write_str("\"")?;
    for (index, part) in text.split('"').enumerate() {
        if index > 0 {
            f.write_str("\"\"")?;
        }
        f.write_str(part)?;
    }
    f.write_str("\"")
}

const fn is_name_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

const fn is_name_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}
