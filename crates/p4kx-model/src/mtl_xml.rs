// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_cryxml::Element;

/// The read-only view of an XML element the material reader needs, over text XML and `CryXml`.
pub(crate) trait XmlElement<'a>: Copy {
    fn name(self) -> &'a str;

    fn attribute(self, name: &str) -> Option<&'a str>;

    fn children(self) -> impl Iterator<Item = Self>;
}

impl<'a> XmlElement<'a> for Element<'a> {
    fn name(self) -> &'a str {
        Element::name(self)
    }

    fn attribute(self, name: &str) -> Option<&'a str> {
        Element::attribute(self, name)
    }

    fn children(self) -> impl Iterator<Item = Self> {
        Element::children(self)
    }
}

impl<'a> XmlElement<'a> for roxmltree::Node<'a, '_> {
    fn name(self) -> &'a str {
        self.tag_name().name()
    }

    fn attribute(self, name: &str) -> Option<&'a str> {
        roxmltree::Node::attribute(&self, name)
    }

    fn children(self) -> impl Iterator<Item = Self> {
        roxmltree::Node::children(&self).filter(roxmltree::Node::is_element)
    }
}
