/// A text of the Record preview, looked up in the Fluent file of the current language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RecordText {
    /// The preview tab that shows the selected Record.
    Tab,
    /// The action that selects the Record before the current one on the way taken.
    Back,
    /// The action that selects the Record after the current one on the way taken.
    Forward,
    /// The key of the Record's type.
    Type,
    /// The key of the Record's GUID.
    Guid,
    /// The key of the Record's name.
    Name,
    /// The value column.
    HeadValue,
    /// Shown while the Record is read; takes `path`.
    Loading,
    /// Shown when the Record cannot be read; takes `path` and `reason`.
    Failed,
    /// Shown when reading stopped without a result; takes `path`.
    Stopped,
    /// Shown instead of the Record once the Archive changed on disk.
    ArchiveChanged,
    /// Shown after following a Reference to a Record that does not exist; takes `guid`.
    Dangling,
    /// The accessible name of a Reference link; takes `path`.
    Follow,
    /// A field row for assistive technology and the tooltip; takes `field` and `value`.
    Row,
    /// Shown instead of the field rows of a Record without fields.
    NoFields,
}

impl RecordText {
    /// Every text of the Record preview.
    pub(crate) const ALL: [Self; 15] = [
        Self::Tab,
        Self::Back,
        Self::Forward,
        Self::Type,
        Self::Guid,
        Self::Name,
        Self::HeadValue,
        Self::Loading,
        Self::Failed,
        Self::Stopped,
        Self::ArchiveChanged,
        Self::Dangling,
        Self::Follow,
        Self::Row,
        Self::NoFields,
    ];

    /// Returns the Fluent message id.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Tab => "preview-tab-record",
            Self::Back => "record-back",
            Self::Forward => "record-forward",
            Self::Type => "record-type",
            Self::Guid => "record-guid",
            Self::Name => "record-name",
            Self::HeadValue => "record-head-value",
            Self::Loading => "record-loading",
            Self::Failed => "record-failed",
            Self::Stopped => "record-stopped",
            Self::ArchiveChanged => "record-archive-changed",
            Self::Dangling => "record-dangling",
            Self::Follow => "record-follow",
            Self::Row => "record-row",
            Self::NoFields => "record-no-fields",
        }
    }
}
