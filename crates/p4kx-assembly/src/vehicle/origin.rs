/// Where the value of a part's attribute comes from once the Modification is applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ValueOrigin {
    /// The implementation file named by the Entity.
    #[default]
    Base,
    /// The patch file of the Modification, which replaced or merged the part.
    Patch,
    /// An `Elem` of the Modification, which set the attribute.
    Elem,
}

impl ValueOrigin {
    /// Returns the stable code of the origin, e.g. `elem`.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Base => "base",
            Self::Patch => "patch",
            Self::Elem => "elem",
        }
    }
}
