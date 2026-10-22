use std::borrow::Cow;

use crate::contract::LocalisedText;
use crate::texts::Texts;

const OPEN: &str = "~mission(";
const MAX_DEPTH: usize = 3;

/// What a `~mission(Token)` in a text is replaced by.
pub(crate) enum Token<'a> {
    /// The text of this localisation key, in the language being written.
    Key(&'a str),
    /// A value the game picks when the mission starts, written as `[NAME]`.
    Placeholder(&'a str),
}

/// Returns `key` as a [`LocalisedText`] whose texts have every `~mission(Token[|Variant])`
/// replaced by what `resolve` says for the token and the variant, with leading and trailing
/// blanks removed. A token `resolve` returns `None` for, or whose key has no text in the
/// language, becomes `[TOKEN]` in upper case.
pub(crate) fn title<'a>(
    key: Option<&'a str>,
    texts: &Texts<'a>,
    resolve: &dyn Fn(&str, Option<&str>) -> Option<Token<'a>>,
) -> LocalisedText<'a> {
    let mut title = texts.localised(key);
    title.en = title
        .en
        .take()
        .map(|text| render(text, english, texts, resolve));
    title.de = title
        .de
        .take()
        .map(|text| render(text, german, texts, resolve));
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
    pick: for<'x> fn(&'x LocalisedText<'a>) -> Option<&'x str>,
    texts: &Texts<'a>,
    resolve: &dyn Fn(&str, Option<&str>) -> Option<Token<'a>>,
) -> Cow<'a, str> {
    if !text.contains(OPEN) {
        return trimmed(text);
    }
    let mut out = String::with_capacity(text.len());
    substitute(&mut out, &text, 0, pick, texts, resolve);
    trimmed(Cow::Owned(out))
}

fn substitute<'a>(
    out: &mut String,
    text: &str,
    depth: usize,
    pick: for<'x> fn(&'x LocalisedText<'a>) -> Option<&'x str>,
    texts: &Texts<'a>,
    resolve: &dyn Fn(&str, Option<&str>) -> Option<Token<'a>>,
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
            expand(out, name, variant, depth, pick, texts, resolve);
            rest = tail;
        } else {
            out.push_str(OPEN);
            rest = after;
        }
    }
    out.push_str(rest);
}

fn expand<'a>(
    out: &mut String,
    name: &str,
    variant: Option<&str>,
    depth: usize,
    pick: for<'x> fn(&'x LocalisedText<'a>) -> Option<&'x str>,
    texts: &Texts<'a>,
    resolve: &dyn Fn(&str, Option<&str>) -> Option<Token<'a>>,
) {
    match resolve(name, variant) {
        Some(Token::Key(key)) => {
            let text = texts.localised(Some(key));
            if let Some(found) = pick(&text).filter(|found| !found.is_empty()) {
                let found = found.trim();
                if depth < MAX_DEPTH && found.contains(OPEN) {
                    substitute(out, found, depth + 1, pick, texts, resolve);
                } else {
                    out.push_str(found);
                }
                return;
            }
        }
        Some(Token::Placeholder(label)) => {
            out.push('[');
            out.push_str(label);
            out.push(']');
            return;
        }
        None => {}
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
