// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::fmt;

use p4kx_foundation::ParseError;
use thiserror::Error;

/// A table of a `CryXml` file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Table {
    /// The node table, 28 bytes per element.
    Nodes,
    /// The attribute table, a name and a value offset per attribute.
    Attributes,
    /// The child table, one node index per child.
    Children,
    /// The string data all names, values and contents point into.
    Strings,
}

impl fmt::Display for Table {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Nodes => "node",
            Self::Attributes => "attribute",
            Self::Children => "child",
            Self::Strings => "string",
        })
    }
}

/// A failure to read `CryXml`.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CryXmlError {
    /// The input has no `CryXml` signature and is not text XML.
    #[error("not CryXml: no CryXmlB, CryXml or CRY3SDK signature and no text XML")]
    NotCryXml,
    /// The header could not be read.
    #[error("corrupt CryXml header: {0}")]
    Header(#[from] ParseError),
    /// The header declares more bytes than the input holds.
    #[error("truncated CryXml: the header declares {declared} bytes, {available} are present")]
    Truncated {
        /// The size the header declares.
        declared: u64,
        /// The size of the input.
        available: u64,
    },
    /// A table does not lie within the data.
    #[error(
        "corrupt CryXml: the {table} table at offset {position} with {count} entries lies outside the data"
    )]
    TableOutOfBounds {
        /// The table.
        table: Table,
        /// Its offset in the file.
        position: u32,
        /// Its number of entries, or of bytes for the string data.
        count: u32,
    },
    /// A string offset lies outside the string data, or its string has no terminating zero.
    #[error(
        "corrupt CryXml: string offset {offset} lies outside the string data or is unterminated"
    )]
    BadString {
        /// The offset into the string data.
        offset: u32,
    },
    /// A node's attributes lie outside the attribute table.
    #[error(
        "corrupt CryXml: node {node} has attributes {first}..{first}+{count} outside the attribute table"
    )]
    BadAttributes {
        /// The index of the node.
        node: u32,
        /// The index of its first attribute.
        first: u32,
        /// Its number of attributes.
        count: u16,
    },
    /// The first node has a parent, or another node's parent does not precede it.
    #[error("corrupt CryXml: node {node} has the invalid parent {parent}")]
    BadParent {
        /// The index of the node.
        node: u32,
        /// The parent index it names.
        parent: u32,
    },
    /// The node table is empty, so there is no root element.
    #[error("corrupt CryXml: the node table is empty")]
    NoRoot,
}
