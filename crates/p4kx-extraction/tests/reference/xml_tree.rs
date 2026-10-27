// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Tree comparison of two XML documents: tag, attributes in order, trimmed text and children.

use p4kx_extraction::cryxml::Element;

use super::tally::Compared;

const MAX_DEPTH: usize = 256;

pub(crate) const UNFORGE_BUGGED: &str =
    "unforge prints BUGGED for an attribute offset into a string";
pub(crate) const UNFORGE_MOJIBAKE: &str = "unforge decodes multi-byte UTF-8 as single bytes";

/// One element with owned strings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct XmlNode {
    pub(crate) name: String,
    pub(crate) attributes: Vec<(String, String)>,
    pub(crate) text: String,
    pub(crate) children: Vec<Self>,
}

/// The tree is deeper than the comparison follows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TooDeep;

impl XmlNode {
    pub(crate) fn from_cryxml(element: Element<'_>) -> Result<Self, TooDeep> {
        Self::from_cryxml_at(element, 0)
    }

    fn from_cryxml_at(element: Element<'_>, depth: usize) -> Result<Self, TooDeep> {
        if depth > MAX_DEPTH {
            return Err(TooDeep);
        }
        Ok(Self {
            name: element.name().to_owned(),
            attributes: element
                .attributes()
                .map(|attribute| (attribute.name().to_owned(), attribute.value().to_owned()))
                .collect(),
            text: element.content().unwrap_or_default().trim().to_owned(),
            children: element
                .children()
                .map(|child| Self::from_cryxml_at(child, depth + 1))
                .collect::<Result<_, _>>()?,
        })
    }

    pub(crate) fn from_roxml(node: roxmltree::Node<'_, '_>) -> Result<Self, TooDeep> {
        Self::from_roxml_at(node, 0)
    }

    fn from_roxml_at(node: roxmltree::Node<'_, '_>, depth: usize) -> Result<Self, TooDeep> {
        if depth > MAX_DEPTH {
            return Err(TooDeep);
        }
        let text: String = node
            .children()
            .filter(roxmltree::Node::is_text)
            .filter_map(|child| child.text())
            .collect();
        Ok(Self {
            name: node.tag_name().name().to_owned(),
            attributes: node
                .attributes()
                .map(|attribute| (attribute.name().to_owned(), attribute.value().to_owned()))
                .collect(),
            text: text.trim().to_owned(),
            children: node
                .children()
                .filter(roxmltree::Node::is_element)
                .map(|child| Self::from_roxml_at(child, depth + 1))
                .collect::<Result<_, _>>()?,
        })
    }
}

/// Compares `ours` with the tree a reference tool `reference` wrote.
#[must_use]
pub(crate) fn compare(ours: &XmlNode, reference: &XmlNode) -> Compared {
    let mut allowed = None;
    walk(ours, reference, &mut String::new(), &mut allowed).map_or_else(
        || allowed.map_or(Compared::Equal, Compared::Allowed),
        Compared::Different,
    )
}

fn mojibake(text: &str) -> String {
    text.bytes().map(char::from).collect()
}

fn allowance(ours: &str, reference: &str) -> Option<&'static str> {
    if reference == "BUGGED" {
        Some(UNFORGE_BUGGED)
    } else if !ours.is_ascii() && mojibake(ours) == reference {
        Some(UNFORGE_MOJIBAKE)
    } else {
        None
    }
}

fn walk(
    ours: &XmlNode,
    reference: &XmlNode,
    path: &mut String,
    allowed: &mut Option<&'static str>,
) -> Option<String> {
    let length = path.len();
    path.push('/');
    path.push_str(&ours.name);
    let found = element_difference(ours, reference, path, allowed);
    path.truncate(length);
    found
}

fn element_difference(
    ours: &XmlNode,
    reference: &XmlNode,
    path: &mut String,
    allowed: &mut Option<&'static str>,
) -> Option<String> {
    if ours.name != reference.name {
        return Some(format!(
            "{path}: tag {:?} against {:?}",
            ours.name, reference.name
        ));
    }
    if ours.attributes.len() != reference.attributes.len() {
        return Some(format!(
            "{path}: {} attributes against {}",
            ours.attributes.len(),
            reference.attributes.len()
        ));
    }
    for ((name, value), (other_name, other_value)) in
        ours.attributes.iter().zip(&reference.attributes)
    {
        if name != other_name {
            return Some(format!("{path}: attribute {name:?} against {other_name:?}"));
        }
        if value != other_value {
            match allowance(value, other_value) {
                Some(reason) => *allowed = allowed.or(Some(reason)),
                None => {
                    return Some(format!("{path}@{name}: {value:?} against {other_value:?}"));
                }
            }
        }
    }
    if ours.text != reference.text {
        match allowance(&ours.text, &reference.text) {
            Some(reason) => *allowed = allowed.or(Some(reason)),
            None => {
                return Some(format!(
                    "{path}: text {:?} against {:?}",
                    ours.text, reference.text
                ));
            }
        }
    }
    if ours.children.len() != reference.children.len() {
        return Some(format!(
            "{path}: {} children against {}",
            ours.children.len(),
            reference.children.len()
        ));
    }
    ours.children
        .iter()
        .zip(&reference.children)
        .find_map(|(child, other)| walk(child, other, path, allowed))
}
