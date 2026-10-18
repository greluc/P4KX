/// The matching rule of a Filter over paths in an Archive or DataForge.
///
/// Text with a `*` or `?` is a glob over the full path: `*` matches any run of characters,
/// separators included, and `?` exactly one character. Any other text matches when it occurs
/// anywhere in the path. Both forms ignore ASCII case and treat `/` and `\` alike; every other
/// character, whitespace included, is literal. Matching allocates nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathPattern {
    folded: String,
    glob: bool,
}

impl PathPattern {
    /// Creates the pattern for `text`.
    #[must_use]
    pub fn new(text: &str) -> Self {
        Self {
            folded: text.chars().map(fold).collect(),
            glob: text.contains(['*', '?']),
        }
    }

    /// Returns whether the pattern is a glob rather than a substring.
    #[must_use]
    pub const fn is_glob(&self) -> bool {
        self.glob
    }

    /// Returns whether `path` matches.
    #[must_use]
    pub fn matches(&self, path: &str) -> bool {
        if self.glob {
            glob_matches(&self.folded, path)
        } else {
            contains(path.as_bytes(), self.folded.as_bytes())
        }
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

fn glob_matches(pattern: &str, path: &str) -> bool {
    let (mut token, mut position) = (0, 0);
    let mut resume: Option<(usize, usize)> = None;
    while let Some(c) = path.get(position..).and_then(|rest| rest.chars().next()) {
        let wanted = pattern.get(token..).and_then(|rest| rest.chars().next());
        match wanted {
            Some('*') => {
                resume = Some((token, position));
                token += 1;
            }
            Some('?') => {
                token += 1;
                position += c.len_utf8();
            }
            Some(wanted) if wanted == fold(c) => {
                token += wanted.len_utf8();
                position += c.len_utf8();
            }
            _ => match resume {
                Some((star, start)) => {
                    let skipped = path
                        .get(start..)
                        .and_then(|rest| rest.chars().next())
                        .map_or(1, char::len_utf8);
                    resume = Some((star, start + skipped));
                    token = star + 1;
                    position = start + skipped;
                }
                None => return false,
            },
        }
    }
    pattern
        .get(token..)
        .is_some_and(|rest| rest.chars().all(|c| c == '*'))
}
