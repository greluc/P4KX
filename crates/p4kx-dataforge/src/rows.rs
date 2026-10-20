use crate::table::{Fields, Row};

/// The parent index of a struct without parent.
pub(crate) const NO_PARENT: u32 = u32::MAX;

/// The string offset that stands for no string, as a Record name or tag.
pub(crate) const NO_STRING: u32 = u32::MAX;

/// One row of the struct definition table.
#[derive(Debug, Clone, Copy)]
pub(crate) struct StructDef {
    pub(crate) name: u32,
    pub(crate) parent: u32,
    pub(crate) property_count: u16,
    pub(crate) first_property: u16,
    pub(crate) size: u32,
}

impl Row for StructDef {
    const SIZE: usize = 16;

    fn decode(fields: &mut Fields<'_>) -> Self {
        Self {
            name: fields.u32(),
            parent: fields.u32(),
            property_count: fields.u16(),
            first_property: fields.u16(),
            size: fields.u32(),
        }
    }
}

/// One row of the property definition table.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PropertyDef {
    pub(crate) name: u32,
    pub(crate) target: u16,
    pub(crate) data_type: u16,
    pub(crate) conversion: u16,
    pub(crate) flags: u8,
}

impl Row for PropertyDef {
    const SIZE: usize = 12;

    fn decode(fields: &mut Fields<'_>) -> Self {
        let name = fields.u32();
        let target = fields.u16();
        let data_type = fields.u16();
        let [conversion, flags] = fields.u16().to_le_bytes();
        Self {
            name,
            target,
            data_type,
            conversion: u16::from(conversion),
            flags,
        }
    }
}

/// One row of the enum definition table.
#[derive(Debug, Clone, Copy)]
pub(crate) struct EnumDef {
    pub(crate) name: u32,
    pub(crate) option_count: u16,
    pub(crate) first_option: u16,
}

impl Row for EnumDef {
    const SIZE: usize = 8;

    fn decode(fields: &mut Fields<'_>) -> Self {
        Self {
            name: fields.u32(),
            option_count: fields.u16(),
            first_option: fields.u16(),
        }
    }
}

/// One row of the data mapping table: where the instances of one struct lie.
#[derive(Debug, Clone, Copy)]
pub(crate) struct DataMapping {
    pub(crate) instance_count: u32,
    pub(crate) struct_index: u32,
}

impl Row for DataMapping {
    const SIZE: usize = 8;

    fn decode(fields: &mut Fields<'_>) -> Self {
        Self {
            instance_count: fields.u32(),
            struct_index: fields.u32(),
        }
    }
}

/// One row of the Record definition table, decoded in the layout of its version.
#[derive(Debug, Clone, Copy)]
pub(crate) struct RecordDef {
    pub(crate) name: u32,
    pub(crate) file_name: u32,
    pub(crate) tag: Option<u32>,
    pub(crate) struct_index: u32,
    pub(crate) guid: [u8; 16],
    pub(crate) instance_index: u16,
    pub(crate) trailing: u16,
}

/// One element of the enum option pool: the blob offset of an option name.
#[derive(Debug, Clone, Copy)]
pub(crate) struct OptionName(pub(crate) u32);

impl Row for OptionName {
    const SIZE: usize = 4;

    fn decode(fields: &mut Fields<'_>) -> Self {
        Self(fields.u32())
    }
}
