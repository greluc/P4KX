use std::fmt::Write as _;
use std::mem;
use std::sync::Arc;

use crate::build::Span;
use crate::{DataForge, Field, RecordTree, Reference, TreeBuilder, Value, ValueError};

impl<'f> TreeBuilder<'f> {
    /// Replaces the placeholder of every weak pointer of `tree` with its Reference.
    pub(crate) fn resolve_weak(&mut self, forge: &'f DataForge<'f>, tree: &mut RecordTree<'f>) {
        let positions = mem::take(&mut self.positions);
        let weak = mem::take(&mut self.weak);
        for pending in &weak {
            let value = self.weak_target(forge, tree, pending.target, &positions);
            if let Some(slot) = value_at_mut(&mut tree.fields, span(&positions, pending.at)) {
                *slot = value;
            }
        }
        self.positions = positions;
        self.weak = weak;
    }

    fn weak_target(
        &mut self,
        forge: &'f DataForge<'f>,
        tree: &RecordTree<'f>,
        target: (u32, u32),
        positions: &[u32],
    ) -> Value<'f> {
        let outside = Value::Error(ValueError::OutsideRecord {
            struct_index: target.0,
            instance_index: target.1,
        });
        let Some(owned) = self.owned.get(&target).copied() else {
            return forge
                .record_at(target.0, target.1)
                .and_then(|index| forge.record(index))
                .map_or(outside, |record| record.link());
        };
        let at = span(positions, owned);
        if at.is_empty() {
            return Value::Reference(Reference {
                guid: tree.guid,
                record_path: tree.record_path,
                type_name: tree.type_name,
                location: None,
            });
        }
        self.text.clear();
        let Some(type_name) = locate(&tree.fields, at, &mut self.text) else {
            return outside;
        };
        let text = &self.text;
        let location = self
            .locations
            .entry(target)
            .or_insert_with(|| Arc::from(text.as_str()))
            .clone();
        Value::Reference(Reference {
            guid: tree.guid,
            record_path: tree.record_path,
            type_name,
            location: Some(location),
        })
    }
}

fn span(positions: &[u32], span: Span) -> &[u32] {
    positions.get(span.start..span.end).unwrap_or_default()
}

/// Writes the location of the struct at the position `at` into `text` and returns its type.
fn locate<'f>(fields: &[Field<'f>], at: &[u32], text: &mut String) -> Option<&'f str> {
    let mut fields = fields;
    let mut steps = at.iter();
    let mut type_name = None;
    while let Some(&step) = steps.next() {
        let field = fields.get(step as usize)?;
        if !text.is_empty() {
            text.push('.');
        }
        text.push_str(field.name);
        let mut current = &field.value;
        if let Value::Array(items) = current {
            let index = *steps.next()?;
            write!(text, "[{index}]").ok()?;
            current = items.get(index as usize)?;
        }
        let Value::Struct(structure) = current else {
            return None;
        };
        fields = &structure.fields;
        type_name = Some(structure.type_name);
    }
    type_name
}

/// Returns the value at the position `at`: field and element indices from the Record down.
fn value_at_mut<'t, 'f>(fields: &'t mut [Field<'f>], at: &[u32]) -> Option<&'t mut Value<'f>> {
    let (first, rest) = at.split_first()?;
    let mut value = &mut fields.get_mut(*first as usize)?.value;
    for &step in rest {
        value = match value {
            Value::Struct(structure) => &mut structure.fields.get_mut(step as usize)?.value,
            Value::Array(items) => items.get_mut(step as usize)?,
            _ => return None,
        };
    }
    Some(value)
}
