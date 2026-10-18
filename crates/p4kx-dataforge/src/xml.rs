use std::io::{self, Write};

use crate::xml_text::{is_plain_name, write_display, write_escaped, write_float};
use crate::{Field, RecordTree, Value};

/// The namespace of the `p4kx:` elements of Record XML.
pub const RECORD_XML_NAMESPACE: &str = "https://github.com/krt-profit/p4k-extractor/record-xml";

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

impl RecordXml {
    /// Creates a writer.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Writes `tree` as one Record XML document to `out`.
    ///
    /// # Errors
    ///
    /// Returns the I/O error of `out`.
    pub fn write<W: Write>(&mut self, tree: &RecordTree<'_>, out: &mut W) -> io::Result<()> {
        self.classify(tree.fields());
        let plain_root = is_plain_name(tree.type_name());
        let children = self
            .attribute
            .iter()
            .filter(|attribute| !**attribute)
            .count();
        out.write_all(DECLARATION)?;
        if plain_root {
            out.write_all(b"<")?;
            out.write_all(tree.type_name().as_bytes())?;
        } else {
            out.write_all(b"<p4kx:record")?;
        }
        if !plain_root || children > 0 {
            attribute(out, "xmlns:p4kx", RECORD_XML_NAMESPACE)?;
        }
        if !plain_root {
            attribute(out, "type", tree.type_name())?;
        }
        out.write_all(b" guid=\"")?;
        write!(out, "{}", tree.guid())?;
        out.write_all(b"\"")?;
        attribute(out, "path", tree.record_path())?;
        for (field, _) in self.fields(tree, true) {
            out.write_all(b" ")?;
            out.write_all(field.name().as_bytes())?;
            out.write_all(b"=\"")?;
            write_value(out, field.value())?;
            out.write_all(b"\"")?;
        }
        if children == 0 {
            return out.write_all(b"/>\n");
        }
        out.write_all(b">\n")?;
        for (field, _) in self.fields(tree, false) {
            write_child(out, field)?;
        }
        if plain_root {
            out.write_all(b"</")?;
            out.write_all(tree.type_name().as_bytes())?;
            out.write_all(b">\n")
        } else {
            out.write_all(b"</p4kx:record>\n")
        }
    }

    /// Decides for every field whether it is written as an attribute: a scalar whose name is a
    /// plain XML name, not reserved and not taken by an earlier field.
    fn classify(&mut self, fields: &[Field<'_>]) {
        self.attribute.clear();
        self.attribute.extend(fields.iter().map(|field| {
            !matches!(field.value(), Value::Error(_))
                && is_plain_name(field.name())
                && !RESERVED.contains(&field.name())
        }));
        self.order.clear();
        self.order.extend(
            self.attribute
                .iter()
                .enumerate()
                .filter_map(|(index, attribute)| attribute.then_some(index)),
        );
        let name = |index: &usize| fields.get(*index).map(Field::name);
        self.order
            .sort_unstable_by(|left, right| name(left).cmp(&name(right)).then(left.cmp(right)));
        for pair in self.order.windows(2) {
            if let [first, second] = pair
                && name(first) == name(second)
                && let Some(attribute) = self.attribute.get_mut(*second)
            {
                *attribute = false;
            }
        }
    }

    fn fields<'t, 'f>(
        &'t self,
        tree: &'t RecordTree<'f>,
        attributes: bool,
    ) -> impl Iterator<Item = (&'t Field<'f>, bool)> + use<'t, 'f> {
        tree.fields()
            .iter()
            .zip(self.attribute.iter().copied())
            .filter(move |(_, attribute)| *attribute == attributes)
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

fn attribute(out: &mut impl Write, name: &str, value: &str) -> io::Result<()> {
    out.write_all(b" ")?;
    out.write_all(name.as_bytes())?;
    out.write_all(b"=\"")?;
    write_escaped(out, value)?;
    out.write_all(b"\"")
}

fn write_child(out: &mut impl Write, field: &Field<'_>) -> io::Result<()> {
    if let Value::Error(error) = field.value() {
        out.write_all(b"  <p4kx:error field=\"")?;
        write_escaped(out, field.name())?;
        out.write_all(b"\" reason=\"")?;
        write_display(out, error)?;
    } else {
        out.write_all(b"  <p4kx:field name=\"")?;
        write_escaped(out, field.name())?;
        out.write_all(b"\" value=\"")?;
        write_value(out, field.value())?;
    }
    out.write_all(b"\"/>\n")
}

fn write_value(out: &mut impl Write, value: &Value<'_>) -> io::Result<()> {
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
        Value::Error(error) => write_display(out, error),
    }
}
