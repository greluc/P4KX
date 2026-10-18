//! Reads `CryXml`, the binary XML encoding `CryEngine` uses for many Entries, and writes it as
//! UTF-8 XML.
//!
//! [`detect`] recognises the `CryXmlB`, `CryXml` and `CRY3SDK` signatures and text XML; [`parse`]
//! turns a `CryXml` byte slice of either byte order into a [`Document`] that borrows its strings
//! from the input; [`read`] does both and passes text XML through unchanged.

mod document;
mod element;
mod error;
mod escape;
mod format;
mod header;
mod read;
mod strings;
mod write;

pub use document::{Document, parse};
pub use element::{Attribute, Children, Element};
pub use error::{CryXmlError, Table};
pub use format::{ByteOrder, Format, Signature, detect};
pub use read::{Xml, read};
