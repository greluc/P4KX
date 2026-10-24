// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::borrow::Cow;

use crate::contract::LocalisedText;
use crate::texts::Texts;

const OPEN: &str = "~mission(";
const MAX_DEPTH: usize = 3;
const MAX_REPLACEMENTS: usize = 64;
const MAX_OUTPUT: usize = 4096;

/// What a `~mission(Token)` in a text is replaced by.
pub(crate) enum Token<'a> {
    /// The text of this localisation key, in the language being written.
    Key(&'a str),
    /// A value the game picks when the mission starts, written as `[NAME]`.
    Placeholder(&'a str),
}

/// The text a token stands for in one language.
enum Replacement<'a> {
    Text(Cow<'a, str>),
    Placeholder(&'a str),
}

type Lookup<'x, 'a> = dyn Fn(&str, Option<&str>) -> Option<Replacement<'a>> + 'x;

/// Returns `key` as a [`LocalisedText`] whose texts have every `~mission(Token[|Variant])`
/// replaced by what `resolve` says for the token and the variant, with leading and trailing
/// blanks removed. A token `resolve` returns `None` for, or whose key has no text in the
/// language, becomes `[TOKEN]` in upper case. A replacement that holds tokens is expanded up to
/// three levels deep; a title takes at most 64 replacements and about 4 KiB, and a token past
/// either limit becomes `[TOKEN]` too.
pub(crate) fn title<'a>(
    key: Option<&'a str>,
    texts: &Texts<'a>,
    resolve: &dyn Fn(&str, Option<&str>) -> Option<Token<'a>>,
) -> LocalisedText<'a> {
    let mut title = texts.localised(key);
    title.en = title
        .en
        .take()
        .map(|text| render(text, texts, resolve, english));
    title.de = title
        .de
        .take()
        .map(|text| render(text, texts, resolve, german));
    title
}

fn english<'x>(text: &'x LocalisedText<'_>) -> Option<&'x str> {
    text.en.as_deref()
}

fn german<'x>(text: &'x LocalisedText<'_>) -> Option<&'x str> {
    text.de.as_deref()
}

fn render<'a>(
    text: Cow<'a, str>,
    texts: &Texts<'a>,
    resolve: &dyn Fn(&str, Option<&str>) -> Option<Token<'a>>,
    pick: for<'x> fn(&'x LocalisedText<'a>) -> Option<&'x str>,
) -> Cow<'a, str> {
    if !text.contains(OPEN) {
        return trimmed(text);
    }
    let lookup = |name: &str, variant: Option<&str>| match resolve(name, variant)? {
        Token::Key(key) => {
            let found = texts.localised(Some(key));
            pick(&found)
                .filter(|found| !found.is_empty())
                .map(|found| Replacement::Text(Cow::Owned(found.trim().to_owned())))
        }
        Token::Placeholder(label) => Some(Replacement::Placeholder(label)),
    };
    trimmed(Cow::Owned(expand_tokens(&text, &lookup)))
}

/// Replaces the tokens of `text` through `lookup`, within the depth, count and size limits.
fn expand_tokens(text: &str, lookup: &Lookup<'_, '_>) -> String {
    let mut out = String::with_capacity(text.len());
    let mut count = 0;
    substitute(&mut out, text, 0, &mut count, lookup);
    out
}

fn substitute(
    out: &mut String,
    text: &str,
    depth: usize,
    count: &mut usize,
    lookup: &Lookup<'_, '_>,
) {
    let mut rest: &str = text;
    while let Some((before, after)) = rest.split_once(OPEN) {
        out.push_str(before);
        if let Some((content, tail)) = after.split_once(')') {
            let (name, variant) = match content.split_once('|') {
                Some((name, variant)) => {
                    (name, Some(variant).filter(|variant| !variant.is_empty()))
                }
                None => (content, None),
            };
            expand(out, (name, variant), depth, count, lookup);
            rest = tail;
        } else {
            out.push_str(OPEN);
            rest = after;
        }
    }
    out.push_str(rest);
}

fn expand(
    out: &mut String,
    (name, variant): (&str, Option<&str>),
    depth: usize,
    count: &mut usize,
    lookup: &Lookup<'_, '_>,
) {
    *count = count.saturating_add(1);
    if *count <= MAX_REPLACEMENTS && out.len() < MAX_OUTPUT {
        match lookup(name, variant) {
            Some(Replacement::Placeholder(label)) => {
                out.push('[');
                out.push_str(label);
                out.push(']');
                return;
            }
            Some(Replacement::Text(found)) => {
                let fits = out.len().saturating_add(found.len()) <= MAX_OUTPUT;
                if !found.contains(OPEN) && fits {
                    out.push_str(&found);
                    return;
                }
                if depth < MAX_DEPTH && fits {
                    substitute(out, &found, depth + 1, count, lookup);
                    return;
                }
            }
            None => {}
        }
    }
    out.push('[');
    out.push_str(&name.to_ascii_uppercase());
    out.push(']');
}

fn trimmed(text: Cow<'_, str>) -> Cow<'_, str> {
    match text {
        Cow::Borrowed(text) => Cow::Borrowed(text.trim()),
        Cow::Owned(text) => {
            let trimmed = text.trim();
            if trimmed.len() == text.len() {
                Cow::Owned(text)
            } else {
                Cow::Owned(trimmed.to_owned())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(text: &str, table: &[(&str, &str)]) -> String {
        let lookup = |name: &str, _: Option<&str>| {
            table
                .iter()
                .find(|(token, _)| *token == name)
                .map(|(_, text)| Replacement::Text(Cow::Owned((*text).trim().to_owned())))
        };
        expand_tokens(text, &lookup)
    }

    #[test]
    fn a_self_referencing_replacement_stops_at_the_depth_limit() {
        let out = run("~mission(A)", &[("A", "x ~mission(A)")]);
        assert_eq!(out, "x x x [A]");
    }

    #[test]
    fn a_high_fan_out_is_bounded_by_count_and_size() {
        let bomb = "~mission(A)".repeat(100);
        let out = run(&bomb, &[("A", &bomb)]);
        assert!(out.len() <= MAX_OUTPUT * 2, "{} bytes", out.len());
        assert!(out.ends_with("[A]"));
    }

    #[test]
    fn an_unterminated_token_stays_as_written() {
        assert_eq!(run("a ~mission(B", &[("B", "b")]), "a ~mission(B");
    }

    #[test]
    fn an_empty_token_becomes_empty_brackets() {
        assert_eq!(run("a ~mission() b", &[]), "a [] b");
    }

    #[test]
    fn an_unknown_token_is_written_in_upper_case() {
        assert_eq!(run("~mission(cargo|x)", &[]), "[CARGO]");
    }

    #[test]
    fn a_nested_replacement_is_expanded_and_trimmed() {
        let out = run(
            "<~mission(A)>",
            &[("A", "  in ~mission(B)  "), ("B", " deep ")],
        );
        assert_eq!(out, "<in deep>");
    }
}
