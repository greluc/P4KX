use crate::TestValue;
use crate::dataforge_builder::{Writer, guid_bytes};

impl Writer<'_> {
    /// Returns the instance data: one block per struct in struct order, one instance per Record
    /// in Record order, zero-filled except for the values the Records set.
    pub(crate) fn instances(&mut self, instance_counts: &[u32]) -> Vec<u8> {
        let mut starts = Vec::with_capacity(instance_counts.len());
        let mut total = 0_usize;
        for (count, size) in instance_counts.iter().zip(&self.sizes) {
            starts.push(total);
            total += *count as usize * *size as usize;
        }
        let mut out = vec![0; total];
        let mut next = vec![0_usize; instance_counts.len()];
        let spec = self.spec;
        for record in &spec.records {
            let Some(&index) = self.struct_index.get(record.type_name.as_str()) else {
                continue;
            };
            let index = index as usize;
            let (Some(start), Some(size), Some(slot)) = (
                starts.get(index).copied(),
                self.sizes.get(index).copied(),
                next.get_mut(index),
            ) else {
                continue;
            };
            let base = start + *slot * size as usize;
            *slot += 1;
            for (name, value) in &record.values {
                let Some((offset, len)) = self.field_slot(index, name) else {
                    continue;
                };
                let bytes = self.encode(value);
                let at = base + offset as usize;
                let len = (len as usize).min(bytes.len());
                if let (Some(target), Some(source)) = (out.get_mut(at..at + len), bytes.get(..len))
                {
                    target.copy_from_slice(source);
                }
            }
        }
        out
    }

    /// Returns the offset and size of the first field called `name` of a struct, inherited
    /// fields first.
    fn field_slot(&self, index: usize, name: &str) -> Option<(u32, u32)> {
        let depth = self.spec.structs.len();
        let mut chain = Vec::new();
        let mut current = Some(index);
        while let Some(at) = current {
            if chain.len() > depth {
                return None;
            }
            chain.push(at);
            current = self
                .spec
                .structs
                .get(at)?
                .parent
                .as_deref()
                .and_then(|parent| self.struct_index.get(parent))
                .map(|parent| *parent as usize);
        }
        let mut offset = 0_u32;
        for at in chain.into_iter().rev() {
            for property in &self.spec.structs.get(at)?.properties {
                let size = self.property_size(property, depth);
                if property.name == name {
                    return Some((offset, size));
                }
                offset = offset.saturating_add(size);
            }
        }
        None
    }

    fn encode(&mut self, value: &TestValue) -> Vec<u8> {
        match value {
            TestValue::Bool(value) => vec![u8::from(*value)],
            TestValue::Int8(value) => value.to_le_bytes().to_vec(),
            TestValue::Int16(value) => value.to_le_bytes().to_vec(),
            TestValue::Int32(value) => value.to_le_bytes().to_vec(),
            TestValue::Int64(value) => value.to_le_bytes().to_vec(),
            TestValue::UInt8(value) => vec![*value],
            TestValue::UInt16(value) => value.to_le_bytes().to_vec(),
            TestValue::UInt32(value) => value.to_le_bytes().to_vec(),
            TestValue::UInt64(value) => value.to_le_bytes().to_vec(),
            TestValue::Single(value) => value.to_le_bytes().to_vec(),
            TestValue::Double(value) => value.to_le_bytes().to_vec(),
            TestValue::Text(text) => self.text.offset(text).to_le_bytes().to_vec(),
            TestValue::Guid(guid) => guid_bytes(*guid).to_vec(),
            TestValue::Raw(bytes) => bytes.clone(),
        }
    }
}
