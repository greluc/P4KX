// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

const MAX_RULES: usize = 1024;
const MAX_DECLARATIONS: usize = 64;

#[derive(Debug, PartialEq, Eq)]
enum Selector {
    All,
    Tag(String),
    Class(String),
    Id(String),
}

/// The simple rules of the `<style>` elements of a document: universal, tag, class and id
/// selectors with plain declarations.
#[derive(Debug, Default)]
pub(super) struct Sheet {
    rules: Vec<(Selector, Vec<(String, String)>)>,
}

/// Splits `a:b;c:d` into declarations, at most [`MAX_DECLARATIONS`].
pub(super) fn declarations(text: &str) -> Vec<(String, String)> {
    text.split(';')
        .filter_map(|part| part.split_once(':'))
        .map(|(name, value)| (name.trim().to_ascii_lowercase(), value.trim().to_owned()))
        .filter(|(name, _)| !name.is_empty())
        .take(MAX_DECLARATIONS)
        .collect()
}

fn without_comments(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("/*") {
        out.push_str(rest.get(..start).unwrap_or_default());
        let after = rest.get(start + 2..).unwrap_or_default();
        match after.find("*/") {
            Some(end) => rest = after.get(end + 2..).unwrap_or_default(),
            None => return out,
        }
    }
    out.push_str(rest);
    out
}

fn selector(text: &str) -> Option<Selector> {
    let text = text.trim();
    if text == "*" {
        return Some(Selector::All);
    }
    let name = |name: &str| {
        (!name.is_empty()
            && name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_')))
        .then(|| name.to_owned())
    };
    if let Some(class) = text.strip_prefix('.') {
        return name(class).map(Selector::Class);
    }
    if let Some(id) = text.strip_prefix('#') {
        return name(id).map(Selector::Id);
    }
    name(text).map(Selector::Tag)
}

impl Sheet {
    pub(super) fn add(&mut self, css: &str) {
        let css = without_comments(css);
        for block in css.split('}') {
            let Some((selectors, body)) = block.split_once('{') else {
                continue;
            };
            let declared = declarations(body);
            for selector in selectors.split(',').filter_map(selector) {
                if self.rules.len() >= MAX_RULES {
                    return;
                }
                self.rules.push((selector, declared.clone()));
            }
        }
    }

    /// Returns the declarations that apply to an element, in the order of the rules.
    pub(super) fn matching<'s>(
        &'s self,
        tag: &str,
        class: &str,
        id: &str,
    ) -> impl Iterator<Item = &'s (String, String)> {
        let classes: Vec<&str> = class.split_whitespace().collect();
        let tag = tag.to_owned();
        let id = id.to_owned();
        self.rules
            .iter()
            .filter(move |(selector, _)| match selector {
                Selector::All => true,
                Selector::Tag(name) => *name == tag,
                Selector::Class(name) => classes.contains(&name.as_str()),
                Selector::Id(name) => *name == id,
            })
            .flat_map(|(_, declared)| declared.iter())
    }
}
