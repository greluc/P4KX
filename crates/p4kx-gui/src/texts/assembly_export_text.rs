// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

/// A text of the Export of an Entity as GLB windows, looked up in the Fluent file of the current
/// language. The texts of a failed export are those of [`ModelExportText`](crate::ModelExportText).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AssemblyExportText {
    /// The title of the setup window.
    SetupTitle,
    /// The Entity that is exported; takes `name`.
    Entity,
    /// What the changes to the Loadout amount to; takes `excluded` and `replaced`.
    Changes,
    /// Shown when the Loadout is the default one.
    ChangesNone,
    /// The heading of the level of detail choice.
    LodHeading,
    /// One level of detail; takes `level`.
    Lod,
    /// The heading of the material choice.
    MaterialsHeading,
    /// Materials as plain colours.
    MaterialsColours,
    /// Materials with their textures.
    MaterialsTextures,
    /// The heading of the texture size choice.
    TextureMipHeading,
    /// One texture size; takes `level` and `divisor`.
    TextureMip,
    /// The switch of the vertex colours.
    VertexColours,
    /// The heading of the file choice.
    FileHeading,
    /// The action that opens the file dialog.
    ChooseFile,
    /// The hint of the file field while no file is chosen.
    FileNone,
    /// The name of the GLB file type in the file dialog.
    FileFilter,
    /// The action that starts the export.
    Start,
    /// The title of the progress window.
    RunningTitle,
    /// Shown while the Loadout is resolved; takes `name`.
    Resolving,
    /// Shown while the models are read; takes `name`.
    Reading,
    /// The progress while the file is written; takes `size`.
    Writing,
    /// The title of the summary of a finished export.
    SummaryTitle,
    /// The title of the summary of a cancelled export.
    SummaryCancelledTitle,
    /// The note that the export was cancelled and wrote no file.
    SummaryCancelled,
    /// The file that was written; takes `path`.
    SummaryFile,
    /// What the file holds; takes `parts`, `modelled`, `nodes`, `meshes`, `materials`,
    /// `vertices`, `triangles` and `size`.
    SummaryCounts,
    /// The textures in the file; takes `images`, `textured` and `materials`.
    SummaryImages,
    /// The models read at a lower level of detail; takes `count` and `number`.
    SummaryFallbacks,
    /// Submeshes left out because their material draws nothing; takes `count` and `number`.
    SummaryHidden,
    /// The heading of the report of the parts that were not resolved in full; takes `number`.
    ReportHeading,
    /// Shown when every part was resolved.
    ReportAll,
    /// A part left out of the file; takes `path`, `name` and `reason`.
    ReportSkipped,
    /// A part kept with a note; takes `path`, `name` and `reason`.
    ReportNote,
}

impl AssemblyExportText {
    /// Every text, for checks of the Fluent files.
    pub(crate) const ALL: [Self; 33] = [
        Self::SetupTitle,
        Self::Entity,
        Self::Changes,
        Self::ChangesNone,
        Self::LodHeading,
        Self::Lod,
        Self::MaterialsHeading,
        Self::MaterialsColours,
        Self::MaterialsTextures,
        Self::TextureMipHeading,
        Self::TextureMip,
        Self::VertexColours,
        Self::FileHeading,
        Self::ChooseFile,
        Self::FileNone,
        Self::FileFilter,
        Self::Start,
        Self::RunningTitle,
        Self::Resolving,
        Self::Reading,
        Self::Writing,
        Self::SummaryTitle,
        Self::SummaryCancelledTitle,
        Self::SummaryCancelled,
        Self::SummaryFile,
        Self::SummaryCounts,
        Self::SummaryImages,
        Self::SummaryFallbacks,
        Self::SummaryHidden,
        Self::ReportHeading,
        Self::ReportAll,
        Self::ReportSkipped,
        Self::ReportNote,
    ];

    /// Returns the Fluent message id.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::SetupTitle => "assembly-export-title",
            Self::Entity => "assembly-export-entity",
            Self::Changes => "assembly-export-changes",
            Self::ChangesNone => "assembly-export-changes-none",
            Self::LodHeading => "assembly-export-lod-heading",
            Self::Lod => "assembly-export-lod",
            Self::MaterialsHeading => "assembly-export-materials-heading",
            Self::MaterialsColours => "assembly-export-materials-colours",
            Self::MaterialsTextures => "assembly-export-materials-textures",
            Self::TextureMipHeading => "assembly-export-texture-mip-heading",
            Self::TextureMip => "assembly-export-texture-mip",
            Self::VertexColours => "assembly-export-vertex-colours",
            Self::FileHeading => "assembly-export-file-heading",
            Self::ChooseFile => "assembly-export-choose-file",
            Self::FileNone => "assembly-export-file-none",
            Self::FileFilter => "assembly-export-file-filter",
            Self::Start => "assembly-export-start",
            Self::RunningTitle => "assembly-export-running-title",
            Self::Resolving => "assembly-export-resolving",
            Self::Reading => "assembly-export-reading",
            Self::Writing => "assembly-export-writing",
            Self::SummaryTitle => "assembly-export-summary-title",
            Self::SummaryCancelledTitle => "assembly-export-summary-cancelled-title",
            Self::SummaryCancelled => "assembly-export-summary-cancelled",
            Self::SummaryFile => "assembly-export-summary-file",
            Self::SummaryCounts => "assembly-export-summary-counts",
            Self::SummaryImages => "assembly-export-summary-images",
            Self::SummaryFallbacks => "assembly-export-summary-fallbacks",
            Self::SummaryHidden => "assembly-export-summary-hidden",
            Self::ReportHeading => "assembly-export-report-heading",
            Self::ReportAll => "assembly-export-report-all",
            Self::ReportSkipped => "assembly-export-report-skipped",
            Self::ReportNote => "assembly-export-report-note",
        }
    }
}

impl From<AssemblyExportText> for super::Text {
    fn from(text: AssemblyExportText) -> Self {
        Self::AssemblyExport(text)
    }
}
