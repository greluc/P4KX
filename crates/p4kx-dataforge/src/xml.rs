// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::io::{self, Write};

use crate::xml_text::{is_plain_name, write_escaped};
use crate::xml_write::{
    attribute, indent, write_annotation, write_attribute_field, write_error_end, write_reference,
    write_scalar,
};
use crate::{Annotations, Field, RecordTree, Value};

/// The namespace of the `p4kx:` elements of Record XML.
pub const RECORD_XML_NAMESPACE: &str = "https://github.com/p4kx-org/p4kx/record-xml";

const DECLARATION: &[u8] = b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n";
const RESERVED: [&str; 3] = ["guid", "path", "type"];

/// Writes Record trees as Record XML, the format `docs/record-xml.md` defines.
///
/// One writer can be reused for many Records; it keeps its scratch space between them. The same
/// tree always gives the same bytes.
#[derive(Debug, Default)]
pub struct RecordXml {
    order: Vec<usize>,
    attribute: Vec<bool>,
}

/// The start of a struct element: the root of a Record or a nested struct.
#[derive(Clone, Copy)]
enum Head<'t, 'f> {
    Record(&'t RecordTree<'f>, bool),
    Nested(&'f str),
}

impl RecordXml {
    /// Creates a writer.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Writes `tree` as one Record XML document to `out`: the XML declaration, then the root
    /// element.
    ///
    /// # Errors
    ///
    /// Returns the I/O error of `out`.
    pub fn write<W: Write>(&mut self, tree: &RecordTree<'_>, out: &mut W) -> io::Result<()> {
        self.write_annotated(tree, Annotations::none(), out)
    }

    /// Writes `tree` like [`RecordXml::write`] with the `annotations`: the texts of localisation
    /// keys, each with its resolved text or a mark that it has none, and the tag path of every
    /// tag Reference in a `tag` attribute, see `docs/record-xml.md`.
    ///
    /// # Errors
    ///
    /// Returns the I/O error of `out`.
    pub fn write_annotated<W: Write>(
        &mut self,
        tree: &RecordTree<'_>,
        annotations: Annotations<'_>,
        out: &mut W,
    ) -> io::Result<()> {
        out.write_all(DECLARATION)?;
        self.write_element_annotated(tree, annotations, 0, out)
    }

    /// Writes the root element of `tree` without the XML declaration, indented by `depth` levels
    /// of two spaces, e.g. to place many Records in one document.
    ///
    /// # Errors
    ///
    /// Returns the I/O error of `out`.
    pub fn write_element<W: Write>(
        &mut self,
        tree: &RecordTree<'_>,
        depth: usize,
        out: &mut W,
    ) -> io::Result<()> {
        self.write_element_annotated(tree, Annotations::none(), depth, out)
    }

    /// Writes the root element like [`RecordXml::write_element`], with the `annotations` of
    /// [`RecordXml::write_annotated`].
    ///
    /// # Errors
    ///
    /// Returns the I/O error of `out`.
    pub fn write_element_annotated<W: Write>(
        &mut self,
        tree: &RecordTree<'_>,
        annotations: Annotations<'_>,
        depth: usize,
        out: &mut W,
    ) -> io::Result<()> {
        self.attribute.clear();
        let head = Head::Record(tree, true);
        self.write_struct(out, head, tree.fields(), depth, annotations)
    }

    /// Writes the root element of `tree` like [`RecordXml::write_element`], but without the
    /// `xmlns:p4kx` declaration, for a wrapper element that declares [`RECORD_XML_NAMESPACE`]
    /// itself.
    ///
    /// # Errors
    ///
    /// Returns the I/O error of `out`.
    pub fn write_wrapped_element<W: Write>(
        &mut self,
        tree: &RecordTree<'_>,
        depth: usize,
        out: &mut W,
    ) -> io::Result<()> {
        self.write_wrapped_element_annotated(tree, Annotations::none(), depth, out)
    }

    /// Writes the wrapped root element like [`RecordXml::write_wrapped_element`], with the
    /// `annotations` of [`RecordXml::write_annotated`]; the wrapper declares the namespace.
    ///
    /// # Errors
    ///
    /// Returns the I/O error of `out`.
    pub fn write_wrapped_element_annotated<W: Write>(
        &mut self,
        tree: &RecordTree<'_>,
        annotations: Annotations<'_>,
        depth: usize,
        out: &mut W,
    ) -> io::Result<()> {
        self.attribute.clear();
        let head = Head::Record(tree, false);
        self.write_struct(out, head, tree.fields(), depth, annotations)
    }

    fn write_struct(
        &mut self,
        out: &mut impl Write,
        head: Head<'_, '_>,
        fields: &[Field<'_>],
        depth: usize,
        annotations: Annotations<'_>,
    ) -> io::Result<()> {
        let base = self.attribute.len();
        self.classify(fields);
        let flags = self.attribute.get(base..).unwrap_or_default();
        let children = flags.iter().filter(|attribute| !**attribute).count();
        let (type_name, fallback) = match head {
            Head::Record(tree, _) => (tree.type_name(), "p4kx:record"),
            Head::Nested(type_name) => (type_name, "p4kx:struct"),
        };
        let plain = is_plain_name(type_name);
        indent(out, depth)?;
        out.write_all(b"<")?;
        out.write_all(if plain { type_name } else { fallback }.as_bytes())?;
        if let Head::Record(_, true) = head {
            attribute(out, "xmlns:p4kx", RECORD_XML_NAMESPACE)?;
        }
        if !plain {
            attribute(out, "type", type_name)?;
        }
        if let Head::Record(tree, _) = head {
            out.write_all(b" guid=\"")?;
            write!(out, "{}", tree.guid())?;
            out.write_all(b"\"")?;
            attribute(out, "path", tree.record_path())?;
            attribute(out, "p4kx:name", tree.name())?;
            if let Some(tag) = tree.tag() {
                attribute(out, "p4kx:tag", tag)?;
            }
        }
        for (field, _) in fields
            .iter()
            .zip(flags)
            .filter(|(_, attribute)| **attribute)
        {
            write_attribute_field(out, field, annotations)?;
        }
        if children == 0 {
            self.attribute.truncate(base);
            return out.write_all(b"/>\n");
        }
        out.write_all(b">\n")?;
        for (index, field) in fields.iter().enumerate() {
            if self.attribute.get(base + index) == Some(&false) {
                self.write_child(out, field, depth + 1, annotations)?;
            }
        }
        self.attribute.truncate(base);
        indent(out, depth)?;
        out.write_all(b"</")?;
        out.write_all(if plain { type_name } else { fallback }.as_bytes())?;
        out.write_all(b">\n")
    }

    fn classify(&mut self, fields: &[Field<'_>]) {
        classify(fields, &mut self.attribute, &mut self.order);
    }

    fn write_child(
        &mut self,
        out: &mut impl Write,
        field: &Field<'_>,
        depth: usize,
        annotations: Annotations<'_>,
    ) -> io::Result<()> {
        let value = field.value();
        indent(out, depth)?;
        if let Value::Error(error) = value {
            out.write_all(b"<p4kx:error field=\"")?;
            write_escaped(out, field.name())?;
            out.write_all(b"\"")?;
            return write_error_end(out, error);
        }
        if value.is_scalar() {
            out.write_all(b"<p4kx:field name=\"")?;
            write_escaped(out, field.name())?;
            out.write_all(b"\" value=\"")?;
            write_scalar(out, value)?;
            out.write_all(b"\"")?;
            return write_annotation(out, value, annotations, b"/>\n");
        }
        let plain = is_plain_name(field.name());
        if plain {
            out.write_all(b"<")?;
            out.write_all(field.name().as_bytes())?;
        } else {
            out.write_all(b"<p4kx:field name=\"")?;
            write_escaped(out, field.name())?;
            out.write_all(b"\"")?;
        }
        let items = match value {
            Value::Array(items) => &items[..],
            single => std::slice::from_ref(single),
        };
        if items.is_empty() {
            return out.write_all(b"/>\n");
        }
        out.write_all(b">\n")?;
        for item in items {
            self.write_value(out, item, depth + 1, annotations)?;
        }
        indent(out, depth)?;
        if plain {
            out.write_all(b"</")?;
            out.write_all(field.name().as_bytes())?;
            out.write_all(b">\n")
        } else {
            out.write_all(b"</p4kx:field>\n")
        }
    }

    fn write_value(
        &mut self,
        out: &mut impl Write,
        value: &Value<'_>,
        depth: usize,
        annotations: Annotations<'_>,
    ) -> io::Result<()> {
        match value {
            Value::Struct(structure) => {
                return self.write_struct(
                    out,
                    Head::Nested(structure.type_name()),
                    structure.fields(),
                    depth,
                    annotations,
                );
            }
            Value::Array(items) => {
                for item in items {
                    self.write_value(out, item, depth, annotations)?;
                }
                return Ok(());
            }
            _ => {}
        }
        indent(out, depth)?;
        match value {
            Value::Reference(reference) => write_reference(out, reference, annotations),
            Value::Null => out.write_all(b"<p4kx:null/>\n"),
            Value::Error(error) => {
                out.write_all(b"<p4kx:error")?;
                write_error_end(out, error)
            }
            scalar => {
                out.write_all(b"<p4kx:value value=\"")?;
                write_scalar(out, scalar)?;
                out.write_all(b"\"")?;
                write_annotation(out, scalar, annotations, b"/>\n")
            }
        }
    }
}

impl RecordTree<'_> {
    /// Writes the Record as one Record XML document to `out`.
    ///
    /// Use a [`RecordXml`] instead to write many Records with one scratch space.
    ///
    /// # Errors
    ///
    /// Returns the I/O error of `out`.
    pub fn write_xml<W: Write>(&self, out: &mut W) -> io::Result<()> {
        RecordXml::new().write(self, out)
    }
}

/// Pushes onto `attribute` for every field of `fields` whether Record XML writes it as an
/// attribute: a scalar whose name is a plain XML name, not reserved and not taken by an earlier
/// such field. `order` is scratch space.
pub(crate) fn classify(fields: &[Field<'_>], attribute: &mut Vec<bool>, order: &mut Vec<usize>) {
    let base = attribute.len();
    attribute.extend(fields.iter().map(|field| {
        field.value().is_scalar()
            && is_plain_name(field.name())
            && !RESERVED.contains(&field.name())
    }));
    order.clear();
    order.extend(
        attribute
            .get(base..)
            .unwrap_or_default()
            .iter()
            .enumerate()
            .filter_map(|(index, attribute)| attribute.then_some(index)),
    );
    let name = |index: &usize| fields.get(*index).map(Field::name);
    order.sort_unstable_by(|left, right| name(left).cmp(&name(right)).then(left.cmp(right)));
    for pair in order.windows(2) {
        if let [first, second] = pair
            && name(first) == name(second)
            && let Some(flag) = attribute.get_mut(base + *second)
        {
            *flag = false;
        }
    }
}
