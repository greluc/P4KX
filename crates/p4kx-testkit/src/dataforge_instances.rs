use std::collections::HashMap;

use crate::bytes::put_u32;
use crate::dataforge_builder::{Writer, guid_bytes};
use crate::dataforge_schema::{TestPool, TestProperty, TestType};
use crate::{TestInstance, TestValue};

const NULL: u32 = u32::MAX;

/// The instance data and the value pools of a synthetic DataForge.
pub(crate) struct Instances {
    pub(crate) blocks: Vec<Vec<u8>>,
    pub(crate) counts: Vec<u32>,
    pub(crate) pools: Vec<Vec<u8>>,
    pub(crate) records: Vec<u32>,
}

impl Instances {
    pub(crate) fn pool(&self, pool: TestPool) -> &[u8] {
        self.pools
            .get(pool.stored_index())
            .map_or(&[], Vec::as_slice)
    }

    fn pool_mut(&mut self, pool: TestPool) -> Option<&mut Vec<u8>> {
        self.pools.get_mut(pool.stored_index())
    }
}

/// One field of a struct as it lies in an instance.
struct Slot<'a> {
    property: &'a TestProperty,
    offset: usize,
    size: usize,
}

impl Writer<'_> {
    /// Returns the instances: per struct the instances of its Records in Record order, then the
    /// named instances, then those values create, zero-filled except for the values set and with
    /// null pointers where none is set.
    pub(crate) fn instances(&mut self, options: Vec<u8>) -> Instances {
        let structs = self.spec.structs.len();
        let mut instances = Instances {
            blocks: vec![Vec::new(); structs],
            counts: vec![0; structs],
            pools: vec![Vec::new(); TestPool::COUNT],
            records: Vec::new(),
        };
        if let Some(pool) = instances.pool_mut(TestPool::EnumOption) {
            *pool = options;
        }
        let spec = self.spec;
        let mut names = HashMap::new();
        let mut pending = Vec::new();
        let described = spec
            .records
            .iter()
            .map(|record| (&record.name, &record.type_name, &record.values, true))
            .chain(
                spec.instances
                    .iter()
                    .map(|(name, instance)| (name, &instance.type_name, &instance.values, false)),
            );
        for (name, type_name, values, is_record) in described {
            let Some(&index) = self.struct_index.get(type_name.as_str()) else {
                if is_record {
                    instances.records.push(0);
                }
                continue;
            };
            let instance = self.allocate(&mut instances, index, 1);
            if is_record {
                instances.records.push(instance);
            }
            names.entry(name.clone()).or_insert((index, instance));
            pending.push((index, instance, values));
        }
        self.names = names;
        for (index, instance, values) in pending {
            let bytes = self.encode_struct(&mut instances, index, values);
            store(&mut instances, index, instance, &bytes);
        }
        instances
    }

    /// Appends `count` instances of the struct at `index` holding its defaults and returns the
    /// index of the first.
    fn allocate(&self, instances: &mut Instances, index: u32, count: usize) -> u32 {
        let defaults = self.defaults(index, self.spec.structs.len());
        let first = instances.counts.get(index as usize).copied().unwrap_or(0);
        if let (Some(block), Some(slot)) = (
            instances.blocks.get_mut(index as usize),
            instances.counts.get_mut(index as usize),
        ) {
            for _ in 0..count {
                block.extend_from_slice(&defaults);
            }
            *slot = slot.saturating_add(u32::try_from(count).unwrap_or(u32::MAX));
        }
        first
    }

    /// Returns the bytes of an instance of the struct at `index` with no value set.
    fn defaults(&self, index: u32, depth: usize) -> Vec<u8> {
        let size = self.sizes.get(index as usize).copied().unwrap_or(0) as usize;
        let mut bytes = vec![0; size];
        let Some(depth) = depth.checked_sub(1) else {
            return bytes;
        };
        for slot in self.slots(index as usize) {
            let fill = match &slot.property.ty {
                _ if slot.property.array => continue,
                TestType::StrongPointer(_) | TestType::WeakPointer(_) => [NULL, NULL]
                    .iter()
                    .flat_map(|word| word.to_le_bytes())
                    .collect(),
                TestType::Class(name) => match self.struct_index.get(name.as_str()) {
                    Some(target) => self.defaults(*target, depth),
                    None => continue,
                },
                _ => continue,
            };
            copy(&mut bytes, slot.offset, &fill);
        }
        bytes
    }

    /// Returns the fields of the struct at `index`, inherited ones first, with their offsets.
    fn slots(&self, index: usize) -> Vec<Slot<'_>> {
        let depth = self.spec.structs.len();
        let mut chain = Vec::new();
        let mut current = Some(index);
        while let Some(at) = current {
            if chain.len() > depth {
                break;
            }
            chain.push(at);
            current = self
                .spec
                .structs
                .get(at)
                .and_then(|structure| structure.parent.as_deref())
                .and_then(|parent| self.struct_index.get(parent))
                .map(|parent| *parent as usize);
        }
        let mut slots = Vec::new();
        let mut offset = 0;
        for at in chain.into_iter().rev() {
            for property in self.spec.structs.get(at).map_or(&[][..], |s| &s.properties) {
                let size = self.property_size(property, depth) as usize;
                slots.push(Slot {
                    property,
                    offset,
                    size,
                });
                offset += size;
            }
        }
        slots
    }

    fn encode_struct(
        &mut self,
        instances: &mut Instances,
        index: u32,
        values: &[(String, TestValue)],
    ) -> Vec<u8> {
        let mut bytes = self.defaults(index, self.spec.structs.len());
        for (name, value) in values {
            let Some((offset, size, property)) = self
                .slots(index as usize)
                .into_iter()
                .find(|slot| slot.property.name == *name)
                .map(|slot| (slot.offset, slot.size, slot.property.clone()))
            else {
                continue;
            };
            let encoded = self.encode(instances, &property, value);
            let len = size.min(encoded.len());
            copy(&mut bytes, offset, encoded.get(..len).unwrap_or_default());
        }
        bytes
    }

    fn encode(
        &mut self,
        instances: &mut Instances,
        property: &TestProperty,
        value: &TestValue,
    ) -> Vec<u8> {
        match (value, &property.ty) {
            (TestValue::Array(items), ty) => self.encode_array(instances, ty, items),
            (TestValue::Struct(instance), TestType::Class(_)) => {
                let index = self.lookup(&instance.type_name);
                self.encode_struct(instances, index, &instance.values)
            }
            (TestValue::Struct(instance), _) => self.own(instances, instance),
            (TestValue::Pointer(name), _) => {
                let (index, instance) = self.names.get(name).copied().unwrap_or((NULL, NULL));
                pointer(index, instance)
            }
            (TestValue::Null, TestType::Reference) => vec![0; 20],
            (TestValue::Null, _) => pointer(NULL, NULL),
            (TestValue::Reference(guid), _) => {
                let mut bytes = Vec::new();
                put_u32(&mut bytes, 0);
                bytes.extend_from_slice(&guid_bytes(*guid));
                bytes
            }
            (scalar, _) => self.encode_scalar(scalar),
        }
    }

    fn own(&mut self, instances: &mut Instances, instance: &TestInstance) -> Vec<u8> {
        let index = self.lookup(&instance.type_name);
        let at = self.allocate(instances, index, 1);
        let bytes = self.encode_struct(instances, index, &instance.values);
        store(instances, index, at, &bytes);
        pointer(index, at)
    }

    fn encode_array(
        &mut self,
        instances: &mut Instances,
        ty: &TestType,
        items: &[TestValue],
    ) -> Vec<u8> {
        let count = u32::try_from(items.len()).unwrap_or(u32::MAX);
        let item = TestProperty {
            name: String::new(),
            ty: ty.clone(),
            array: false,
        };
        let first = if let TestType::Class(name) = ty {
            let index = self.lookup(name);
            let first = self.allocate(instances, index, items.len());
            for (at, value) in (first..).zip(items) {
                if let TestValue::Struct(instance) = value {
                    let bytes = self.encode_struct(instances, index, &instance.values);
                    store(instances, index, at, &bytes);
                }
            }
            first
        } else {
            let pool = TestPool::of(ty);
            let len = pool.map_or(1, TestPool::element_len);
            let encoded: Vec<Vec<u8>> = items
                .iter()
                .map(|value| self.encode(instances, &item, value))
                .collect();
            let target = pool.and_then(|pool| instances.pool_mut(pool));
            let Some(target) = target else {
                return pointer(count, 0);
            };
            let first = u32::try_from(target.len() / len).unwrap_or(u32::MAX);
            for mut bytes in encoded {
                bytes.resize(len, 0);
                target.extend_from_slice(&bytes);
            }
            first
        };
        pointer(count, first)
    }

    fn lookup(&self, name: &str) -> u32 {
        self.struct_index.get(name).copied().unwrap_or(NULL)
    }

    fn encode_scalar(&mut self, value: &TestValue) -> Vec<u8> {
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
            TestValue::Struct(_)
            | TestValue::Pointer(_)
            | TestValue::Null
            | TestValue::Reference(_)
            | TestValue::Array(_) => Vec::new(),
        }
    }
}

fn pointer(first: u32, second: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    put_u32(&mut bytes, first);
    put_u32(&mut bytes, second);
    bytes
}

fn store(instances: &mut Instances, index: u32, at: u32, bytes: &[u8]) {
    if let Some(block) = instances.blocks.get_mut(index as usize) {
        copy(block, at as usize * bytes.len(), bytes);
    }
}

fn copy(target: &mut [u8], at: usize, bytes: &[u8]) {
    if let Some(slot) = target.get_mut(at..at + bytes.len()) {
        slot.copy_from_slice(bytes);
    }
}
