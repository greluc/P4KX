/// A text of the XML preview of a `CryXml` Entry, looked up in the Fluent file of the current
/// language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum XmlText {
    /// The preview tab that shows a `CryXml` Entry as XML.
    Tab,
    /// Shown while the Entry is converted; takes `path`.
    Converting,
    /// Shown for an Entry too large to convert; takes `path`, `size` and `limit`.
    TooLarge,
    /// Shown when the Entry cannot be converted; takes `path` and `reason`.
    Failed,
    /// Shown when converting stopped without a result; takes `path`.
    Stopped,
    /// Shown above XML cut at the limit; takes `lines` and `limit`.
    Truncated,
    /// The accessible name of the scrollable lines.
    View,
}

impl XmlText {
    /// Every text of the XML preview.
    pub(crate) const ALL: [Self; 7] = [
        Self::Tab,
        Self::Converting,
        Self::TooLarge,
        Self::Failed,
        Self::Stopped,
        Self::Truncated,
        Self::View,
    ];

    /// Returns the Fluent message id.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Tab => "preview-tab-xml",
            Self::Converting => "xml-converting",
            Self::TooLarge => "xml-too-large",
            Self::Failed => "xml-failed",
            Self::Stopped => "xml-stopped",
            Self::Truncated => "xml-truncated",
            Self::View => "xml-view",
        }
    }
}
