use std::ops::Range;

use crate::query::lexer::{Keyword, Lexer, Symbol, Token};
use crate::query::{MAX_PATH_SEGMENTS, MAX_QUERY_LEN, Span};

/// Where the cursor stands in a Query text: the word typed before it, the whole word around it
/// that a name replaces, and what fits there.
#[derive(Debug)]
pub(super) struct Spot<'q> {
    pub(super) prefix: &'q str,
    pub(super) replace: Range<usize>,
    pub(super) from: Option<&'q str>,
    pub(super) want: Want<'q>,
}

/// What fits at the cursor.
#[derive(Debug)]
pub(super) enum Want<'q> {
    /// A struct name, after `from`.
    Type,
    /// The first field of a path, after `select`, `,`, `where`, `and`, `or`, `not` or `(`.
    Head,
    /// A name behind `trigger` at the end of `path`.
    Behind { path: Walk<'q>, trigger: Trigger },
}

/// What stands right before the cursor's word in a path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Trigger {
    /// `.`, a field follows.
    Dot,
    /// `->`, a field of the linked struct follows.
    Arrow,
    /// `[`, a struct name follows.
    Bracket,
}

/// A path written before the cursor: its first field and its steps.
#[derive(Debug)]
pub(super) struct Walk<'q> {
    pub(super) head: &'q str,
    pub(super) steps: Vec<Step<'q>>,
}

/// One step of a [`Walk`].
#[derive(Debug, Clone, Copy)]
pub(super) enum Step<'q> {
    Field(&'q str),
    Elements,
    Type(&'q str),
    Follow(&'q str),
}

/// Returns where `cursor` stands in `text`, or `None` where no name fits or the text before the
/// cursor cannot be read.
pub(super) fn spot(text: &str, cursor: usize) -> Option<Spot<'_>> {
    if cursor > MAX_QUERY_LEN {
        return None;
    }
    let before = text.get(..cursor)?;
    let tokens = tokens(before)?;
    let (prefix, replace, context) = match tokens.split_last() {
        Some(((Token::Name(word), span), rest)) if span.end() == cursor => {
            (*word, span.range(), rest)
        }
        Some(((Token::Keyword(keyword), span), rest)) if span.end() == cursor => (
            before.get(span.range()).unwrap_or(keyword.as_str()),
            span.range(),
            rest,
        ),
        _ => ("", cursor..cursor, tokens.as_slice()),
    };
    let rest = text
        .get(cursor..)
        .unwrap_or_default()
        .bytes()
        .take_while(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
        .count();
    let replace = replace.start..cursor.saturating_add(rest);
    let from = from_type(&tokens);
    let want = match context.last() {
        Some((Token::Keyword(Keyword::From), _)) => Want::Type,
        Some((token, _)) if starts_path(token) => Want::Head,
        Some((Token::Dot, _)) => behind(context, Trigger::Dot)?,
        Some((Token::Symbol(Symbol::Arrow), _)) => behind(context, Trigger::Arrow)?,
        Some((Token::Symbol(Symbol::OpenBracket), _)) => behind(context, Trigger::Bracket)?,
        _ => return None,
    };
    Some(Spot {
        prefix,
        replace,
        from,
        want,
    })
}

fn tokens(text: &str) -> Option<Vec<(Token<'_>, Span)>> {
    let mut lexer = Lexer::new(text);
    let mut tokens = Vec::new();
    loop {
        let (token, span) = lexer.next_token().ok()?;
        if token == Token::End {
            return Some(tokens);
        }
        tokens.push((token, span));
    }
}

/// Returns the type name of `from`, the token after the keyword.
fn from_type<'q>(tokens: &[(Token<'q>, Span)]) -> Option<&'q str> {
    let at = tokens
        .iter()
        .position(|(token, _)| *token == Token::Keyword(Keyword::From))?;
    match tokens.get(at.checked_add(1)?) {
        Some((Token::Name(name) | Token::Quoted(name), _)) => Some(name),
        _ => None,
    }
}

const fn starts_path(token: &Token<'_>) -> bool {
    matches!(
        token,
        Token::Keyword(
            Keyword::Select | Keyword::Where | Keyword::And | Keyword::Or | Keyword::Not
        ) | Token::Comma
            | Token::Symbol(Symbol::Open)
    )
}

const fn in_path(token: &Token<'_>) -> bool {
    matches!(
        token,
        Token::Name(_)
            | Token::Quoted(_)
            | Token::Number(_)
            | Token::Dot
            | Token::Symbol(
                Symbol::Arrow | Symbol::OpenBracket | Symbol::CloseBracket | Symbol::Star
            )
    )
}

/// Reads the path that ends with the trigger, the last of `context`.
fn behind<'q>(context: &[(Token<'q>, Span)], trigger: Trigger) -> Option<Want<'q>> {
    let (_, written) = context.split_last()?;
    let start = written
        .iter()
        .rposition(|(token, _)| !in_path(token))
        .map_or(0, |index| index.saturating_add(1));
    let (opener, _) = written.get(start.checked_sub(1)?)?;
    if !starts_path(opener) {
        return None;
    }
    let path = walk(written.get(start..)?)?;
    Some(Want::Behind { path, trigger })
}

const fn name<'q>(token: Option<&(Token<'q>, Span)>) -> Option<&'q str> {
    match token {
        Some((Token::Name(name) | Token::Quoted(name), _)) => Some(name),
        _ => None,
    }
}

/// Parses a complete path from `tokens`.
fn walk<'q>(tokens: &[(Token<'q>, Span)]) -> Option<Walk<'q>> {
    let mut tokens = tokens.iter();
    let head = name(tokens.next())?;
    let mut steps = Vec::new();
    while let Some((token, _)) = tokens.next() {
        if steps.len() >= MAX_PATH_SEGMENTS {
            return None;
        }
        let step = match token {
            Token::Dot => Step::Field(name(tokens.next())?),
            Token::Symbol(Symbol::Arrow) => Step::Follow(name(tokens.next())?),
            Token::Symbol(Symbol::OpenBracket) => {
                let step = match tokens.next() {
                    Some((Token::Number(_) | Token::Symbol(Symbol::Star), _)) => Step::Elements,
                    other => Step::Type(name(other)?),
                };
                match tokens.next() {
                    Some((Token::Symbol(Symbol::CloseBracket), _)) => step,
                    _ => return None,
                }
            }
            _ => return None,
        };
        steps.push(step);
    }
    Some(Walk { head, steps })
}
