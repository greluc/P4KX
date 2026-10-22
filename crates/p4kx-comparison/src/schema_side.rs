use std::collections::{BTreeMap, BTreeSet, HashMap};

use p4kx_dataforge::{DataForge, DataForgeError, Property, Struct};

use crate::schema_diff::PropertyShape;

/// The definitions of one DataForge, keyed by name.
#[derive(Debug, Default)]
pub(crate) struct SideSchema<'f> {
    pub(crate) structs: BTreeMap<&'f str, StructDef<'f>>,
    pub(crate) enums: BTreeMap<&'f str, EnumDef<'f>>,
    pub(crate) unreadable: u64,
}

/// A struct with the properties it declares.
#[derive(Debug, Default)]
pub(crate) struct StructDef<'f> {
    pub(crate) parent: Option<&'f str>,
    pub(crate) properties: BTreeMap<&'f str, PropertyShape>,
    pub(crate) records: u64,
}

/// An enum with its option names and the Records that carry it.
#[derive(Debug, Default)]
pub(crate) struct EnumDef<'f> {
    pub(crate) options: BTreeSet<&'f str>,
    pub(crate) records: u64,
}

impl<'f> SideSchema<'f> {
    /// Reads the structs and enums of `forge`; a definition whose name cannot be read is counted
    /// and left out, and the first definition of a name wins.
    pub(crate) fn read(forge: &'f DataForge<'_>) -> Self {
        let mut side = Self::default();
        let mut own: Vec<(&'f str, u64)> = Vec::new();
        let mut uses: Vec<(&'f str, Vec<&'f str>)> = Vec::new();
        for structure in forge.structs() {
            let Ok(name) = structure.name() else {
                side.unreadable += 1;
                continue;
            };
            if side.structs.contains_key(name) {
                continue;
            }
            let (def, enums) = struct_def(&structure, &mut side.unreadable);
            own.push((name, u64::from(structure.instance_count())));
            uses.push((name, enums));
            side.structs.insert(name, def);
        }
        for enumeration in forge.enums() {
            let Ok(name) = enumeration.name() else {
                side.unreadable += 1;
                continue;
            };
            let def = side.enums.entry(name).or_default();
            for option in enumeration.options() {
                match option {
                    Ok(option) => {
                        def.options.insert(option);
                    }
                    Err(_) => side.unreadable += 1,
                }
            }
        }
        side.count_records(&own, &uses);
        side
    }

    fn count_records(&mut self, own: &[(&'f str, u64)], uses: &[(&'f str, Vec<&'f str>)]) {
        let mut effective: HashMap<&str, u64> = HashMap::with_capacity(own.len());
        for &(name, count) in own {
            let mut at = Some(name);
            let mut steps = 0;
            while let Some(current) = at
                && steps <= own.len()
            {
                *effective.entry(current).or_default() += count;
                at = self.structs.get(current).and_then(|def| def.parent);
                steps += 1;
            }
        }
        for (name, def) in &mut self.structs {
            def.records = effective.get(name).copied().unwrap_or_default();
        }
        for (name, enums) in uses {
            let records = effective.get(name).copied().unwrap_or_default();
            for enumeration in enums {
                if let Some(def) = self.enums.get_mut(enumeration) {
                    def.records += records;
                }
            }
        }
    }
}

fn struct_def<'f>(structure: &Struct<'f>, unreadable: &mut u64) -> (StructDef<'f>, Vec<&'f str>) {
    let mut def = StructDef {
        parent: structure.parent().and_then(|parent| parent.name().ok()),
        ..StructDef::default()
    };
    let mut enums: Vec<&'f str> = Vec::new();
    for property in structure.properties() {
        let Ok(name) = property.name() else {
            *unreadable += 1;
            continue;
        };
        let shape = shape(&property);
        if let Some(enumeration) = property.target_enum().and_then(|e| e.name().ok())
            && !enums.contains(&enumeration)
        {
            enums.push(enumeration);
        }
        def.properties.entry(name).or_insert(shape);
    }
    (def, enums)
}

fn shape(property: &Property<'_>) -> PropertyShape {
    let data_type = match property.data_type() {
        Ok(data_type) => data_type.to_string(),
        Err(DataForgeError::UnknownDataType { code }) => format!("Unknown({code:#06x})"),
        Err(_) => "Unknown".to_owned(),
    };
    let target = property
        .target_struct()
        .and_then(|structure| structure.name().ok())
        .or_else(|| property.target_enum().and_then(|e| e.name().ok()))
        .map(str::to_owned);
    PropertyShape {
        data_type,
        conversion: property.conversion(),
        target,
    }
}
