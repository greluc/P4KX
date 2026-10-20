use std::collections::HashMap;

const NO_PARENT: u32 = u32::MAX;
const HEADER_FIELDS: usize = 9;
const NODE_LEN: usize = 28;
const ATTRIBUTE_LEN: usize = 8;
const CHILD_LEN: usize = 4;

/// The signature a synthetic `CryXml` file starts with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CryXmlSignature {
    /// `CryXmlB` and a zero byte.
    CryXmlB,
    /// `CryXml` and two zero bytes.
    CryXml,
    /// `CRY3SDK` and two bytes.
    Cry3Sdk,
}

impl CryXmlSignature {
    const fn prefix(self) -> &'static [u8] {
        match self {
            Self::CryXmlB => b"CryXmlB\0",
            Self::CryXml => b"CryXml\0\0",
            Self::Cry3Sdk => b"CRY3SDK\0\0",
        }
    }
}

/// The byte order of the integers in a synthetic `CryXml` file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CryXmlByteOrder {
    /// Little-endian, as in current Archives.
    Little,
    /// Big-endian, as written for console builds.
    Big,
}

/// One element of a synthetic `CryXml` document.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TestElement {
    name: String,
    attributes: Vec<(String, String)>,
    content: String,
    children: Vec<Self>,
}

impl TestElement {
    /// Creates an element without attributes, content or children.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Self::default()
        }
    }

    /// Adds an attribute after the existing ones.
    #[must_use]
    pub fn attribute(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.push((name.into(), value.into()));
        self
    }

    /// Sets the text content.
    #[must_use]
    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.content = content.into();
        self
    }

    /// Adds a child after the existing ones.
    #[must_use]
    pub fn child(mut self, child: Self) -> Self {
        self.children.push(child);
        self
    }

    /// Writes this element as the root of a text XML document.
    #[must_use]
    pub fn to_xml(&self) -> String {
        let mut out = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        self.write_xml(&mut out);
        out
    }

    fn write_xml(&self, out: &mut String) {
        out.push('<');
        out.push_str(&self.name);
        for (name, value) in &self.attributes {
            out.push(' ');
            out.push_str(name);
            out.push_str("=\"");
            out.push_str(
                &value
                    .replace('&', "&amp;")
                    .replace('<', "&lt;")
                    .replace('"', "&quot;"),
            );
            out.push('"');
        }
        if self.children.is_empty() && self.content.is_empty() {
            out.push_str("/>");
            return;
        }
        out.push('>');
        out.push_str(&self.content.replace('&', "&amp;").replace('<', "&lt;"));
        for child in &self.children {
            child.write_xml(out);
        }
        out.push_str("</");
        out.push_str(&self.name);
        out.push('>');
    }

    /// Encodes this element as the root of a `CryXml` file, laid out like the game's files: header,
    /// node table, child table, attribute table and string data, with deduplicated strings.
    #[must_use]
    pub fn to_cryxml(&self, signature: CryXmlSignature, byte_order: CryXmlByteOrder) -> Vec<u8> {
        let mut tables = Tables::default();
        tables.add(self, NO_PARENT);
        tables.encode(signature, byte_order)
    }
}

struct Node {
    name: u32,
    content: u32,
    attribute_count: u16,
    child_count: u16,
    parent: u32,
    first_attribute: u32,
    first_child: u32,
}

#[derive(Default)]
struct Tables {
    nodes: Vec<Node>,
    attributes: Vec<(u32, u32)>,
    children: Vec<u32>,
    strings: Vec<u8>,
    offsets: HashMap<String, u32>,
}

impl Tables {
    fn add(&mut self, element: &TestElement, parent: u32) {
        let index = count(self.nodes.len());
        let name = self.string(&element.name);
        let content = self.string(&element.content);
        let first_attribute = count(self.attributes.len());
        for (key, value) in &element.attributes {
            let pair = (self.string(key), self.string(value));
            self.attributes.push(pair);
        }
        let first_child = self.children.len();
        self.children
            .resize(first_child + element.children.len(), NO_PARENT);
        self.nodes.push(Node {
            name,
            content,
            attribute_count: u16::try_from(element.attributes.len()).unwrap_or(u16::MAX),
            child_count: u16::try_from(element.children.len()).unwrap_or(u16::MAX),
            parent,
            first_attribute,
            first_child: count(first_child),
        });
        for (slot, child) in (first_child..).zip(&element.children) {
            let child_index = count(self.nodes.len());
            if let Some(entry) = self.children.get_mut(slot) {
                *entry = child_index;
            }
            self.add(child, index);
        }
    }

    fn string(&mut self, text: &str) -> u32 {
        if let Some(&offset) = self.offsets.get(text) {
            return offset;
        }
        let offset = count(self.strings.len());
        self.strings.extend_from_slice(text.as_bytes());
        self.strings.push(0);
        self.offsets.insert(text.to_owned(), offset);
        offset
    }

    fn encode(&self, signature: CryXmlSignature, byte_order: CryXmlByteOrder) -> Vec<u8> {
        let prefix = signature.prefix();
        let node_position = prefix.len() + HEADER_FIELDS * 4;
        let child_position = node_position + self.nodes.len() * NODE_LEN;
        let attribute_position = child_position + self.children.len() * CHILD_LEN;
        let string_position = attribute_position + self.attributes.len() * ATTRIBUTE_LEN;
        let total = string_position + self.strings.len();

        let mut out = Writer {
            bytes: Vec::with_capacity(total),
            byte_order,
        };
        out.bytes.extend_from_slice(prefix);
        for field in [
            total,
            node_position,
            self.nodes.len(),
            attribute_position,
            self.attributes.len(),
            child_position,
            self.children.len(),
            string_position,
            self.strings.len(),
        ] {
            out.u32(count(field));
        }
        for node in &self.nodes {
            out.u32(node.name);
            out.u32(node.content);
            out.u16(node.attribute_count);
            out.u16(node.child_count);
            out.u32(node.parent);
            out.u32(node.first_attribute);
            out.u32(node.first_child);
            out.u32(0);
        }
        for &child in &self.children {
            out.u32(child);
        }
        for &(name, value) in &self.attributes {
            out.u32(name);
            out.u32(value);
        }
        out.bytes.extend_from_slice(&self.strings);
        out.bytes
    }
}

struct Writer {
    bytes: Vec<u8>,
    byte_order: CryXmlByteOrder,
}

impl Writer {
    fn u16(&mut self, value: u16) {
        let bytes = match self.byte_order {
            CryXmlByteOrder::Little => value.to_le_bytes(),
            CryXmlByteOrder::Big => value.to_be_bytes(),
        };
        self.bytes.extend_from_slice(&bytes);
    }

    fn u32(&mut self, value: u32) {
        let bytes = match self.byte_order {
            CryXmlByteOrder::Little => value.to_le_bytes(),
            CryXmlByteOrder::Big => value.to_be_bytes(),
        };
        self.bytes.extend_from_slice(&bytes);
    }
}

fn count(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}
