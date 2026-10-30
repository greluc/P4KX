// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::io::{self, Write};

use crate::json_text::new_line;
use crate::texts::annotation;
use crate::value_json::{write_json_string as write_string, write_json_value};
use crate::xml::classify;
use crate::{Annotations, Field, RecordTree, Reference, Resolution, Value};

/// How Record JSON is laid out; both layouts hold the same value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum JsonLayout {
    /// Indented by two spaces per level, one key or element per line, with a final line ending.
    #[default]
    Pretty,
    /// One line without whitespace or line ending, e.g. for NDJSON.
    Compact,
}

/// Writes Record trees as Record JSON, the format `docs/record-json.md` defines.
///
/// One writer can be reused for many Records; it keeps its scratch space between them. The same
/// tree always gives the same bytes.
#[derive(Debug, Default)]
pub struct RecordJson {
    layout: JsonLayout,
    attribute: Vec<bool>,
    escaped: Vec<bool>,
    order: Vec<usize>,
}

/// The output of one Record, its layout and its texts.
#[expect(
    clippy::struct_field_names,
    reason = "`out` is the writer, read as `out.out` throughout"
)]
struct Out<'w, W> {
    out: &'w mut W,
    pretty: bool,
    annotations: Annotations<'w>,
}

impl RecordJson {
    /// Creates a writer for `layout`.
    #[must_use]
    pub fn new(layout: JsonLayout) -> Self {
        Self {
            layout,
            ..Self::default()
        }
    }

    /// Returns the layout the writer writes.
    #[must_use]
    pub const fn layout(&self) -> JsonLayout {
        self.layout
    }

    /// Writes `tree` as one Record JSON object to `out`.
    ///
    /// # Errors
    ///
    /// Returns the I/O error of `out`.
    pub fn write<W: Write>(&mut self, tree: &RecordTree<'_>, out: &mut W) -> io::Result<()> {
        self.write_annotated(tree, Annotations::none(), out)
    }

    /// Writes `tree` like [`RecordJson::write`] with the `annotations`: every localisation key is
    /// an object with the key and its resolved text or a mark that it has none, and the tag path
    /// of every tag Reference is `tag`, see `docs/record-json.md`.
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
        self.attribute.clear();
        self.escaped.clear();
        let pretty = self.layout == JsonLayout::Pretty;
        let mut out = Out {
            out,
            pretty,
            annotations,
        };
        self.write_struct(&mut out, Some(tree), tree.type_name(), tree.fields(), 0)?;
        if pretty {
            out.out.write_all(b"\n")?;
        }
        Ok(())
    }

    fn write_struct<W: Write>(
        &mut self,
        out: &mut Out<'_, W>,
        record: Option<&RecordTree<'_>>,
        type_name: &str,
        fields: &[Field<'_>],
        depth: usize,
    ) -> io::Result<()> {
        out.out.write_all(b"{")?;
        out.key(true, depth + 1, "$type")?;
        write_string(out.out, type_name)?;
        if let Some(tree) = record {
            out.key(false, depth + 1, "$guid")?;
            write!(out.out, "\"{}\"", tree.guid())?;
            out.key(false, depth + 1, "$path")?;
            write_string(out.out, tree.record_path())?;
            out.key(false, depth + 1, "$name")?;
            write_string(out.out, tree.name())?;
            if let Some(tag) = tree.tag() {
                out.key(false, depth + 1, "$tag")?;
                write_string(out.out, tag)?;
            }
        }
        let base = self.attribute.len();
        classify(fields, &mut self.attribute, &mut self.order);
        let escaped_base = self.escaped.len();
        self.mark_escaped(fields);
        for attributes in [true, false] {
            for (index, field) in fields.iter().enumerate() {
                if self.attribute.get(base + index) != Some(&attributes) {
                    continue;
                }
                if self.escaped.get(escaped_base + index) == Some(&true) {
                    out.separator(false, depth + 1)?;
                    write!(out.out, "\"${index}\":")?;
                    out.space()?;
                    out.out.write_all(b"{")?;
                    out.key(true, depth + 2, "name")?;
                    write_string(out.out, field.name())?;
                    out.key(false, depth + 2, "value")?;
                    self.write_value(out, field.value(), depth + 2)?;
                    out.close(depth + 1, b"}")?;
                } else {
                    out.key(false, depth + 1, field.name())?;
                    self.write_value(out, field.value(), depth + 1)?;
                }
            }
        }
        self.attribute.truncate(base);
        self.escaped.truncate(escaped_base);
        out.close(depth, b"}")
    }

    /// Pushes for every field whether its name cannot be a key: it starts with `$` or an earlier
    /// field has it.
    fn mark_escaped(&mut self, fields: &[Field<'_>]) {
        let base = self.escaped.len();
        self.escaped
            .extend(fields.iter().map(|field| field.name().starts_with('$')));
        self.order.clear();
        self.order.extend(0..fields.len());
        let name = |index: &usize| fields.get(*index).map(Field::name);
        self.order
            .sort_unstable_by(|left, right| name(left).cmp(&name(right)).then(left.cmp(right)));
        for pair in self.order.windows(2) {
            if let [first, second] = pair
                && name(first) == name(second)
                && let Some(flag) = self.escaped.get_mut(base + *second)
            {
                *flag = true;
            }
        }
    }

    fn write_value<W: Write>(
        &mut self,
        out: &mut Out<'_, W>,
        value: &Value<'_>,
        depth: usize,
    ) -> io::Result<()> {
        match value {
            Value::Struct(structure) => {
                self.write_struct(out, None, structure.type_name(), structure.fields(), depth)
            }
            Value::Array(items) => {
                out.out.write_all(b"[")?;
                let mut first = true;
                self.write_items(out, items, depth + 1, &mut first)?;
                if first {
                    return out.out.write_all(b"]");
                }
                out.close(depth, b"]")
            }
            Value::Reference(reference) => write_reference(out, reference, depth),
            Value::Null => out.out.write_all(b"null"),
            Value::Error(error) => {
                out.out.write_all(b"{")?;
                out.key(true, depth + 1, "error")?;
                write_string(out.out, &error.to_string())?;
                if let Some(guid) = error.dangling_guid() {
                    out.key(false, depth + 1, "guid")?;
                    write!(out.out, "\"{guid}\"")?;
                }
                out.close(depth, b"}")
            }
            Value::Locale(key) => write_locale(out, key, depth),
            scalar => write_json_value(out.out, scalar),
        }
    }

    fn write_items<W: Write>(
        &mut self,
        out: &mut Out<'_, W>,
        items: &[Value<'_>],
        depth: usize,
        first: &mut bool,
    ) -> io::Result<()> {
        for item in items {
            if let Value::Array(inner) = item {
                self.write_items(out, inner, depth, first)?;
                continue;
            }
            out.separator(*first, depth)?;
            *first = false;
            self.write_value(out, item, depth)?;
        }
        Ok(())
    }
}

impl<W: Write> Out<'_, W> {
    fn separator(&mut self, first: bool, depth: usize) -> io::Result<()> {
        if !first {
            self.out.write_all(b",")?;
        }
        if self.pretty {
            new_line(self.out, depth)?;
        }
        Ok(())
    }

    fn space(&mut self) -> io::Result<()> {
        if self.pretty {
            self.out.write_all(b" ")?;
        }
        Ok(())
    }

    fn key(&mut self, first: bool, depth: usize, key: &str) -> io::Result<()> {
        self.separator(first, depth)?;
        write_string(self.out, key)?;
        self.out.write_all(b":")?;
        self.space()
    }

    fn close(&mut self, depth: usize, bracket: &[u8]) -> io::Result<()> {
        if self.pretty {
            new_line(self.out, depth)?;
        }
        self.out.write_all(bracket)
    }
}

fn write_locale<W: Write>(out: &mut Out<'_, W>, key: &str, depth: usize) -> io::Result<()> {
    let Some(resolution) = annotation(&out.annotations, key) else {
        return write_string(out.out, key);
    };
    out.out.write_all(b"{")?;
    out.key(true, depth + 1, "key")?;
    write_string(out.out, key)?;
    if let Resolution::Text(text) = resolution {
        out.key(false, depth + 1, "text")?;
        write_string(out.out, &text)?;
    } else {
        out.key(false, depth + 1, "missing")?;
        out.out.write_all(b"true")?;
    }
    out.close(depth, b"}")
}

fn write_reference<W: Write>(
    out: &mut Out<'_, W>,
    reference: &Reference<'_>,
    depth: usize,
) -> io::Result<()> {
    out.out.write_all(b"{")?;
    out.key(true, depth + 1, "guid")?;
    write!(out.out, "\"{}\"", reference.guid())?;
    out.key(false, depth + 1, "path")?;
    write_string(out.out, reference.record_path())?;
    out.key(false, depth + 1, "type")?;
    write_string(out.out, reference.type_name())?;
    if let Some(location) = reference.location() {
        out.key(false, depth + 1, "at")?;
        write_string(out.out, location.as_str())?;
    }
    if let Some(path) = out.annotations.tag_path(reference) {
        out.key(false, depth + 1, "tag")?;
        write_string(out.out, path)?;
    }
    out.close(depth, b"}")
}
