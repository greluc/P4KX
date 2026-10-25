// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

/// A text of the Characters tab, looked up in the Fluent file of the current language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CharacterText {
    /// The tab.
    Tab,
    /// The hint of the empty state before an Archive is open.
    EmptyHint,
    /// The heading of the choice of the body.
    BodyHeading,
    /// The male body.
    BodyMale,
    /// The female body.
    BodyFemale,
    /// Shown while a head decides the body.
    BodyFromHead,
    /// The heading of the choice of the head.
    HeadHeading,
    /// Shown while no head is chosen.
    HeadNone,
    /// The head chosen; takes `name` and `rig`.
    HeadLine,
    /// Shown while the CHF file is read.
    HeadReading,
    /// The CHF file cannot be used; takes `reason`.
    HeadFailed,
    /// The action that chooses a CHF file of the Archive.
    HeadFromArchive,
    /// The action that chooses a CHF file on disk.
    HeadFromFile,
    /// The file type filter of the dialog that opens a CHF file.
    ChfFilter,
    /// The action that takes the head away.
    HeadRemove,
    /// The heading of the choice of the pieces.
    ArmourHeading,
    /// Shown while no Loadout is chosen.
    LoadoutNone,
    /// The Loadout chosen; takes `name`.
    LoadoutLine,
    /// The action that chooses a Loadout.
    LoadoutChoose,
    /// The action that takes the Loadout away.
    LoadoutClear,
    /// Shown while no piece is added.
    PiecesNone,
    /// The action that adds a piece.
    PieceAdd,
    /// The accessible name of the action that takes a piece off; takes `name`.
    PieceRemove,
    /// The title of the choice of a piece.
    ChooseTitlePiece,
    /// The title of the choice of a Loadout.
    ChooseTitleLoadout,
    /// The title of the choice of a head.
    ChooseTitleHead,
    /// The hint inside the search field of the choice.
    ChooseHint,
    /// How many candidates the search leaves; takes `number` and `total`.
    ChooseCount,
    /// Shown when the search leaves no candidate.
    ChooseNone,
    /// The heading of the expressions.
    ExpressionsHeading,
    /// The checkbox that loads the expressions of the head.
    ExpressionsLoad,
    /// Shown while there is no head whose expressions could be loaded.
    ExpressionsNoHead,
    /// Shown when the head has no expressions.
    ExpressionsNone,
    /// The choice that shows the neutral face.
    ExpressionNeutral,
    /// The label of the slider of the strength; takes `percent`.
    ExpressionStrength,
    /// The action that opens the export of the Character.
    ExportAction,
    /// The title of the setup window of the export.
    ExportSetupTitle,
    /// The Character that is exported; takes `name`.
    ExportSetupName,
    /// What the Blender package of a Character is.
    ExportPackageNote,
    /// What the Blender package of a Character holds; takes `parts`, `bones`, `skinned`,
    /// `morphs`, `animations`, `vertices`, `triangles`, `files` and `size`.
    ExportPackageCounts,
    /// How many pieces the export left out; takes `count` and `number`.
    ExportSkipped,
    /// What the export wrote; takes `parts`, `bones`, `vertices`, `triangles` and `size`.
    ExportCounts,
    /// The title of the window of the running export.
    ExportRunningTitle,
    /// The title of the window of the finished export.
    ExportDoneTitle,
    /// The accessible name of the 3D view; takes `name`, `parts` and `triangles`.
    ViewLabel,
    /// Shown while the models are turned into the scene.
    ViewPreparing,
    /// The heading of the report; takes `number`.
    ReportHeading,
    /// Shown when every piece fits.
    ReportAll,
    /// A piece that is left out; takes `name` and `reason`.
    ReportSkipped,
    /// A note on a piece that is worn; takes `name` and `reason`.
    ReportNote,
    /// A piece of the Loadout that another piece took the place of; takes `name` and `by`.
    ReportReplaced,
}

impl CharacterText {
    /// Every text, for checks of the Fluent files.
    pub(crate) const ALL: [Self; 51] = [
        Self::Tab,
        Self::EmptyHint,
        Self::BodyHeading,
        Self::BodyMale,
        Self::BodyFemale,
        Self::BodyFromHead,
        Self::HeadHeading,
        Self::HeadNone,
        Self::HeadLine,
        Self::HeadReading,
        Self::HeadFailed,
        Self::HeadFromArchive,
        Self::HeadFromFile,
        Self::ChfFilter,
        Self::HeadRemove,
        Self::ArmourHeading,
        Self::LoadoutNone,
        Self::LoadoutLine,
        Self::LoadoutChoose,
        Self::LoadoutClear,
        Self::PiecesNone,
        Self::PieceAdd,
        Self::PieceRemove,
        Self::ChooseTitlePiece,
        Self::ChooseTitleLoadout,
        Self::ChooseTitleHead,
        Self::ChooseHint,
        Self::ChooseCount,
        Self::ChooseNone,
        Self::ExpressionsHeading,
        Self::ExpressionsLoad,
        Self::ExpressionsNoHead,
        Self::ExpressionsNone,
        Self::ExpressionNeutral,
        Self::ExpressionStrength,
        Self::ExportAction,
        Self::ExportSetupTitle,
        Self::ExportSetupName,
        Self::ExportPackageNote,
        Self::ExportPackageCounts,
        Self::ExportSkipped,
        Self::ExportCounts,
        Self::ExportRunningTitle,
        Self::ExportDoneTitle,
        Self::ViewLabel,
        Self::ViewPreparing,
        Self::ReportHeading,
        Self::ReportAll,
        Self::ReportSkipped,
        Self::ReportNote,
        Self::ReportReplaced,
    ];

    /// Returns the Fluent message id.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Tab => "characters-tab",
            Self::EmptyHint => "characters-empty-hint",
            Self::BodyHeading => "characters-body-heading",
            Self::BodyMale => "characters-body-male",
            Self::BodyFemale => "characters-body-female",
            Self::BodyFromHead => "characters-body-from-head",
            Self::HeadHeading => "characters-head-heading",
            Self::HeadNone => "characters-head-none",
            Self::HeadLine => "characters-head-line",
            Self::HeadReading => "characters-head-reading",
            Self::HeadFailed => "characters-head-failed",
            Self::HeadFromArchive => "characters-head-from-archive",
            Self::HeadFromFile => "characters-head-from-file",
            Self::ChfFilter => "characters-chf-filter",
            Self::HeadRemove => "characters-head-remove",
            Self::ArmourHeading => "characters-armour-heading",
            Self::LoadoutNone => "characters-loadout-none",
            Self::LoadoutLine => "characters-loadout-line",
            Self::LoadoutChoose => "characters-loadout-choose",
            Self::LoadoutClear => "characters-loadout-clear",
            Self::PiecesNone => "characters-pieces-none",
            Self::PieceAdd => "characters-piece-add",
            Self::PieceRemove => "characters-piece-remove",
            Self::ChooseTitlePiece => "characters-choose-title-piece",
            Self::ChooseTitleLoadout => "characters-choose-title-loadout",
            Self::ChooseTitleHead => "characters-choose-title-head",
            Self::ChooseHint => "characters-choose-hint",
            Self::ChooseCount => "characters-choose-count",
            Self::ChooseNone => "characters-choose-none",
            Self::ExpressionsHeading => "characters-expressions-heading",
            Self::ExpressionsLoad => "characters-expressions-load",
            Self::ExpressionsNoHead => "characters-expressions-no-head",
            Self::ExpressionsNone => "characters-expressions-none",
            Self::ExpressionNeutral => "characters-expression-neutral",
            Self::ExpressionStrength => "characters-expression-strength",
            Self::ExportAction => "characters-export-action",
            Self::ExportSetupTitle => "characters-export-setup-title",
            Self::ExportSetupName => "characters-export-setup-name",
            Self::ExportPackageNote => "characters-export-package-note",
            Self::ExportPackageCounts => "characters-export-package-counts",
            Self::ExportSkipped => "characters-export-skipped",
            Self::ExportCounts => "characters-export-counts",
            Self::ExportRunningTitle => "characters-export-running-title",
            Self::ExportDoneTitle => "characters-export-done-title",
            Self::ViewLabel => "characters-view-label",
            Self::ViewPreparing => "characters-view-preparing",
            Self::ReportHeading => "characters-report-heading",
            Self::ReportAll => "characters-report-all",
            Self::ReportSkipped => "characters-report-skipped",
            Self::ReportNote => "characters-report-note",
            Self::ReportReplaced => "characters-report-replaced",
        }
    }
}

impl From<CharacterText> for super::Text {
    fn from(text: CharacterText) -> Self {
        Self::Character(text)
    }
}
