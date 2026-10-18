use std::io::{self, Write};

use crate::xml_text::{is_plain_name, write_display, write_escaped, write_float};
use crate::{Field, RecordTree, Reference, Value};

/// The namespace of the `p4kx:` elements of Record XML.
pub const RECORD_XML_NAMESPACE: &str = "https://github.com/krt-profit/p4k-extractor/record-xml";

const DECLARATION: &[u8] = b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n";
const RESERVED: [&str; 3] = ["guid", "path", "type"];
const INDENT: &[u8; 64] = b"                                                                ";

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
        out.write_all(DECLARATION)?;
        self.write_element(tree, 0, out)
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
        self.attribute.clear();
        self.write_struct(out, Head::Record(tree, true), tree.fields(), depth)
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
        self.attribute.clear();
        self.write_struct(out, Head::Record(tree, false), tree.fields(), depth)
    }

    fn write_struct(
        &mut self,
        out: &mut impl Write,
        head: Head<'_, '_>,
        fields: &[Field<'_>],
        depth: usize,
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
        if let Head::Record(_, true) = head
            && (!plain || children > 0)
        {
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
        }
        for (field, _) in fields
            .iter()
            .zip(flags)
            .filter(|(_, attribute)| **attribute)
        {
            out.write_all(b" ")?;
            out.write_all(field.name().as_bytes())?;
            out.write_all(b"=\"")?;
            write_scalar(out, field.value())?;
            out.write_all(b"\"")?;
        }
        if children == 0 {
            self.attribute.truncate(base);
            return out.write_all(b"/>\n");
        }
        out.write_all(b">\n")?;
        for (index, field) in fields.iter().enumerate() {
            if self.attribute.get(base + index) == Some(&false) {
                self.write_child(out, field, depth + 1)?;
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
    ) -> io::Result<()> {
        let value = field.value();
        indent(out, depth)?;
        if let Value::Error(error) = value {
            out.write_all(b"<p4kx:error field=\"")?;
            write_escaped(out, field.name())?;
            out.write_all(b"\" reason=\"")?;
            write_display(out, error)?;
            return out.write_all(b"\"/>\n");
        }
        if value.is_scalar() {
            out.write_all(b"<p4kx:field name=\"")?;
            write_escaped(out, field.name())?;
            out.write_all(b"\" value=\"")?;
            write_scalar(out, value)?;
            return out.write_all(b"\"/>\n");
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
            self.write_value(out, item, depth + 1)?;
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
    ) -> io::Result<()> {
        match value {
            Value::Struct(structure) => {
                return self.write_struct(
                    out,
                    Head::Nested(structure.type_name()),
                    structure.fields(),
                    depth,
                );
            }
            Value::Array(items) => {
                for item in items {
                    self.write_value(out, item, depth)?;
                }
                return Ok(());
            }
            _ => {}
        }
        indent(out, depth)?;
        match value {
            Value::Reference(reference) => write_reference(out, reference),
            Value::Null => out.write_all(b"<p4kx:null/>\n"),
            Value::Error(error) => {
                out.write_all(b"<p4kx:error reason=\"")?;
                write_display(out, error)?;
                out.write_all(b"\"/>\n")
            }
            scalar => {
                out.write_all(b"<p4kx:value value=\"")?;
                write_scalar(out, scalar)?;
                out.write_all(b"\"/>\n")
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

fn indent(out: &mut impl Write, depth: usize) -> io::Result<()> {
    let mut spaces = depth.saturating_mul(2);
    while spaces > 0 {
        let chunk = spaces.min(INDENT.len());
        out.write_all(INDENT.get(..chunk).unwrap_or_default())?;
        spaces -= chunk;
    }
    Ok(())
}

fn attribute(out: &mut impl Write, name: &str, value: &str) -> io::Result<()> {
    out.write_all(b" ")?;
    out.write_all(name.as_bytes())?;
    out.write_all(b"=\"")?;
    write_escaped(out, value)?;
    out.write_all(b"\"")
}

fn write_reference(out: &mut impl Write, reference: &Reference<'_>) -> io::Result<()> {
    out.write_all(b"<p4kx:reference guid=\"")?;
    write!(out, "{}", reference.guid())?;
    out.write_all(b"\"")?;
    attribute(out, "path", reference.record_path())?;
    attribute(out, "type", reference.type_name())?;
    if let Some(location) = reference.location() {
        attribute(out, "at", location.as_str())?;
    }
    out.write_all(b"/>\n")
}

/// Writes the text of a scalar; other values write nothing.
fn write_scalar(out: &mut impl Write, value: &Value<'_>) -> io::Result<()> {
    match value {
        Value::Bool(value) => out.write_all(if *value { b"true" } else { b"false" }),
        Value::Int8(value) => write!(out, "{value}"),
        Value::Int16(value) => write!(out, "{value}"),
        Value::Int32(value) => write!(out, "{value}"),
        Value::Int64(value) => write!(out, "{value}"),
        Value::UInt8(value) => write!(out, "{value}"),
        Value::UInt16(value) => write!(out, "{value}"),
        Value::UInt32(value) => write!(out, "{value}"),
        Value::UInt64(value) => write!(out, "{value}"),
        Value::Single(value) => write_float(out, *value),
        Value::Double(value) => write_float(out, *value),
        Value::String(text) | Value::Locale(text) | Value::Enum(text) => write_escaped(out, text),
        Value::Guid(guid) => write!(out, "{guid}"),
        Value::Struct(_)
        | Value::Array(_)
        | Value::Reference(_)
        | Value::Null
        | Value::Error(_) => Ok(()),
    }
}
