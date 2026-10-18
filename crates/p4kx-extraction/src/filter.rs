use std::fmt;

use p4kx_archive::{Archive, Entry};

/// Text that selects Entries by their path.
///
/// Text with a `*` or `?` is a glob over the full path: `*` matches any run of characters,
/// separators included, and `?` exactly one character. Any other text matches when it occurs
/// anywhere in the path. Both forms ignore ASCII case and treat `/` and `\` alike; every other
/// character, whitespace included, is literal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Filter {
    text: String,
    pattern: Pattern,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Pattern {
    Substring(Vec<u8>),
    Glob(Vec<Token>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Token {
    Literal(char),
    AnyOne,
    AnyRun,
}

impl Filter {
    /// Creates the Filter for `text`.
    #[must_use]
    pub fn new(text: &str) -> Self {
        let pattern = if text.contains(['*', '?']) {
            Pattern::Glob(
                text.chars()
                    .map(|c| match c {
                        '*' => Token::AnyRun,
                        '?' => Token::AnyOne,
                        _ => Token::Literal(fold(c)),
                    })
                    .collect(),
            )
        } else {
            Pattern::Substring(text.bytes().map(fold_byte).collect())
        };
        Self {
            text: text.to_owned(),
            pattern,
        }
    }

    /// Returns the text the Filter was created from.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// Returns whether the Filter is a glob rather than a substring.
    #[must_use]
    pub const fn is_glob(&self) -> bool {
        matches!(self.pattern, Pattern::Glob(_))
    }

    /// Returns whether the Entry path `path` matches.
    #[must_use]
    pub fn matches(&self, path: &str) -> bool {
        match &self.pattern {
            Pattern::Substring(needle) => contains(path.as_bytes(), needle),
            Pattern::Glob(tokens) => {
                glob_matches(tokens, &path.chars().map(fold).collect::<Vec<_>>())
            }
        }
    }

    /// Returns the Entries of `archive` whose path matches, in Archive order.
    #[must_use]
    pub fn select<'a>(&self, archive: &'a Archive) -> Vec<&'a Entry> {
        archive
            .entries()
            .iter()
            .filter(|entry| self.matches(entry.path()))
            .collect()
    }
}

impl fmt::Display for Filter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text)
    }
}

const fn fold(c: char) -> char {
    if c == '/' {
        '\\'
    } else {
        c.to_ascii_lowercase()
    }
}

const fn fold_byte(byte: u8) -> u8 {
    if byte == b'/' {
        b'\\'
    } else {
        byte.to_ascii_lowercase()
    }
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    needle.is_empty()
        || haystack.windows(needle.len()).any(|window| {
            window
                .iter()
                .zip(needle)
                .all(|(&byte, &wanted)| fold_byte(byte) == wanted)
        })
}

fn glob_matches(tokens: &[Token], path: &[char]) -> bool {
    let (mut token, mut position) = (0, 0);
    let mut resume: Option<(usize, usize)> = None;
    while position < path.len() {
        match (tokens.get(token), path.get(position)) {
            (Some(Token::AnyRun), _) => {
                resume = Some((token, position));
                token += 1;
            }
            (Some(Token::AnyOne), _) => {
                token += 1;
                position += 1;
            }
            (Some(Token::Literal(wanted)), Some(c)) if wanted == c => {
                token += 1;
                position += 1;
            }
            _ => match resume {
                Some((star, start)) => {
                    resume = Some((star, start + 1));
                    token = star + 1;
                    position = start + 1;
                }
                None => return false,
            },
        }
    }
    tokens
        .get(token..)
        .is_some_and(|rest| rest.iter().all(|t| *t == Token::AnyRun))
}
