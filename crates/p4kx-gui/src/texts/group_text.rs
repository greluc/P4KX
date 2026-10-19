/// A text of the split textures in the Entry table, looked up in the Fluent file of the current
/// language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GroupText {
    /// The number of parts folded into a texture row; takes `count` and `number`.
    Parts,
    /// Expands a texture to its parts; takes `name`.
    Expand,
    /// Collapses a texture to its row; takes `name`.
    Collapse,
}

impl GroupText {
    /// Every text of the split textures.
    pub(crate) const ALL: [Self; 3] = [Self::Parts, Self::Expand, Self::Collapse];

    /// Returns the Fluent message id.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Parts => "texture-parts",
            Self::Expand => "texture-expand",
            Self::Collapse => "texture-collapse",
        }
    }
}
