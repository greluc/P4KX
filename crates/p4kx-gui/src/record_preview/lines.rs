use std::fmt::Write as _;

use p4kx_extraction::{DataForge, Field, RecordTree, RecordXml, Reference, Value, ValueError};

use super::{FieldLine, LineValue, RecordLink, RecordView};
use crate::model::describe;
use crate::{RecordId, XmlDocument};

/// Builds the preview of the Record `tree` of `forge`, resolving each Reference to the Record it
/// selects.
pub(super) fn view(forge: &DataForge<'_>, tree: &RecordTree<'_>) -> RecordView {
    let mut lines = Lines {
        forge,
        out: Vec::new(),
        text: String::new(),
    };
    lines.fields(tree.fields(), 0);
    let mut xml = Vec::new();
    let xml_error = RecordXml::new()
        .write(tree, &mut xml)
        .err()
        .map(|error| describe(&error).into_boxed_str());
    if xml_error.is_some() {
        xml.clear();
    }
    lines.out.shrink_to_fit();
    RecordView {
        name: tree.name().into(),
        type_name: tree.type_name().into(),
        guid: tree.guid(),
        record_path: tree.record_path().into(),
        lines: lines.out,
        errors: tree.error_count(),
        xml: XmlDocument::new(xml, false),
        xml_error,
    }
}

struct Lines<'a, 'f> {
    forge: &'a DataForge<'f>,
    out: Vec<FieldLine>,
    text: String,
}

impl Lines<'_, '_> {
    fn fields(&mut self, fields: &[Field<'_>], depth: u16) {
        for field in fields {
            self.value(field.name().into(), field.value(), depth);
        }
    }

    fn value(&mut self, name: Box<str>, value: &Value<'_>, depth: u16) {
        let deeper = depth.saturating_add(1);
        let shown = match value {
            Value::Struct(structure) => LineValue::Struct(structure.type_name().into()),
            Value::Array(items) => LineValue::Array(items.len()),
            Value::Null => LineValue::Null,
            Value::Reference(reference) => LineValue::Reference(self.link(reference)),
            Value::Error(ValueError::DanglingReference { guid }) => LineValue::Dangling {
                guid: *guid,
                reason: self.shown(value_error(value)),
            },
            Value::Error(_) => LineValue::Error(self.shown(value_error(value))),
            scalar => {
                self.text.clear();
                let _infallible = write!(self.text, "{scalar}");
                LineValue::Scalar(self.text.as_str().into())
            }
        };
        self.out.push(FieldLine {
            depth,
            name,
            value: shown,
        });
        match value {
            Value::Struct(structure) => self.fields(structure.fields(), deeper),
            Value::Array(items) => {
                for (index, item) in items.iter().enumerate() {
                    self.text.clear();
                    let _infallible = write!(self.text, "[{index}]");
                    let name = self.text.as_str().into();
                    self.value(name, item, deeper);
                }
            }
            _ => {}
        }
    }

    fn shown(&mut self, error: Option<&ValueError>) -> Box<str> {
        self.text.clear();
        if let Some(error) = error {
            let _infallible = write!(self.text, "{error}");
        }
        self.text.as_str().into()
    }

    fn link(&self, reference: &Reference<'_>) -> RecordLink {
        RecordLink {
            guid: reference.guid(),
            record_path: reference.record_path().into(),
            type_name: reference.type_name().into(),
            at: reference.location().map(|at| at.as_str().into()),
            target: self
                .forge
                .record_by_guid(reference.guid())
                .map(|record| RecordId(record.index())),
        }
    }
}

const fn value_error<'v>(value: &'v Value<'_>) -> Option<&'v ValueError> {
    match value {
        Value::Error(error) => Some(error),
        _ => None,
    }
}
