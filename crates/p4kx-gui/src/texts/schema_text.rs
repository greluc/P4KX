// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

/// A text of the schema list of the Comparison tab, looked up in the Fluent file of the current
/// language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SchemaText {
    /// The tab that lists the schema drift.
    Tab,
    /// The name of the structs in the counts.
    LabelStructs,
    /// The name of the properties in the counts.
    LabelProperties,
    /// The name of the enums in the counts.
    LabelEnums,
    /// The name of the enum options in the counts.
    LabelOptions,
    /// The counts of one level; takes `label`, `added`, `removed` and `changed`.
    Counts,
    /// The column of what a row describes.
    ColumnKind,
    /// The column of the name.
    ColumnName,
    /// The column of the definition on the old side.
    ColumnOld,
    /// The column of the definition on the new side.
    ColumnNew,
    /// The column of the Records on the old side.
    ColumnOldRecords,
    /// The column of the Records on the new side.
    ColumnNewRecords,
    /// A struct row.
    KindStruct,
    /// A parent row.
    KindParent,
    /// A property row.
    KindProperty,
    /// An enum row.
    KindEnum,
    /// An enum option row.
    KindOption,
    /// Shown in the list when the schemas do not differ.
    NoDifferences,
    /// Shown when the schemas could not be compared; takes `reason`.
    Failed,
    /// The accessible name of a row; takes `change`, `kind` and `name`.
    Row,
}

impl SchemaText {
    /// Every text of the schema list.
    pub const ALL: [Self; 20] = [
        Self::Tab,
        Self::LabelStructs,
        Self::LabelProperties,
        Self::LabelEnums,
        Self::LabelOptions,
        Self::Counts,
        Self::ColumnKind,
        Self::ColumnName,
        Self::ColumnOld,
        Self::ColumnNew,
        Self::ColumnOldRecords,
        Self::ColumnNewRecords,
        Self::KindStruct,
        Self::KindParent,
        Self::KindProperty,
        Self::KindEnum,
        Self::KindOption,
        Self::NoDifferences,
        Self::Failed,
        Self::Row,
    ];

    /// Returns the Fluent message id.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Tab => "schema-tab",
            Self::LabelStructs => "schema-label-structs",
            Self::LabelProperties => "schema-label-properties",
            Self::LabelEnums => "schema-label-enums",
            Self::LabelOptions => "schema-label-options",
            Self::Counts => "schema-counts",
            Self::ColumnKind => "schema-column-kind",
            Self::ColumnName => "schema-column-name",
            Self::ColumnOld => "schema-column-old",
            Self::ColumnNew => "schema-column-new",
            Self::ColumnOldRecords => "schema-column-old-records",
            Self::ColumnNewRecords => "schema-column-new-records",
            Self::KindStruct => "schema-kind-struct",
            Self::KindParent => "schema-kind-parent",
            Self::KindProperty => "schema-kind-property",
            Self::KindEnum => "schema-kind-enum",
            Self::KindOption => "schema-kind-option",
            Self::NoDifferences => "schema-no-differences",
            Self::Failed => "schema-failed",
            Self::Row => "schema-row",
        }
    }
}
