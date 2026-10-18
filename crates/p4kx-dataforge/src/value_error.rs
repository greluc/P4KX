use thiserror::Error;

use crate::Guid;

/// Why one value of a Record cannot be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum ValueError {
    /// A string, localisation key or enum value is not a valid string offset.
    #[error("offset {offset} in the {table} table is not a valid string")]
    InvalidString {
        /// The table the offset points into.
        table: &'static str,
        /// The offset.
        offset: u32,
    },
    /// An enum field names an enum that does not exist.
    #[error("the field refers to enum {index}, which does not exist")]
    UnknownEnum {
        /// The enum index the property gives.
        index: u16,
    },
    /// A class array names a struct that does not exist.
    #[error("the field refers to struct {index}, which does not exist")]
    UnknownStruct {
        /// The struct index the property gives.
        index: u16,
    },
    /// A bool byte is neither 0 nor 1.
    #[error("the bool byte {byte} is neither 0 nor 1")]
    InvalidBool {
        /// The stored byte.
        byte: u8,
    },
    /// A strong or weak pointer names a struct or an instance that does not exist.
    #[error(
        "the pointer refers to instance {instance_index} of struct {struct_index}, which does not exist"
    )]
    InvalidPointer {
        /// The struct index of the pointer.
        struct_index: u32,
        /// The instance index of the pointer.
        instance_index: u32,
    },
    /// The elements of an array lie outside the values stored for its type.
    #[error(
        "the array of {count} values from index {first} lies outside the {pool} of {len} values"
    )]
    ArrayOutOfRange {
        /// The index of the first element.
        first: u32,
        /// The number of elements.
        count: u32,
        /// The number of values stored for the type.
        len: u64,
        /// Where the values are stored, e.g. `int32 pool` or `instance block`.
        pool: &'static str,
    },
    /// An instance is owned a second time within one Record, e.g. by a cycle of strong
    /// pointers.
    #[error(
        "instance {instance_index} of struct {struct_index} is owned more than once in the Record"
    )]
    OwnedTwice {
        /// The struct of the instance.
        struct_index: u32,
        /// The instance.
        instance_index: u32,
    },
    /// An array has more elements than one Record tree may hold.
    #[error("the array of {count} values is larger than the {limit} values a Record may hold")]
    TooManyValues {
        /// The number of elements.
        count: u32,
        /// The most values one Record tree may hold.
        limit: usize,
    },
    /// Owned structs are nested deeper than [`MAX_NESTING_DEPTH`](crate::MAX_NESTING_DEPTH).
    #[error("the value is nested deeper than {limit} structs")]
    TooDeep {
        /// The deepest nesting that is read.
        limit: usize,
    },
    /// A Reference names a Record that does not exist.
    #[error("the Reference refers to the Record {guid}, which does not exist")]
    DanglingReference {
        /// The GUID of the missing Record.
        guid: Guid,
    },
    /// A strong pointer refers to an instance of a struct that neither is nor derives from the
    /// struct the field declares.
    #[error(
        "the pointer refers to struct {struct_index}, which does not derive from struct {declared}, the type of the field"
    )]
    UnexpectedPointerType {
        /// The struct index of the pointer.
        struct_index: u32,
        /// The struct the field declares.
        declared: u32,
    },
    /// A Query path reaches more array elements in one Record than
    /// [`MAX_PATH_VALUES`](crate::query::MAX_PATH_VALUES).
    #[error("the path reaches more than {limit} array elements")]
    PathTooWide {
        /// The most elements a path may reach.
        limit: usize,
    },
    /// A weak pointer refers to an instance that is neither inside the Record nor a Record.
    #[error(
        "the weak pointer refers to instance {instance_index} of struct {struct_index}, which lies outside the Record"
    )]
    OutsideRecord {
        /// The struct index of the pointer.
        struct_index: u32,
        /// The instance index of the pointer.
        instance_index: u32,
    },
}
