/// A value pool of DataForge; the variants are in the order the pools are stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValueKind {
    /// Signed 8-bit integers.
    Int8,
    /// Signed 16-bit integers.
    Int16,
    /// Signed 32-bit integers.
    Int32,
    /// Signed 64-bit integers.
    Int64,
    /// Unsigned 8-bit integers.
    UInt8,
    /// Unsigned 16-bit integers.
    UInt16,
    /// Unsigned 32-bit integers.
    UInt32,
    /// Unsigned 64-bit integers.
    UInt64,
    /// Booleans.
    Bool,
    /// 32-bit floats.
    Single,
    /// 64-bit floats.
    Double,
    /// GUIDs.
    Guid,
    /// Strings, as offsets into the text table.
    String,
    /// Localisation keys, as offsets into the text table.
    Locale,
    /// Enum values, as offsets into the text table.
    Enum,
    /// Strong pointers to owned instances.
    StrongPointer,
    /// Weak pointers to instances.
    WeakPointer,
    /// References to Records.
    Reference,
    /// Enum option names, as offsets into the blob table.
    EnumOption,
}

impl ValueKind {
    pub(crate) const COUNT: usize = 19;

    /// Every pool in stored order.
    pub const ALL: [Self; Self::COUNT] = [
        Self::Int8,
        Self::Int16,
        Self::Int32,
        Self::Int64,
        Self::UInt8,
        Self::UInt16,
        Self::UInt32,
        Self::UInt64,
        Self::Bool,
        Self::Single,
        Self::Double,
        Self::Guid,
        Self::String,
        Self::Locale,
        Self::Enum,
        Self::StrongPointer,
        Self::WeakPointer,
        Self::Reference,
        Self::EnumOption,
    ];

    /// Every pool in the order the header counts them.
    pub(crate) const HEADER: [Self; Self::COUNT] = [
        Self::Bool,
        Self::Int8,
        Self::Int16,
        Self::Int32,
        Self::Int64,
        Self::UInt8,
        Self::UInt16,
        Self::UInt32,
        Self::UInt64,
        Self::Single,
        Self::Double,
        Self::Guid,
        Self::String,
        Self::Locale,
        Self::Enum,
        Self::StrongPointer,
        Self::WeakPointer,
        Self::Reference,
        Self::EnumOption,
    ];

    /// Returns the number of bytes of one value.
    #[must_use]
    pub const fn element_len(self) -> usize {
        match self {
            Self::Int8 | Self::UInt8 | Self::Bool => 1,
            Self::Int16 | Self::UInt16 => 2,
            Self::Int32
            | Self::UInt32
            | Self::Single
            | Self::String
            | Self::Locale
            | Self::Enum
            | Self::EnumOption => 4,
            Self::Int64 | Self::UInt64 | Self::Double | Self::StrongPointer | Self::WeakPointer => {
                8
            }
            Self::Guid => 16,
            Self::Reference => 20,
        }
    }

    pub(crate) const fn index(self) -> usize {
        self as usize
    }

    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::Int8 => "int8 pool",
            Self::Int16 => "int16 pool",
            Self::Int32 => "int32 pool",
            Self::Int64 => "int64 pool",
            Self::UInt8 => "uint8 pool",
            Self::UInt16 => "uint16 pool",
            Self::UInt32 => "uint32 pool",
            Self::UInt64 => "uint64 pool",
            Self::Bool => "bool pool",
            Self::Single => "float pool",
            Self::Double => "double pool",
            Self::Guid => "GUID pool",
            Self::String => "string pool",
            Self::Locale => "localisation key pool",
            Self::Enum => "enum value pool",
            Self::StrongPointer => "strong pointer pool",
            Self::WeakPointer => "weak pointer pool",
            Self::Reference => "Reference pool",
            Self::EnumOption => "enum option pool",
        }
    }
}

/// The data type of a struct property.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DataType {
    /// A boolean.
    Bool,
    /// A signed 8-bit integer.
    Int8,
    /// A signed 16-bit integer.
    Int16,
    /// A signed 32-bit integer.
    Int32,
    /// A signed 64-bit integer.
    Int64,
    /// An unsigned 8-bit integer.
    UInt8,
    /// An unsigned 16-bit integer.
    UInt16,
    /// An unsigned 32-bit integer.
    UInt32,
    /// An unsigned 64-bit integer.
    UInt64,
    /// A string.
    String,
    /// A 32-bit float.
    Single,
    /// A 64-bit float.
    Double,
    /// A localisation key.
    Locale,
    /// A GUID.
    Guid,
    /// A value of an enum.
    Enum,
    /// An inline instance of a struct.
    Class,
    /// An owned instance of a struct.
    StrongPointer,
    /// A pointer to an instance elsewhere.
    WeakPointer,
    /// A Reference to a Record.
    Reference,
}

impl DataType {
    /// Returns the data type stored as `code`, or `None` for an unknown code.
    #[must_use]
    pub const fn from_code(code: u16) -> Option<Self> {
        Some(match code {
            0x0001 => Self::Bool,
            0x0002 => Self::Int8,
            0x0003 => Self::Int16,
            0x0004 => Self::Int32,
            0x0005 => Self::Int64,
            0x0006 => Self::UInt8,
            0x0007 => Self::UInt16,
            0x0008 => Self::UInt32,
            0x0009 => Self::UInt64,
            0x000A => Self::String,
            0x000B => Self::Single,
            0x000C => Self::Double,
            0x000D => Self::Locale,
            0x000E => Self::Guid,
            0x000F => Self::Enum,
            0x0010 => Self::Class,
            0x0110 => Self::StrongPointer,
            0x0210 => Self::WeakPointer,
            0x0310 => Self::Reference,
            _ => return None,
        })
    }
}
