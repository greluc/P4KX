use std::collections::HashMap;

use crate::bytes::{put_u16, put_u32};
use crate::dataforge_schema::{TestPool, TestProperty, TestRecord, TestStruct, TestType};

const NONE: u32 = u32::MAX;
const ARRAY_LEN: u32 = 8;
const CONVERSION_ATTRIBUTE: u16 = 0;
const CONVERSION_COMPLEX_ARRAY: u16 = 1;
const CONVERSION_SIMPLE_ARRAY: u16 = 2;

/// Builds a synthetic DataForge database in the byte layout of version 6 from a declarative
/// description of its structs, enums and Records.
///
/// Every Record gets one zero-filled instance of its struct; struct and enum names refer to
/// declarations by name, and an unknown name is written as index `u32::MAX`.
#[derive(Debug, Clone)]
pub struct DataForgeBuilder {
    version: u32,
    structs: Vec<TestStruct>,
    enums: Vec<(String, Vec<String>)>,
    records: Vec<TestRecord>,
    extra_values: [u32; TestPool::COUNT],
}

impl Default for DataForgeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl DataForgeBuilder {
    /// Creates a builder for an empty version 6 database.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            version: 6,
            structs: Vec::new(),
            enums: Vec::new(),
            records: Vec::new(),
            extra_values: [0; TestPool::COUNT],
        }
    }

    /// Writes `version` into the header, keeping the version 6 layout.
    #[must_use]
    pub const fn version(mut self, version: u32) -> Self {
        self.version = version;
        self
    }

    /// Adds a struct definition.
    #[must_use]
    pub fn structure(mut self, structure: TestStruct) -> Self {
        self.structs.push(structure);
        self
    }

    /// Adds an enum with its options in order.
    #[must_use]
    pub fn enumeration<S: Into<String>>(
        mut self,
        name: impl Into<String>,
        options: impl IntoIterator<Item = S>,
    ) -> Self {
        self.enums
            .push((name.into(), options.into_iter().map(Into::into).collect()));
        self
    }

    /// Adds a Record.
    #[must_use]
    pub fn record(mut self, record: TestRecord) -> Self {
        self.records.push(record);
        self
    }

    /// Appends `count` zero-filled values to a value pool.
    #[must_use]
    pub fn values(mut self, pool: TestPool, count: u32) -> Self {
        if let Some(slot) = TestPool::STORED
            .iter()
            .position(|stored| *stored == pool)
            .and_then(|index| self.extra_values.get_mut(index))
        {
            *slot = slot.saturating_add(count);
        }
        self
    }

    /// Returns the bytes of the database.
    #[must_use]
    pub fn build(&self) -> Vec<u8> {
        Writer::new(self).finish()
    }
}

#[derive(Default)]
struct Strings {
    bytes: Vec<u8>,
    offsets: HashMap<String, u32>,
}

impl Strings {
    fn offset(&mut self, text: &str) -> u32 {
        if let Some(offset) = self.offsets.get(text) {
            return *offset;
        }
        let offset = to_u32(self.bytes.len());
        self.bytes.extend_from_slice(text.as_bytes());
        self.bytes.push(0);
        self.offsets.insert(text.to_owned(), offset);
        offset
    }
}

struct Writer<'a> {
    spec: &'a DataForgeBuilder,
    struct_index: HashMap<&'a str, u32>,
    enum_index: HashMap<&'a str, u32>,
    sizes: Vec<u32>,
    text: Strings,
    blob: Strings,
}

impl<'a> Writer<'a> {
    fn new(spec: &'a DataForgeBuilder) -> Self {
        let struct_index = names(spec.structs.iter().map(|s| s.name.as_str()));
        let enum_index = names(spec.enums.iter().map(|(name, _)| name.as_str()));
        let mut writer = Self {
            spec,
            struct_index,
            enum_index,
            sizes: Vec::new(),
            text: Strings::default(),
            blob: Strings::default(),
        };
        writer.sizes = (0..spec.structs.len())
            .map(|index| writer.struct_size(index, spec.structs.len()))
            .collect();
        writer
    }

    fn struct_size(&self, index: usize, depth: usize) -> u32 {
        let Some(structure) = self.spec.structs.get(index) else {
            return 0;
        };
        let Some(depth) = depth.checked_sub(1) else {
            return 0;
        };
        let inherited = structure
            .parent
            .as_deref()
            .and_then(|parent| self.struct_index.get(parent))
            .map_or(0, |parent| self.struct_size(*parent as usize, depth));
        structure
            .properties
            .iter()
            .fold(inherited, |size, property| {
                size.saturating_add(self.property_size(property, depth))
            })
    }

    fn property_size(&self, property: &TestProperty, depth: usize) -> u32 {
        if property.array {
            return ARRAY_LEN;
        }
        match &property.ty {
            TestType::Bool | TestType::Int8 | TestType::UInt8 => 1,
            TestType::Int16 | TestType::UInt16 => 2,
            TestType::Int32
            | TestType::UInt32
            | TestType::String
            | TestType::Single
            | TestType::Locale
            | TestType::Enum(_) => 4,
            TestType::Int64
            | TestType::UInt64
            | TestType::Double
            | TestType::StrongPointer(_)
            | TestType::WeakPointer(_) => 8,
            TestType::Guid => 16,
            TestType::Reference => 20,
            TestType::Class(name) => self
                .struct_index
                .get(name.as_str())
                .map_or(0, |index| self.struct_size(*index as usize, depth)),
        }
    }

    fn property_target(&self, ty: &TestType) -> u16 {
        let target = match ty {
            TestType::Enum(name) => self.enum_index.get(name.as_str()),
            TestType::Class(name) | TestType::StrongPointer(name) | TestType::WeakPointer(name) => {
                self.struct_index.get(name.as_str())
            }
            _ => None,
        };
        target.map_or(0, |index| u16::try_from(*index).unwrap_or(u16::MAX))
    }

    fn finish(mut self) -> Vec<u8> {
        let spec = self.spec;
        let (structs, properties, property_count) = self.struct_tables();
        let (enums, options) = self.enum_tables();
        let (records, instance_counts) = self.records();
        let (mappings, instances) = self.mappings(&instance_counts);
        let option_count = to_u32(options.len() / 4);
        let mut out = Vec::new();
        for word in [0, spec.version, 0, 0] {
            put_u32(&mut out, word);
        }
        for count in [
            to_u32(spec.structs.len()),
            property_count,
            to_u32(spec.enums.len()),
            to_u32(spec.structs.len()),
            to_u32(spec.records.len()),
        ] {
            put_u32(&mut out, count);
        }
        for pool in TestPool::HEADER {
            let own = if pool == TestPool::EnumOption {
                option_count
            } else {
                0
            };
            put_u32(&mut out, own.saturating_add(self.extra(pool)));
        }
        put_u32(&mut out, to_u32(self.text.bytes.len()));
        put_u32(&mut out, to_u32(self.blob.bytes.len()));
        for table in [&structs, &properties, &enums, &mappings, &records] {
            out.extend_from_slice(table);
        }
        for pool in TestPool::STORED {
            if pool == TestPool::EnumOption {
                out.extend_from_slice(&options);
            }
            out.resize(
                out.len() + self.extra(pool) as usize * pool.element_len(),
                0,
            );
        }
        out.extend_from_slice(&self.text.bytes);
        out.extend_from_slice(&self.blob.bytes);
        out.resize(out.len() + instances, 0);
        out
    }

    fn extra(&self, pool: TestPool) -> u32 {
        TestPool::STORED
            .iter()
            .zip(self.spec.extra_values)
            .find_map(|(stored, extra)| (*stored == pool).then_some(extra))
            .unwrap_or(0)
    }

    fn struct_tables(&mut self) -> (Vec<u8>, Vec<u8>, u32) {
        let mut structs = Vec::new();
        let mut properties = Vec::new();
        let mut property_count = 0_u32;
        let sizes = self.sizes.clone();
        for (structure, size) in self.spec.structs.iter().zip(&sizes) {
            put_u32(&mut structs, self.blob.offset(&structure.name));
            put_u32(
                &mut structs,
                self.lookup_struct(structure.parent.as_deref()),
            );
            put_u16(&mut structs, to_u16(structure.properties.len()));
            put_u16(&mut structs, to_u16(property_count as usize));
            put_u32(&mut structs, *size);
            for property in &structure.properties {
                self.put_property(&mut properties, property);
                property_count = property_count.saturating_add(1);
            }
        }
        (structs, properties, property_count)
    }

    fn enum_tables(&mut self) -> (Vec<u8>, Vec<u8>) {
        let mut enums = Vec::new();
        let mut options = Vec::new();
        for (name, values) in &self.spec.enums {
            put_u32(&mut enums, self.blob.offset(name));
            put_u16(&mut enums, to_u16(values.len()));
            put_u16(&mut enums, to_u16(options.len() / 4));
            for value in values {
                put_u32(&mut options, self.blob.offset(value));
            }
        }
        (enums, options)
    }

    fn mappings(&self, instance_counts: &[u32]) -> (Vec<u8>, usize) {
        let mut mappings = Vec::new();
        let mut instances = 0_usize;
        for (index, (count, size)) in instance_counts.iter().zip(&self.sizes).enumerate() {
            put_u32(&mut mappings, *count);
            put_u32(&mut mappings, to_u32(index));
            instances += *count as usize * *size as usize;
        }
        (mappings, instances)
    }
    fn records(&mut self) -> (Vec<u8>, Vec<u32>) {
        let mut instance_counts = vec![0_u32; self.spec.structs.len()];
        let mut rows = Vec::new();
        for record in &self.spec.records {
            let struct_index = self.lookup_struct(Some(&record.type_name));
            let slot = instance_counts.get_mut(struct_index as usize);
            let instance = slot.as_ref().map_or(0, |count| **count);
            if let Some(count) = slot {
                *count = count.saturating_add(1);
            }
            let size = self.sizes.get(struct_index as usize).copied().unwrap_or(0);
            put_u32(&mut rows, self.blob.offset(&record.name));
            put_u32(&mut rows, self.text.offset(&record.file_name));
            put_u32(&mut rows, struct_index);
            rows.extend_from_slice(&guid_bytes(record.guid));
            put_u16(&mut rows, to_u16(instance as usize));
            put_u16(&mut rows, to_u16(size as usize));
        }
        (rows, instance_counts)
    }

    fn put_property(&mut self, out: &mut Vec<u8>, property: &TestProperty) {
        let conversion = match (&property.array, &property.ty) {
            (false, _) => CONVERSION_ATTRIBUTE,
            (
                true,
                TestType::Class(_)
                | TestType::StrongPointer(_)
                | TestType::WeakPointer(_)
                | TestType::Reference,
            ) => CONVERSION_COMPLEX_ARRAY,
            (true, _) => CONVERSION_SIMPLE_ARRAY,
        };
        put_u32(out, self.blob.offset(&property.name));
        put_u16(out, self.property_target(&property.ty));
        put_u16(out, property.ty.code());
        put_u16(out, conversion);
        put_u16(out, 0);
    }

    fn lookup_struct(&self, name: Option<&str>) -> u32 {
        name.and_then(|name| self.struct_index.get(name))
            .copied()
            .unwrap_or(NONE)
    }
}

/// Returns the 16 stored bytes of a GUID written in text as the hexadecimal number `guid`.
fn guid_bytes(guid: u128) -> [u8; 16] {
    let text = guid.to_be_bytes();
    let mut stored = [0; 16];
    for (position, byte) in text.iter().enumerate() {
        let target = if position < 8 {
            7 - position
        } else {
            23 - position
        };
        if let Some(slot) = stored.get_mut(target) {
            *slot = *byte;
        }
    }
    stored
}

fn names<'a>(names: impl Iterator<Item = &'a str>) -> HashMap<&'a str, u32> {
    names
        .enumerate()
        .map(|(index, name)| (name, to_u32(index)))
        .collect()
}

fn to_u32(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

fn to_u16(value: usize) -> u16 {
    u16::try_from(value).unwrap_or(u16::MAX)
}
