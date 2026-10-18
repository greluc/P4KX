use crate::TestValue;

/// The type of a field in a synthetic DataForge struct.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestType {
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
    /// A string from the text table.
    String,
    /// A 32-bit float.
    Single,
    /// A 64-bit float.
    Double,
    /// A localisation key.
    Locale,
    /// A GUID.
    Guid,
    /// A value of the named enum.
    Enum(String),
    /// An inline instance of the named struct.
    Class(String),
    /// An owned instance of the named struct.
    StrongPointer(String),
    /// A pointer to an instance of the named struct elsewhere.
    WeakPointer(String),
    /// A Reference to a Record.
    Reference,
    /// A Reference to a Record of the named struct or of a struct deriving from it.
    ReferenceTo(String),
}

impl TestType {
    pub(crate) const fn code(&self) -> u16 {
        match self {
            Self::Bool => 0x0001,
            Self::Int8 => 0x0002,
            Self::Int16 => 0x0003,
            Self::Int32 => 0x0004,
            Self::Int64 => 0x0005,
            Self::UInt8 => 0x0006,
            Self::UInt16 => 0x0007,
            Self::UInt32 => 0x0008,
            Self::UInt64 => 0x0009,
            Self::String => 0x000A,
            Self::Single => 0x000B,
            Self::Double => 0x000C,
            Self::Locale => 0x000D,
            Self::Guid => 0x000E,
            Self::Enum(_) => 0x000F,
            Self::Class(_) => 0x0010,
            Self::StrongPointer(_) => 0x0110,
            Self::WeakPointer(_) => 0x0210,
            Self::Reference | Self::ReferenceTo(_) => 0x0310,
        }
    }
}

/// One field of a synthetic DataForge struct.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TestProperty {
    pub(crate) name: String,
    pub(crate) ty: TestType,
    pub(crate) array: bool,
}

/// A struct definition of a synthetic DataForge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestStruct {
    pub(crate) name: String,
    pub(crate) parent: Option<String>,
    pub(crate) properties: Vec<TestProperty>,
    pub(crate) declared_size: Option<u32>,
}

impl TestStruct {
    /// Creates a struct without parent and fields.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            parent: None,
            properties: Vec::new(),
            declared_size: None,
        }
    }

    /// Writes `size` as the instance size instead of the size of the fields.
    #[must_use]
    pub const fn declared_size(mut self, size: u32) -> Self {
        self.declared_size = Some(size);
        self
    }

    /// Makes the struct inherit the fields of the named struct.
    #[must_use]
    pub fn parent(mut self, parent: impl Into<String>) -> Self {
        self.parent = Some(parent.into());
        self
    }

    /// Adds a field holding one value.
    #[must_use]
    pub fn field(mut self, name: impl Into<String>, ty: TestType) -> Self {
        self.properties.push(TestProperty {
            name: name.into(),
            ty,
            array: false,
        });
        self
    }

    /// Adds a field holding an array of values.
    #[must_use]
    pub fn array(mut self, name: impl Into<String>, ty: TestType) -> Self {
        self.properties.push(TestProperty {
            name: name.into(),
            ty,
            array: true,
        });
        self
    }
}

/// A Record of a synthetic DataForge.
#[derive(Debug, Clone, PartialEq)]
pub struct TestRecord {
    pub(crate) guid: u128,
    pub(crate) type_name: String,
    pub(crate) name: String,
    pub(crate) file_name: String,
    pub(crate) tag: String,
    pub(crate) values: Vec<(String, TestValue)>,
}

impl TestRecord {
    /// Creates a Record of the struct `type_name`.
    ///
    /// `guid` is the GUID as it is written in text, read as one hexadecimal number, e.g.
    /// `0xcbee34fd_6441_4000_b99f_924f7f26e19f`; `file_name` is stored as given, e.g.
    /// `libs/foundry/records/ships/avenger.xml`.
    #[must_use]
    pub fn new(
        guid: u128,
        type_name: impl Into<String>,
        name: impl Into<String>,
        file_name: impl Into<String>,
    ) -> Self {
        Self {
            guid,
            type_name: type_name.into(),
            name: name.into(),
            file_name: file_name.into(),
            tag: String::new(),
            values: Vec::new(),
        }
    }

    /// Sets the tag a version 8 database stores with the Record, e.g. `SystemsDesign`; it is
    /// empty by default and not written in the version 6 layout.
    #[must_use]
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tag = tag.into();
        self
    }

    /// Sets the field `name` of the Record's instance to `value`; fields not set stay zero.
    #[must_use]
    pub fn value(mut self, name: impl Into<String>, value: TestValue) -> Self {
        self.values.push((name.into(), value));
        self
    }
}

/// A value pool of a synthetic DataForge, in the order the pools are stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestPool {
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
    /// Strings.
    String,
    /// Localisation keys.
    Locale,
    /// Enum values.
    Enum,
    /// Strong pointers.
    StrongPointer,
    /// Weak pointers.
    WeakPointer,
    /// References.
    Reference,
    /// Enum option names.
    EnumOption,
}

impl TestPool {
    pub(crate) const COUNT: usize = 19;

    pub(crate) const STORED: [Self; Self::COUNT] = [
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

    /// Returns the pool that holds array values of `ty`, or `None` for class arrays.
    pub(crate) const fn of(ty: &TestType) -> Option<Self> {
        Some(match ty {
            TestType::Bool => Self::Bool,
            TestType::Int8 => Self::Int8,
            TestType::Int16 => Self::Int16,
            TestType::Int32 => Self::Int32,
            TestType::Int64 => Self::Int64,
            TestType::UInt8 => Self::UInt8,
            TestType::UInt16 => Self::UInt16,
            TestType::UInt32 => Self::UInt32,
            TestType::UInt64 => Self::UInt64,
            TestType::String => Self::String,
            TestType::Single => Self::Single,
            TestType::Double => Self::Double,
            TestType::Locale => Self::Locale,
            TestType::Guid => Self::Guid,
            TestType::Enum(_) => Self::Enum,
            TestType::StrongPointer(_) => Self::StrongPointer,
            TestType::WeakPointer(_) => Self::WeakPointer,
            TestType::Reference | TestType::ReferenceTo(_) => Self::Reference,
            TestType::Class(_) => return None,
        })
    }

    pub(crate) const fn stored_index(self) -> usize {
        self as usize
    }

    pub(crate) const fn element_len(self) -> usize {
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
}
