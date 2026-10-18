use std::borrow::Cow;
use std::iter::FusedIterator;
use std::slice;

use crate::document::Document;

/// One attribute of an element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attribute<'a> {
    name: Cow<'a, str>,
    value: Cow<'a, str>,
}

impl<'a> Attribute<'a> {
    pub(crate) const fn new(name: Cow<'a, str>, value: Cow<'a, str>) -> Self {
        Self { name, value }
    }

    /// Returns the attribute's name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the attribute's value.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }
}

/// One element of a [`Document`].
#[derive(Debug, Clone, Copy)]
pub struct Element<'d, 'a> {
    document: &'d Document<'a>,
    index: u32,
}

impl<'d, 'a> Element<'d, 'a> {
    pub(crate) const fn new(document: &'d Document<'a>, index: u32) -> Self {
        Self { document, index }
    }

    /// Returns the element's name.
    #[must_use]
    pub fn name(self) -> &'d str {
        self.document
            .nodes
            .get(self.index as usize)
            .map_or("", |node| &node.name)
    }

    /// Returns the element's text content, or `None` when it has none.
    #[must_use]
    pub fn content(self) -> Option<&'d str> {
        self.document
            .nodes
            .get(self.index as usize)
            .map(|node| &*node.content)
            .filter(|content| !content.is_empty())
    }

    /// Returns the element's attributes in file order.
    #[must_use]
    pub fn attributes(self) -> &'d [Attribute<'a>] {
        self.document
            .nodes
            .get(self.index as usize)
            .and_then(|node| {
                let first = node.first_attribute as usize;
                self.document
                    .attributes
                    .get(first..first + usize::from(node.attribute_count))
            })
            .unwrap_or_default()
    }

    /// Returns the value of the first attribute named `name`.
    #[must_use]
    pub fn attribute(self, name: &str) -> Option<&'d str> {
        self.attributes()
            .iter()
            .find(|attribute| attribute.name() == name)
            .map(|attribute| &*attribute.value)
    }

    /// Returns the element's children in file order.
    #[must_use]
    pub fn children(self) -> Children<'d, 'a> {
        let index = self.index as usize;
        let start = self.document.child_starts.get(index).copied();
        let end = self.document.child_starts.get(index + 1).copied();
        let ids = start
            .zip(end)
            .and_then(|(start, end)| self.document.children.get(start as usize..end as usize))
            .unwrap_or_default();
        Children {
            document: self.document,
            ids: ids.iter(),
        }
    }

    /// Returns whether the element has children.
    #[must_use]
    pub fn has_children(self) -> bool {
        self.children().len() > 0
    }
}

/// The children of an [`Element`].
#[derive(Debug, Clone)]
pub struct Children<'d, 'a> {
    document: &'d Document<'a>,
    ids: slice::Iter<'d, u32>,
}

impl<'d, 'a> Iterator for Children<'d, 'a> {
    type Item = Element<'d, 'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.ids
            .next()
            .map(|&index| Element::new(self.document, index))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.ids.size_hint()
    }
}

impl DoubleEndedIterator for Children<'_, '_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.ids
            .next_back()
            .map(|&index| Element::new(self.document, index))
    }
}

impl ExactSizeIterator for Children<'_, '_> {}

impl FusedIterator for Children<'_, '_> {}
