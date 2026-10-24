// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

/// A text of the Tags tab, looked up in the Fluent file of the current language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TagsText {
    /// The tab with the tag tree.
    Tab,
    /// The label of the search field.
    Label,
    /// The placeholder of the empty search field.
    Hint,
    /// Shown while the tags are read.
    Loading,
    /// Shown when the DataForge holds no tags.
    NoTags,
    /// Shown when reading failed; takes `reason`.
    Failed,
    /// Shown when reading stopped without a result.
    Stopped,
    /// Shown when the Archive changed on disk.
    ArchiveChanged,
    /// The title before an Archive is open.
    EmptyTitle,
    /// The hint before an Archive is open.
    EmptyHint,
    /// The head of the tag column.
    ColumnTag,
    /// The head of the GUID column.
    ColumnGuid,
    /// The head of the legacy ID column.
    ColumnLegacy,
    /// The number of tags; takes `count` and `number`.
    Count,
    /// The number of tags shown of all tags; takes `count`, `number` and `total`.
    CountShown,
    /// Shown when no tag path matches the search.
    NoMatch,
    /// The action that copies the tag path of the selected tag.
    CopyPath,
    /// The action that copies the GUID of the selected tag.
    CopyGuid,
    /// The remark that the tree differs from what the `TagDatabase` lists; takes `count` and `number`.
    Irregular,
}

impl TagsText {
    /// Every text of the Tags tab.
    pub(crate) const ALL: [Self; 19] = [
        Self::Tab,
        Self::Label,
        Self::Hint,
        Self::Loading,
        Self::NoTags,
        Self::Failed,
        Self::Stopped,
        Self::ArchiveChanged,
        Self::EmptyTitle,
        Self::EmptyHint,
        Self::ColumnTag,
        Self::ColumnGuid,
        Self::ColumnLegacy,
        Self::Count,
        Self::CountShown,
        Self::NoMatch,
        Self::CopyPath,
        Self::CopyGuid,
        Self::Irregular,
    ];

    /// Returns the Fluent message id.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Tab => "tags-tab",
            Self::Label => "tags-label",
            Self::Hint => "tags-hint",
            Self::Loading => "tags-loading",
            Self::NoTags => "tags-no-tags",
            Self::Failed => "tags-failed",
            Self::Stopped => "tags-stopped",
            Self::ArchiveChanged => "tags-archive-changed",
            Self::EmptyTitle => "tags-empty-title",
            Self::EmptyHint => "tags-empty-hint",
            Self::ColumnTag => "tags-column-tag",
            Self::ColumnGuid => "tags-column-guid",
            Self::ColumnLegacy => "tags-column-legacy",
            Self::Count => "tags-count",
            Self::CountShown => "tags-count-shown",
            Self::NoMatch => "tags-no-match",
            Self::CopyPath => "tags-copy-path",
            Self::CopyGuid => "tags-copy-guid",
            Self::Irregular => "tags-irregular",
        }
    }
}

impl From<TagsText> for crate::Text {
    fn from(text: TagsText) -> Self {
        Self::Tags(text)
    }
}
