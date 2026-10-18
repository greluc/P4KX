/// A text of the Record list of the Comparison tab and of its field changes, looked up in the
/// Fluent file of the current language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RecordChangeText {
    /// The progress of the Record level; takes `done`, `total` and `percent`.
    Progress,
    /// The moved Records.
    Moved,
    /// The Records that cannot be read.
    Unreadable,
    /// The record paths of a moved Record; takes `old` and `new`.
    MovedPath,
    /// Shown in the list when no Record differs.
    NoDifferences,
    /// The field column of the field changes.
    HeadField,
    /// The number of field changes; takes `count` and `number`.
    FieldCount,
    /// Shown while the fields are compared; takes `path`.
    Loading,
    /// Shown when comparing the fields failed; takes `path` and `reason`.
    Failed,
    /// Shown when comparing the fields stopped without a result; takes `path`.
    Stopped,
    /// Shown for an added Record.
    OnlyNew,
    /// Shown for a removed Record.
    OnlyOld,
    /// Shown for a moved Record whose fields are equal.
    OnlyPath,
    /// Shown for a Record without a field change.
    NoFieldChanges,
    /// Why the old Record cannot be read; takes `reason`.
    UnreadableOld,
    /// Why the new Record cannot be read; takes `reason`.
    UnreadableNew,
    /// The row of the Record name.
    RecordName,
    /// The row of the Record type.
    RecordType,
    /// A value the side does not have.
    ValueMissing,
    /// A null pointer or Reference.
    ValueNull,
    /// A struct; takes `type`.
    ValueStruct,
    /// An array; takes `count` and `number`.
    ValueArray,
    /// A Reference to a place inside a Record; takes `path` and `at`.
    ValueReferenceAt,
    /// A value that cannot be read; takes `reason`.
    ValueError,
    /// The accessible name of a field change; takes `field`, `old` and `new`.
    FieldRow,
}

impl RecordChangeText {
    /// Every text of the Record list and its field changes.
    pub const ALL: [Self; 25] = [
        Self::Progress,
        Self::Moved,
        Self::Unreadable,
        Self::MovedPath,
        Self::NoDifferences,
        Self::HeadField,
        Self::FieldCount,
        Self::Loading,
        Self::Failed,
        Self::Stopped,
        Self::OnlyNew,
        Self::OnlyOld,
        Self::OnlyPath,
        Self::NoFieldChanges,
        Self::UnreadableOld,
        Self::UnreadableNew,
        Self::RecordName,
        Self::RecordType,
        Self::ValueMissing,
        Self::ValueNull,
        Self::ValueStruct,
        Self::ValueArray,
        Self::ValueReferenceAt,
        Self::ValueError,
        Self::FieldRow,
    ];

    /// Returns the Fluent message id.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Progress => "record-change-progress",
            Self::Moved => "record-change-moved",
            Self::Unreadable => "record-change-unreadable",
            Self::MovedPath => "record-change-moved-path",
            Self::NoDifferences => "record-change-no-differences",
            Self::HeadField => "record-change-head-field",
            Self::FieldCount => "record-change-field-count",
            Self::Loading => "record-change-loading",
            Self::Failed => "record-change-failed",
            Self::Stopped => "record-change-stopped",
            Self::OnlyNew => "record-change-only-new",
            Self::OnlyOld => "record-change-only-old",
            Self::OnlyPath => "record-change-only-path",
            Self::NoFieldChanges => "record-change-no-field-changes",
            Self::UnreadableOld => "record-change-unreadable-old",
            Self::UnreadableNew => "record-change-unreadable-new",
            Self::RecordName => "record-change-record-name",
            Self::RecordType => "record-change-record-type",
            Self::ValueMissing => "record-change-value-missing",
            Self::ValueNull => "record-change-value-null",
            Self::ValueStruct => "record-change-value-struct",
            Self::ValueArray => "record-change-value-array",
            Self::ValueReferenceAt => "record-change-value-reference-at",
            Self::ValueError => "record-change-value-error",
            Self::FieldRow => "record-change-field-row",
        }
    }
}
